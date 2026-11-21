//! Conversor de taxa em tempo real (Pure Fabrication, S-1/AU-16): quando o fone ou o microfone não
//! aceita a taxa do projeto, o dispositivo abre na taxa mais próxima e o áudio é convertido aqui.
//!
//! - [`Resampler`]: núcleo em streaming para f32 intercalado, multicanal, razão arbitrária entre duas
//!   taxas inteiras. Sinc com janela de Kaiser ([`TAPS`] coeficientes, alongado na mesma proporção
//!   quando a taxa cai, para a faixa de transição ficar igual em relação à taxa de saída; [`PHASES`]
//!   fases tabeladas com interpolação linear entre fases). A posição é **racional exata** (inteiro + numerador sobre a taxa
//!   de destino), então a contagem de quadros nunca deriva, por mais longa que seja a sessão.
//! - [`OutputRateAdapter`] (modelo *pull*): o callback de saída pede N quadros na taxa do dispositivo;
//!   o adaptador pede à engine exatamente os quadros da taxa do projeto que faltam e entrega N quadros.
//! - [`InputRateAdapter`] (modelo *push*): o microfone entrega quadros na taxa dele; o adaptador
//!   entrega quadros na taxa do projeto à fila de entrada.
//!
//! Regras de tempo real: tudo é alocado em `new`; `push`, `pull`, `fill` e `process` não alocam, não
//! bloqueiam e não entram em pânico com dados válidos. Amostras não finitas (NaN/∞) viram 0 na entrada.

/// Coeficientes do filtro sem redução de taxa (24 de cada lado do ponto de interpolação). Reduzindo a
/// taxa, o filtro tem `TAPS × from/to` coeficientes (arredondado para par).
pub const TAPS: usize = 48;
/// Fases tabeladas por amostra de entrada (interpolação linear entre fases vizinhas).
pub const PHASES: usize = 256;
/// Janela de Kaiser β = 7 (rejeição ≈ 70 dB).
const KAISER_BETA: f64 = 7.0;
/// Corte em fração do Nyquist da menor das duas taxas.
const PASSBAND: f64 = 0.9;

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Função de Bessel modificada de ordem 0 (série; só na construção).
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let q = x * x / 4.0;
    for k in 1..64 {
        term *= q / (k * k) as f64;
        sum += term;
        if term < sum * 1e-16 {
            break;
        }
    }
    sum
}

/// Resposta do filtro em `x` quadros de entrada do ponto de interpolação.
fn kernel(x: f64, cutoff: f64, half: usize) -> f64 {
    let half = half as f64;
    if x.abs() >= half {
        return 0.0;
    }
    let r = x / half;
    let w = bessel_i0(KAISER_BETA * (1.0 - r * r).sqrt()) / bessel_i0(KAISER_BETA);
    let a = std::f64::consts::PI * cutoff * x;
    let sinc = if a.abs() < 1e-12 { 1.0 } else { a.sin() / a };
    cutoff * sinc * w
}

/// Conversor de taxa em streaming (f32 intercalado).
pub struct Resampler {
    channels: usize,
    /// Taxas reduzidas pelo MDC: cada quadro de saída avança `from / to` quadros de entrada.
    from: u64,
    to: u64,
    identity: bool,
    /// Quadros necessários antes/depois do ponto de interpolação.
    lag: usize,
    lead: usize,
    /// Coeficientes por ponto (`lag + lead`, 0 na identidade).
    taps: usize,
    /// `(PHASES + 1) × taps` coeficientes; cada linha soma 1 (DC preservado).
    table: Vec<f32>,
    /// Quadros de entrada ainda úteis (intercalados) e capacidade em quadros.
    buf: Vec<f32>,
    cap: usize,
    len: usize,
    /// Posição do próximo quadro de saída em `buf`: `ipos + frac / to`.
    ipos: usize,
    frac: u64,
    /// Rascunho dos coeficientes interpolados do ponto atual.
    coefs: Vec<f32>,
}

impl Resampler {
    /// `max_block_frames`: maior bloco de entrada empurrado ou de saída pedido de uma vez.
    pub fn new(channels: usize, from_rate: u32, to_rate: u32, max_block_frames: usize) -> Self {
        let channels = channels.max(1);
        let (from, to) = (from_rate.max(1) as u64, to_rate.max(1) as u64);
        let g = gcd(from, to);
        let (from, to) = (from / g, to / g);
        let identity = from == to;
        let step = (from as f64 / to as f64).max(1.0);
        // reduzindo a taxa, o corte cai na mesma proporção: o filtro cresce junto (mesma transição)
        let half = if identity {
            0
        } else {
            (TAPS as f64 / 2.0 * step).ceil() as usize
        };
        let taps = 2 * half;
        let (lag, lead) = (half.saturating_sub(1), half);
        let mut table = Vec::new();
        if !identity {
            let cutoff = PASSBAND / step;
            table = vec![0.0f32; (PHASES + 1) * taps];
            for p in 0..=PHASES {
                let phase = p as f64 / PHASES as f64;
                let row: Vec<f64> = (0..taps)
                    .map(|k| kernel(phase + lag as f64 - k as f64, cutoff, half))
                    .collect();
                let sum: f64 = row.iter().sum();
                for (k, v) in row.iter().enumerate() {
                    table[p * taps + k] = (v / sum) as f32;
                }
            }
        }
        let cap = (max_block_frames.max(1) as f64 * step).ceil() as usize + 2 * taps + 4;
        Self {
            channels,
            from,
            to,
            identity,
            lag,
            lead,
            taps,
            table,
            buf: vec![0.0; cap * channels],
            cap,
            len: lag,
            ipos: lag,
            frac: 0,
            coefs: vec![0.0; taps],
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Atraso introduzido, em quadros de **entrada** (quadros que precisam chegar antes do ponto
    /// convertido): 0 na identidade; senão ≈ [`TAPS`]/2 quadros da **menor** das duas taxas
    /// (44,1 ↔ 48 kHz: ~0,55 ms; com 16 kHz: 1,5 ms; 48 kHz → 44,1 kHz: 27 quadros de 48 kHz).
    pub fn latency_input_frames(&self) -> usize {
        self.lead
    }

    /// Coeficientes do filtro (0 na identidade).
    pub fn taps(&self) -> usize {
        self.taps
    }

    /// Quadros de entrada que ainda faltam para produzir `out_frames` quadros de saída.
    pub fn input_needed(&self, out_frames: usize) -> usize {
        if out_frames == 0 {
            return 0;
        }
        let last =
            self.ipos + ((self.frac + (out_frames as u64 - 1) * self.from) / self.to) as usize;
        (last + self.lead + 1).saturating_sub(self.len)
    }

    /// Empurra quadros intercalados; devolve quantos quadros couberam. NaN/∞ viram 0.
    pub fn push(&mut self, input: &[f32]) -> usize {
        let ch = self.channels;
        let n = (input.len() / ch).min(self.cap - self.len);
        let dst = &mut self.buf[self.len * ch..(self.len + n) * ch];
        for (d, s) in dst.iter_mut().zip(input) {
            *d = if s.is_finite() { *s } else { 0.0 };
        }
        self.len += n;
        n
    }

    /// Produz até `out.len() / canais` quadros com a entrada disponível; devolve quantos produziu.
    pub fn pull(&mut self, out: &mut [f32]) -> usize {
        let ch = self.channels;
        let want = out.len() / ch;
        let mut produced = 0;
        while produced < want && self.ipos + self.lead < self.len {
            let dst = &mut out[produced * ch..(produced + 1) * ch];
            if self.identity {
                dst.copy_from_slice(&self.buf[self.ipos * ch..(self.ipos + 1) * ch]);
            } else {
                let scaled = self.frac * PHASES as u64;
                let p = (scaled / self.to) as usize;
                let a = (scaled % self.to) as f32 / self.to as f32;
                let t = self.taps;
                let r0 = &self.table[p * t..(p + 1) * t];
                let r1 = &self.table[(p + 1) * t..(p + 2) * t];
                for ((c, x0), x1) in self.coefs.iter_mut().zip(r0).zip(r1) {
                    *c = x0 + (x1 - x0) * a;
                }
                let first = self.ipos - self.lag;
                for (c, d) in dst.iter_mut().enumerate() {
                    let mut acc = 0.0f32;
                    for (k, coef) in self.coefs.iter().enumerate() {
                        acc += coef * self.buf[(first + k) * ch + c];
                    }
                    *d = acc;
                }
            }
            produced += 1;
            self.frac += self.from;
            self.ipos += (self.frac / self.to) as usize;
            self.frac %= self.to;
        }
        // descarta o que não será mais usado (sem alocar: só desloca o restante)
        let drop = (self.ipos - self.lag).min(self.len);
        if drop > 0 {
            self.buf.copy_within(drop * ch..self.len * ch, 0);
            self.len -= drop;
            self.ipos -= drop;
        }
        produced
    }
}

/// Saída: engine na taxa do projeto → dispositivo na taxa dele (modelo *pull*).
pub struct OutputRateAdapter {
    rs: Resampler,
    scratch: Vec<f32>,
    max_frames: usize,
}

impl OutputRateAdapter {
    /// `max_device_frames`: maior pedaço tratado de uma vez (callbacks maiores são divididos); também
    /// é o maior bloco pedido à engine por vez.
    pub fn new(
        channels: usize,
        engine_rate: u32,
        device_rate: u32,
        max_device_frames: usize,
    ) -> Self {
        let rs = Resampler::new(channels, engine_rate, device_rate, max_device_frames);
        let max_frames = max_device_frames.max(1);
        Self {
            scratch: vec![0.0; max_frames * rs.channels()],
            rs,
            max_frames,
        }
    }

    pub fn latency_engine_frames(&self) -> usize {
        self.rs.latency_input_frames()
    }

    /// Preenche `out` (taxa do dispositivo) por inteiro, chamando `render` (taxa do projeto) com
    /// blocos de no máximo `max_device_frames` quadros, só quantos forem necessários.
    pub fn fill<F: FnMut(&mut [f32])>(&mut self, out: &mut [f32], mut render: F) {
        let ch = self.rs.channels();
        let whole = out.len() / ch * ch;
        let (body, tail) = out.split_at_mut(whole);
        tail.fill(0.0);
        for piece in body.chunks_mut(self.max_frames * ch) {
            let mut need = self.rs.input_needed(piece.len() / ch);
            while need > 0 {
                let n = need.min(self.max_frames);
                let s = &mut self.scratch[..n * ch];
                render(s);
                let pushed = self.rs.push(s);
                if pushed < n {
                    break; // não acontece com a capacidade calculada em `new`; protege o laço
                }
                need -= n;
            }
            let got = self.rs.pull(piece);
            piece[got * ch..].fill(0.0);
        }
    }
}

/// Entrada: microfone na taxa dele → fila na taxa do projeto (modelo *push*).
pub struct InputRateAdapter {
    rs: Resampler,
    out: Vec<f32>,
    max_frames: usize,
}

impl InputRateAdapter {
    pub fn new(
        channels: usize,
        device_rate: u32,
        engine_rate: u32,
        max_device_frames: usize,
    ) -> Self {
        let rs = Resampler::new(channels, device_rate, engine_rate, max_device_frames);
        let max_frames = max_device_frames.max(1);
        let ratio = engine_rate.max(1) as f64 / device_rate.max(1) as f64;
        let out_frames = ((max_frames + 2 * rs.taps()) as f64 * ratio).ceil() as usize + 2;
        Self {
            out: vec![0.0; out_frames * rs.channels()],
            rs,
            max_frames,
        }
    }

    pub fn latency_device_frames(&self) -> usize {
        self.rs.latency_input_frames()
    }

    /// Converte `input` (taxa do dispositivo) e entrega o resultado (taxa do projeto) a `push`, em
    /// um ou mais pedaços de quadros inteiros.
    pub fn process<F: FnMut(&[f32])>(&mut self, input: &[f32], mut push: F) {
        let ch = self.rs.channels();
        let whole = input.len() / ch * ch;
        for piece in input[..whole].chunks(self.max_frames * ch) {
            self.rs.push(piece);
            loop {
                let n = self.rs.pull(&mut self.out);
                if n == 0 {
                    break;
                }
                push(&self.out[..n * ch]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    const AMP: f64 = 0.5;

    /// Quadros de entrada à frente do ponto de interpolação para `from → to`.
    fn half(from: u32, to: u32) -> usize {
        Resampler::new(1, from, to, 1).latency_input_frames()
    }

    fn sine_frames(freq: f64, rate: u32, start: usize, frames: usize, ch: usize) -> Vec<f32> {
        let mut v = Vec::with_capacity(frames * ch);
        for i in start..start + frames {
            let s = (AMP * (TAU * freq * i as f64 / rate as f64).sin()) as f32;
            v.extend(std::iter::repeat_n(s, ch));
        }
        v
    }

    /// Tamanhos de bloco irregulares (inclui 1 quadro e o máximo).
    const BLOCKS: [usize; 9] = [1, 7, 480, 4096, 333, 64, 1023, 2, 129];

    /// Converte um seno pelo adaptador de entrada em blocos irregulares.
    fn through_input(from: u32, to: u32, freq: f64, frames: usize) -> Vec<f32> {
        let ch = 2;
        let src = sine_frames(freq, from, 0, frames, ch);
        let mut a = InputRateAdapter::new(ch, from, to, 4096);
        let mut out = Vec::new();
        let (mut at, mut b) = (0, 0);
        while at < frames {
            let n = BLOCKS[b % BLOCKS.len()].min(frames - at);
            a.process(&src[at * ch..(at + n) * ch], |x| out.extend_from_slice(x));
            at += n;
            b += 1;
        }
        out
    }

    /// Converte pelo adaptador de saída (engine falsa gerando um seno) em blocos irregulares.
    fn through_output(
        from: u32,
        to: u32,
        freq: f64,
        out_frames: usize,
    ) -> (Vec<f32>, usize, Vec<usize>) {
        let ch = 2;
        let mut a = OutputRateAdapter::new(ch, from, to, 4096);
        let mut rendered = 0usize;
        let mut out = Vec::new();
        let mut renders = Vec::new();
        let (mut at, mut b) = (0, 0);
        while at < out_frames {
            let n = BLOCKS[b % BLOCKS.len()].min(out_frames - at);
            let mut dev = vec![f32::NAN; n * ch];
            a.fill(&mut dev, |eng: &mut [f32]| {
                let f = eng.len() / ch;
                eng.copy_from_slice(&sine_frames(freq, from, rendered, f, ch));
                rendered += f;
                renders.push(f);
            });
            out.extend_from_slice(&dev);
            at += n;
            b += 1;
        }
        (out, rendered, renders)
    }

    /// Maior erro em relação ao seno ideal na taxa de saída (canal 0), fora das bordas.
    fn max_error(out: &[f32], freq: f64, rate: u32, skip: usize) -> f64 {
        let frames = out.len() / 2;
        (skip..frames - skip)
            .map(|j| {
                let ideal = AMP * (TAU * freq * j as f64 / rate as f64).sin();
                (out[j * 2] as f64 - ideal).abs()
            })
            .fold(0.0, f64::max)
    }

    fn rms(out: &[f32], skip: usize) -> f64 {
        let frames = out.len() / 2;
        let s: f64 = (skip..frames - skip)
            .map(|j| (out[j * 2] as f64).powi(2))
            .sum();
        (s / (frames - 2 * skip) as f64).sqrt()
    }

    fn zero_crossings(out: &[f32], skip: usize) -> usize {
        let frames = out.len() / 2;
        (skip + 1..frames - skip)
            .filter(|j| (out[(j - 1) * 2] < 0.0) != (out[j * 2] < 0.0))
            .count()
    }

    fn check_sine(out: &[f32], rate: u32) {
        let skip = 2 * TAPS;
        // forma de onda: alinhada no tempo com o seno ideal (frequência, fase e amplitude)
        let err = max_error(out, 1000.0, rate, skip);
        assert!(err < 5e-4, "erro máximo {err} (−60 dB da amplitude)");
        // amplitude dentro de 0,5 dB
        let amp = rms(out, skip) * 2f64.sqrt();
        let db = 20.0 * (amp / AMP).log10();
        assert!(db.abs() < 0.5, "amplitude {db:.3} dB");
        // frequência por cruzamentos de zero: 2 por ciclo
        let frames = out.len() / 2 - 2 * skip;
        let secs = frames as f64 / rate as f64;
        let freq = zero_crossings(out, skip) as f64 / 2.0 / secs;
        assert!((freq - 1000.0).abs() < 2.0, "frequência {freq:.1} Hz");
        // continuidade: nenhum salto maior que a maior derivada do seno (+ folga)
        let max_step = TAU * 1000.0 * AMP / rate as f64 * 1.05;
        for j in skip..out.len() / 2 - skip {
            let d = (out[j * 2] - out[(j - 1) * 2]).abs() as f64;
            assert!(d <= max_step, "descontinuidade {d} no quadro {j}");
        }
        // estéreo intercalado: canais idênticos (nada desalinhou L/R)
        assert!(out.chunks(2).all(|f| f[0] == f[1]));
    }

    #[test]
    fn sine_48k_to_44k1_keeps_frequency_and_amplitude() {
        let out = through_input(48_000, 44_100, 1000.0, 48_000);
        check_sine(&out, 44_100);
        let (out, _, _) = through_output(48_000, 44_100, 1000.0, 44_100);
        check_sine(&out, 44_100);
    }

    #[test]
    fn sine_44k1_to_48k_keeps_frequency_and_amplitude() {
        let out = through_input(44_100, 48_000, 1000.0, 44_100);
        check_sine(&out, 48_000);
        let (out, _, _) = through_output(44_100, 48_000, 1000.0, 48_000);
        check_sine(&out, 48_000);
    }

    #[test]
    fn other_common_ratios_work() {
        // fone Bluetooth em modo chamada (16 kHz), placas a 96 kHz
        for (from, to) in [
            (48_000, 16_000),
            (16_000, 48_000),
            (48_000, 96_000),
            (96_000, 44_100),
        ] {
            let out = through_input(from, to, 1000.0, from as usize);
            check_sine(&out, to);
        }
    }

    #[test]
    fn output_fills_exactly_and_renders_exactly_what_is_needed() {
        for (from, to) in [(48_000u32, 44_100u32), (44_100, 48_000), (48_000, 16_000)] {
            let total = 300_000;
            let (out, rendered, renders) = through_output(from, to, 440.0, total);
            assert_eq!(
                out.len(),
                total * 2,
                "cada callback recebe todos os quadros"
            );
            assert!(
                out.iter().all(|x| x.is_finite()),
                "nenhum quadro ficou sem escrever"
            );
            // quadro de saída j fica na posição j·from/to da entrada; o último precisa de `half` à frente
            let expected =
                ((total as u64 - 1) * from as u64 / to as u64) as usize + half(from, to) + 1;
            assert_eq!(
                rendered, expected,
                "{from}->{to}: contagem exata, sem deriva"
            );
            assert!(
                renders.iter().all(|n| *n <= 4096),
                "blocos da engine ≤ máximo"
            );
        }
    }

    #[test]
    fn input_frame_count_is_exact_over_uneven_blocks() {
        for (from, to) in [(48_000u32, 44_100u32), (44_100, 48_000)] {
            let frames = 250_000;
            let out = through_input(from, to, 440.0, frames);
            // saída j existe quando ⌊j·from/to⌋ + half < frames, isto é, j·from < (frames − half)·to
            let limit = (frames - half(from, to)) as u64 * to as u64;
            let expected = limit.div_ceil(from as u64) as usize;
            assert_eq!(out.len() / 2, expected, "{from}->{to}");
        }
    }

    #[test]
    fn identity_passes_through_unchanged_with_no_latency() {
        let src = sine_frames(440.0, 48_000, 0, 10_000, 2);
        let mut a = InputRateAdapter::new(2, 48_000, 48_000, 4096);
        assert_eq!(a.latency_device_frames(), 0);
        let mut out = Vec::new();
        for c in src.chunks(2 * 333) {
            a.process(c, |x| out.extend_from_slice(x));
        }
        assert_eq!(out, src);
        let mut o = OutputRateAdapter::new(2, 48_000, 48_000, 4096);
        let mut dev = vec![0.0; 2 * 512];
        let mut k = 0;
        o.fill(&mut dev, |e: &mut [f32]| {
            e.copy_from_slice(&src[k..k + e.len()]);
            k += e.len();
        });
        assert_eq!(&dev[..], &src[..2 * 512]);
    }

    #[test]
    fn latency_is_known_and_small() {
        // subindo a taxa: TAPS/2 quadros de entrada (24 a 44,1 kHz = 0,54 ms)
        let i = InputRateAdapter::new(2, 44_100, 48_000, 4096);
        assert_eq!(i.latency_device_frames(), TAPS / 2);
        // reduzindo: o filtro cresce com a razão, o atraso fica ≈ TAPS/2 quadros da taxa de saída
        let o = OutputRateAdapter::new(2, 48_000, 44_100, 4096);
        assert_eq!(
            o.latency_engine_frames(),
            27,
            "27 quadros a 48 kHz = 0,56 ms"
        );
        for (from, to) in [(48_000u32, 16_000u32), (192_000, 48_000), (96_000, 44_100)] {
            let out_frames = half(from, to) as f64 * to as f64 / from as f64;
            assert!(
                (TAPS as f64 / 2.0..TAPS as f64 / 2.0 + 1.0).contains(&out_frames),
                "{from}->{to}: {out_frames:.2} quadros de saída"
            );
        }
        // primeiro callback: pede à engine só o necessário (bloco + `half` de antecedência)
        let (_, rendered, _) = through_output(48_000, 44_100, 440.0, 441);
        assert_eq!(rendered, (440 * 48_000 / 44_100) + 27 + 1);
    }

    #[test]
    fn nan_input_does_not_propagate() {
        let mut a = InputRateAdapter::new(1, 44_100, 48_000, 4096);
        let mut out = Vec::new();
        a.process(&[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.3], |x| {
            out.extend_from_slice(x)
        });
        a.process(&vec![0.3; 10_000], |x| out.extend_from_slice(x));
        assert!(out.iter().all(|x| x.is_finite()), "NaN não passa adiante");
        // depois do filtro, volta ao valor constante
        let tail = &out[out.len() - 1000..];
        assert!(tail.iter().all(|x| (x - 0.3).abs() < 1e-4));
    }

    #[test]
    fn dc_is_preserved() {
        for (from, to) in [(48_000u32, 44_100u32), (44_100, 48_000), (48_000, 16_000)] {
            let mut a = InputRateAdapter::new(1, from, to, 4096);
            let mut out = Vec::new();
            for _ in 0..20 {
                a.process(&[0.25; 999], |x| out.extend_from_slice(x));
            }
            for x in &out[TAPS..] {
                assert!((x - 0.25).abs() < 2e-6, "{from}->{to}: {x}");
            }
        }
    }

    #[test]
    fn large_downsampling_keeps_the_passband() {
        // 192 kHz → 48 kHz e 96 kHz → 44,1 kHz: 0,64 do novo Nyquist ainda passa inteiro (o filtro
        // cresce com a razão; com tamanho fixo a transição começaria bem antes)
        for (from, to) in [(192_000u32, 48_000u32), (96_000, 44_100)] {
            let f = 0.32 * to as f64;
            let out = through_input(from, to, f, from as usize / 2);
            let err = max_error(&out, f, to, 2 * TAPS);
            assert!(err < 2e-3, "{from}->{to} a {f} Hz: {err}");
        }
    }

    #[test]
    fn content_above_the_new_nyquist_is_rejected() {
        // 23 kHz a 48 kHz não cabe em 44,1 kHz (Nyquist 22,05 kHz): sem filtro viraria 21,1 kHz audível
        let out = through_input(48_000, 44_100, 23_000.0, 48_000);
        let level = rms(&out, 2 * TAPS) * 2f64.sqrt() / AMP;
        let db = 20.0 * level.log10();
        assert!(db < -50.0, "alias a {db:.1} dB");
    }

    #[test]
    fn degenerate_arguments_do_not_panic() {
        let mut r = Resampler::new(0, 0, 0, 0);
        assert_eq!(r.channels(), 1);
        assert_eq!(r.push(&[1.0; 3]), 3);
        let mut out = [0.0; 3];
        assert_eq!(r.pull(&mut out), 3);
        let mut o = OutputRateAdapter::new(2, 48_000, 44_100, 64);
        let mut odd = [f32::NAN; 5]; // quadro incompleto no fim: zerado
        o.fill(&mut odd, |e: &mut [f32]| e.fill(0.1));
        assert_eq!(odd[4], 0.0);
        assert!(odd.iter().all(|x| x.is_finite()));
    }
}
