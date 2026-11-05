//! Medidores: pico amostral (com *hold* e decaimento), RMS em janela, *true peak* (4×),
//! loudness ITU-R BS.1770-4 / EBU R128, correlação estéreo e contador de clipping.
//!
//! Todos implementam [`Processor`] **sem alterar** o sinal (análise pura), para poderem ser
//! encadeados com efeitos; o método inerente `analyze(&[f32], &[f32])` aceita fatias imutáveis.
//! Leituras são métodos baratos, chamados pela thread de controle (~30 Hz) após publicação.

use crate::biquad::{BiquadCoeffs, BiquadState};
use crate::util::{lin_to_db, safe_sr, DB_FLOOR};
use crate::Processor;

macro_rules! analyzer_processor {
    ($t:ty) => {
        impl Processor for $t {
            fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
                self.prepare_rate(sample_rate);
            }
            fn reset(&mut self) {
                self.clear();
            }
            fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
                self.analyze(left, right);
            }
        }
    };
}

#[inline]
fn finite_abs(x: f32) -> f32 {
    if x.is_finite() {
        x.abs()
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------------------------
// Pico amostral
// ---------------------------------------------------------------------------------------------

/// Medidor de pico amostral por canal com balística de exibição: sobe instantaneamente, mantém o
/// valor de *hold* por `hold_ms` e decai a `decay_db_per_s`.
#[derive(Debug, Clone)]
pub struct PeakMeter {
    sample_rate: f32,
    hold_ms: f32,
    decay_db_per_s: f32,
    level: [f32; 2],
    hold: [f32; 2],
    hold_left: [u32; 2],
    max: [f32; 2],
}

impl PeakMeter {
    pub fn new(hold_ms: f32, decay_db_per_s: f32) -> Self {
        let mut m = Self {
            sample_rate: 48_000.0,
            hold_ms: crate::util::clamp_param(hold_ms, 0.0, 10_000.0, 1_500.0),
            decay_db_per_s: crate::util::clamp_param(decay_db_per_s, 0.0, 1_000.0, 20.0),
            level: [0.0; 2],
            hold: [0.0; 2],
            hold_left: [0; 2],
            max: [0.0; 2],
        };
        m.clear();
        m
    }

    fn prepare_rate(&mut self, sr: f32) {
        self.sample_rate = safe_sr(sr);
        self.clear();
    }

    fn clear(&mut self) {
        self.level = [0.0; 2];
        self.hold = [0.0; 2];
        self.hold_left = [0; 2];
        self.max = [0.0; 2];
    }

    /// Analisa um bloco (sem alocar).
    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len().min(right.len());
        if n == 0 {
            return;
        }
        let hold_samples = (self.hold_ms * 0.001 * self.sample_rate) as u32;
        let decay = 10f32.powf(-self.decay_db_per_s * n as f32 / (20.0 * self.sample_rate));
        for (ch, buf) in [&left[..n], &right[..n]].into_iter().enumerate() {
            let pk = buf.iter().fold(0.0f32, |m, v| m.max(finite_abs(*v)));
            self.max[ch] = self.max[ch].max(pk);
            self.level[ch] = if pk >= self.level[ch] {
                pk
            } else {
                (self.level[ch] * decay).max(pk)
            };
            if pk >= self.hold[ch] {
                self.hold[ch] = pk;
                self.hold_left[ch] = hold_samples;
            } else if self.hold_left[ch] > n as u32 {
                self.hold_left[ch] -= n as u32;
            } else {
                self.hold_left[ch] = 0;
                self.hold[ch] = self.level[ch];
            }
        }
    }

    /// Nível de exibição (com decaimento), em dBFS por canal.
    pub fn level_db(&self) -> [f32; 2] {
        self.level.map(lin_to_db)
    }

    /// Valor de *hold*, em dBFS por canal.
    pub fn hold_db(&self) -> [f32; 2] {
        self.hold.map(lin_to_db)
    }

    /// Pico máximo desde o último `reset`, em dBFS por canal.
    pub fn max_db(&self) -> [f32; 2] {
        self.max.map(lin_to_db)
    }
}

impl Default for PeakMeter {
    fn default() -> Self {
        Self::new(1_500.0, 20.0)
    }
}

analyzer_processor!(PeakMeter);

// ---------------------------------------------------------------------------------------------
// RMS
// ---------------------------------------------------------------------------------------------

/// RMS em janela retangular deslizante (padrão 300 ms; máx. 3 s), por canal.
#[derive(Debug, Clone)]
pub struct RmsMeter {
    window_ms: f32,
    sq_l: Vec<f32>,
    sq_r: Vec<f32>,
    sum: [f64; 2],
    pos: usize,
}

impl RmsMeter {
    pub const MAX_WINDOW_MS: f32 = 3_000.0;

    pub fn new(window_ms: f32) -> Self {
        let mut m = Self {
            window_ms: crate::util::clamp_param(window_ms, 1.0, Self::MAX_WINDOW_MS, 300.0),
            sq_l: Vec::new(),
            sq_r: Vec::new(),
            sum: [0.0; 2],
            pos: 0,
        };
        m.prepare_rate(48_000.0);
        m
    }

    fn prepare_rate(&mut self, sr: f32) {
        let len = ((self.window_ms * 0.001 * safe_sr(sr)) as usize).max(1);
        self.sq_l = vec![0.0; len];
        self.sq_r = vec![0.0; len];
        self.clear();
    }

    fn clear(&mut self) {
        self.sq_l.iter_mut().for_each(|v| *v = 0.0);
        self.sq_r.iter_mut().for_each(|v| *v = 0.0);
        self.sum = [0.0; 2];
        self.pos = 0;
    }

    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let len = self.sq_l.len();
        if len == 0 || self.sq_r.len() != len {
            return;
        }
        let n = left.len().min(right.len());
        for i in 0..n {
            let (l, r) = (finite_abs(left[i]), finite_abs(right[i]));
            let (ql, qr) = (l * l, r * r);
            self.sum[0] += ql as f64 - self.sq_l[self.pos] as f64;
            self.sum[1] += qr as f64 - self.sq_r[self.pos] as f64;
            self.sq_l[self.pos] = ql;
            self.sq_r[self.pos] = qr;
            self.pos += 1;
            if self.pos >= len {
                self.pos = 0;
                // Uma vez por janela: soma exata, elimina deriva numérica.
                self.sum[0] = self.sq_l.iter().map(|v| *v as f64).sum();
                self.sum[1] = self.sq_r.iter().map(|v| *v as f64).sum();
            }
        }
    }

    /// RMS linear por canal.
    pub fn rms(&self) -> [f32; 2] {
        let len = self.sq_l.len().max(1) as f64;
        self.sum.map(|s| (s.max(0.0) / len).sqrt() as f32)
    }

    /// RMS em dBFS por canal (seno de pico 1 → −3,01 dB).
    pub fn rms_db(&self) -> [f32; 2] {
        self.rms().map(lin_to_db)
    }
}

impl Default for RmsMeter {
    fn default() -> Self {
        Self::new(300.0)
    }
}

analyzer_processor!(RmsMeter);

// ---------------------------------------------------------------------------------------------
// Clipping
// ---------------------------------------------------------------------------------------------

/// Conta eventos de clipping (sequências de amostras com |x| ≥ limiar) e amostras clipadas.
#[derive(Debug, Clone)]
pub struct ClipCounter {
    threshold: f32,
    in_clip: [bool; 2],
    events: u64,
    samples: u64,
}

impl ClipCounter {
    /// `threshold` linear (1,0 = 0 dBFS).
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold: crate::util::clamp_param(threshold, 1.0e-6, 1.0e6, 1.0),
            in_clip: [false; 2],
            events: 0,
            samples: 0,
        }
    }

    fn prepare_rate(&mut self, _sr: f32) {
        self.clear();
    }

    fn clear(&mut self) {
        self.in_clip = [false; 2];
        self.events = 0;
        self.samples = 0;
    }

    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len().min(right.len());
        for (ch, buf) in [&left[..n], &right[..n]].into_iter().enumerate() {
            for v in buf {
                // Não finito também conta como clipping (sinal inválido).
                let clip = !v.is_finite() || v.abs() >= self.threshold;
                if clip {
                    self.samples += 1;
                    if !self.in_clip[ch] {
                        self.events += 1;
                    }
                }
                self.in_clip[ch] = clip;
            }
        }
    }

    pub fn events(&self) -> u64 {
        self.events
    }

    pub fn clipped_samples(&self) -> u64 {
        self.samples
    }
}

impl Default for ClipCounter {
    fn default() -> Self {
        Self::new(1.0)
    }
}

analyzer_processor!(ClipCounter);

// ---------------------------------------------------------------------------------------------
// Correlação
// ---------------------------------------------------------------------------------------------

/// Correlação de fase estéreo (−1..+1) com médias exponenciais (constante de tempo 300 ms).
/// Silêncio em algum canal → 0.
#[derive(Debug, Clone)]
pub struct CorrelationMeter {
    coef: f64,
    lr: f64,
    ll: f64,
    rr: f64,
}

impl CorrelationMeter {
    pub const TIME_MS: f32 = 300.0;

    pub fn new() -> Self {
        let mut m = Self {
            coef: 0.0,
            lr: 0.0,
            ll: 0.0,
            rr: 0.0,
        };
        m.prepare_rate(48_000.0);
        m
    }

    fn prepare_rate(&mut self, sr: f32) {
        self.coef = crate::util::time_coef(Self::TIME_MS, safe_sr(sr)) as f64;
        self.clear();
    }

    fn clear(&mut self) {
        self.lr = 0.0;
        self.ll = 0.0;
        self.rr = 0.0;
    }

    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len().min(right.len());
        let a = self.coef;
        let b = 1.0 - a;
        for i in 0..n {
            let l = if left[i].is_finite() {
                left[i] as f64
            } else {
                0.0
            };
            let r = if right[i].is_finite() {
                right[i] as f64
            } else {
                0.0
            };
            self.lr = a * self.lr + b * l * r;
            self.ll = a * self.ll + b * l * l;
            self.rr = a * self.rr + b * r * r;
        }
        if self.ll < 1e-30 {
            self.ll = 0.0;
        }
        if self.rr < 1e-30 {
            self.rr = 0.0;
        }
    }

    pub fn correlation(&self) -> f32 {
        let d = self.ll * self.rr;
        if d <= 1e-20 {
            0.0
        } else {
            (self.lr / d.sqrt()).clamp(-1.0, 1.0) as f32
        }
    }
}

impl Default for CorrelationMeter {
    fn default() -> Self {
        Self::new()
    }
}

analyzer_processor!(CorrelationMeter);

// ---------------------------------------------------------------------------------------------
// True peak
// ---------------------------------------------------------------------------------------------

const TP_OVERSAMPLE: usize = 4;
const TP_TAPS_PER_PHASE: usize = 12;

/// Coeficientes polifásicos do interpolador 4× (sinc janelado por Hann, 48 taps), cada fase
/// normalizada para ganho DC unitário.
fn true_peak_phases() -> [[f32; TP_TAPS_PER_PHASE]; TP_OVERSAMPLE] {
    let n = TP_OVERSAMPLE * TP_TAPS_PER_PHASE;
    let center = (n - 1) as f64 / 2.0;
    let mut phases = [[0.0f32; TP_TAPS_PER_PHASE]; TP_OVERSAMPLE];
    for (p, phase) in phases.iter_mut().enumerate() {
        let mut taps = [0.0f64; TP_TAPS_PER_PHASE];
        for (j, t) in taps.iter_mut().enumerate() {
            let i = j * TP_OVERSAMPLE + p;
            let x = (i as f64 - center) / TP_OVERSAMPLE as f64;
            let sinc = if x.abs() < 1e-12 {
                1.0
            } else {
                (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
            };
            let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos();
            *t = sinc * w;
        }
        let sum: f64 = taps.iter().sum();
        for (dst, t) in phase.iter_mut().zip(taps) {
            *dst = (t / sum) as f32;
        }
    }
    phases
}

/// Medidor de *true peak* (BS.1770-4, Anexo 2): interpolação 4× e máximo absoluto.
/// Detecta picos entre amostras que o medidor amostral não vê. Leitura em dBTP.
#[derive(Debug, Clone)]
pub struct TruePeakMeter {
    phases: [[f32; TP_TAPS_PER_PHASE]; TP_OVERSAMPLE],
    hist: [[f32; TP_TAPS_PER_PHASE]; 2],
    max: [f32; 2],
}

impl TruePeakMeter {
    pub fn new() -> Self {
        Self {
            phases: true_peak_phases(),
            hist: [[0.0; TP_TAPS_PER_PHASE]; 2],
            max: [0.0; 2],
        }
    }

    fn prepare_rate(&mut self, _sr: f32) {
        self.clear();
    }

    fn clear(&mut self) {
        self.hist = [[0.0; TP_TAPS_PER_PHASE]; 2];
        self.max = [0.0; 2];
    }

    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len().min(right.len());
        for (ch, buf) in [&left[..n], &right[..n]].into_iter().enumerate() {
            let hist = &mut self.hist[ch];
            let mut m = self.max[ch];
            for v in buf {
                hist.copy_within(0..TP_TAPS_PER_PHASE - 1, 1);
                hist[0] = if v.is_finite() { *v } else { 0.0 };
                m = m.max(hist[0].abs());
                for phase in &self.phases {
                    let y: f32 = phase.iter().zip(hist.iter()).map(|(h, x)| h * x).sum();
                    m = m.max(y.abs());
                }
            }
            self.max[ch] = m;
        }
    }

    /// Máximo *true peak* desde o último `reset`, em dBTP por canal.
    pub fn true_peak_db(&self) -> [f32; 2] {
        self.max.map(lin_to_db)
    }

    /// Máximo dos dois canais, em dBTP.
    pub fn max_db(&self) -> f32 {
        lin_to_db(self.max[0].max(self.max[1]))
    }
}

impl Default for TruePeakMeter {
    fn default() -> Self {
        Self::new()
    }
}

analyzer_processor!(TruePeakMeter);

// ---------------------------------------------------------------------------------------------
// Loudness (BS.1770-4 / EBU R128)
// ---------------------------------------------------------------------------------------------

/// Coeficientes da ponderação K (pré-filtro *shelf* + RLB passa-alta) para qualquer taxa,
/// pelas fórmulas analógicas usadas em libebur128; em 48 kHz reproduzem a Tabela 1/2 da BS.1770-4.
pub fn k_weighting_coeffs(sample_rate: f32) -> (BiquadCoeffs, BiquadCoeffs) {
    let sr = safe_sr(sample_rate) as f64;
    let pi = std::f64::consts::PI;
    // Estágio 1: high-shelf (+4 dB acima de ~1,7 kHz; modelo da cabeça).
    let f0 = 1_681.974_450_955_533;
    let g = 3.999_843_853_973_347;
    let q = 0.707_175_236_955_419_6;
    let k = (pi * f0 / sr).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;
    let pre = BiquadCoeffs {
        b0: (vh + vb * k / q + k * k) / a0,
        b1: 2.0 * (k * k - vh) / a0,
        b2: (vh - vb * k / q + k * k) / a0,
        a1: 2.0 * (k * k - 1.0) / a0,
        a2: (1.0 - k / q + k * k) / a0,
    };
    // Estágio 2: RLB (passa-alta de 2ª ordem ~38 Hz).
    let f0 = 38.135_470_876_024_44;
    let q = 0.500_327_037_323_877_3;
    let k = (pi * f0 / sr).tan();
    let a0 = 1.0 + k / q + k * k;
    let rlb = BiquadCoeffs {
        b0: 1.0,
        b1: -2.0,
        b2: 1.0,
        a1: 2.0 * (k * k - 1.0) / a0,
        a2: (1.0 - k / q + k * k) / a0,
    };
    (pre, rlb)
}

/// Loudness a partir da energia média ponderada (soma dos canais): `−0,691 + 10·log10(z)`.
#[inline]
fn energy_to_lufs(z: f64) -> f32 {
    if z <= 0.0 || !z.is_finite() {
        DB_FLOOR
    } else {
        ((-0.691 + 10.0 * z.log10()) as f32).max(DB_FLOOR)
    }
}

const SUBBLOCKS_MOMENTARY: usize = 4; // 400 ms
const SUBBLOCKS_SHORT: usize = 30; // 3 s
const ABS_GATE_LUFS: f64 = -70.0;
const REL_GATE_LU: f64 = -10.0;
const HIST_MIN: f64 = -70.0;
const HIST_STEP: f64 = 0.01;
const HIST_BINS: usize = 8_000; // −70 … +10 LUFS

/// Medidor de loudness ITU-R BS.1770-4 / EBU R128 (estéreo L/R, pesos 1,0).
///
/// - Ponderação K em `f64`.
/// - Energia acumulada em sub-blocos de 100 ms; *momentary* = 4 sub-blocos (400 ms),
///   *short-term* = 30 sub-blocos (3 s), atualizados a cada 100 ms (≥ 10 Hz, EBU Tech 3341).
/// - *Integrated*: blocos de 400 ms com 75 % de sobreposição, porta absoluta −70 LUFS e porta
///   relativa −10 LU; blocos guardados num histograma pré-alocado (8000 classes de 0,01 LU) com
///   soma de energia exata por classe — sem alocação e com memória constante.
#[derive(Debug, Clone)]
pub struct LoudnessMeter {
    sample_rate: f32,
    pre: BiquadCoeffs,
    rlb: BiquadCoeffs,
    st: [[BiquadState; 2]; 2],
    sub_len: usize,
    sub_count: usize,
    sub_acc: f64,
    ring: [f64; SUBBLOCKS_SHORT],
    ring_pos: usize,
    filled: usize,
    momentary: f32,
    short_term: f32,
    max_momentary: f32,
    hist_count: Box<[u64]>,
    hist_energy: Box<[f64]>,
}

impl LoudnessMeter {
    pub fn new() -> Self {
        let mut m = Self {
            sample_rate: 48_000.0,
            pre: BiquadCoeffs::IDENTITY,
            rlb: BiquadCoeffs::IDENTITY,
            st: [[BiquadState::default(); 2]; 2],
            sub_len: 4_800,
            sub_count: 0,
            sub_acc: 0.0,
            ring: [0.0; SUBBLOCKS_SHORT],
            ring_pos: 0,
            filled: 0,
            momentary: DB_FLOOR,
            short_term: DB_FLOOR,
            max_momentary: DB_FLOOR,
            hist_count: vec![0; HIST_BINS].into_boxed_slice(),
            hist_energy: vec![0.0; HIST_BINS].into_boxed_slice(),
        };
        m.prepare_rate(48_000.0);
        m
    }

    fn prepare_rate(&mut self, sr: f32) {
        self.sample_rate = safe_sr(sr);
        let (pre, rlb) = k_weighting_coeffs(self.sample_rate);
        self.pre = pre;
        self.rlb = rlb;
        self.sub_len = ((self.sample_rate as f64 * 0.1).round() as usize).max(1);
        self.clear();
    }

    fn clear(&mut self) {
        self.st = [[BiquadState::default(); 2]; 2];
        self.sub_count = 0;
        self.sub_acc = 0.0;
        self.ring = [0.0; SUBBLOCKS_SHORT];
        self.ring_pos = 0;
        self.filled = 0;
        self.momentary = DB_FLOOR;
        self.short_term = DB_FLOOR;
        self.max_momentary = DB_FLOOR;
        self.hist_count.iter_mut().for_each(|v| *v = 0);
        self.hist_energy.iter_mut().for_each(|v| *v = 0.0);
    }

    pub fn analyze(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len().min(right.len());
        for i in 0..n {
            let mut z = 0.0;
            for (ch, x) in [left[i], right[i]].into_iter().enumerate() {
                let x = if x.is_finite() { x as f64 } else { 0.0 };
                let [s_pre, s_rlb] = &mut self.st[ch];
                let y = s_rlb.tick(&self.rlb, s_pre.tick(&self.pre, x));
                z += y * y;
            }
            self.sub_acc += z;
            self.sub_count += 1;
            if self.sub_count >= self.sub_len {
                self.finish_subblock();
            }
        }
        for s in self.st.iter_mut().flatten() {
            s.flush();
        }
    }

    fn finish_subblock(&mut self) {
        self.ring[self.ring_pos] = self.sub_acc / self.sub_len as f64;
        self.ring_pos = (self.ring_pos + 1) % SUBBLOCKS_SHORT;
        self.sub_acc = 0.0;
        self.sub_count = 0;
        self.filled = (self.filled + 1).min(usize::MAX - 1);
        let mean_last = |k: usize| -> f64 {
            let mut s = 0.0;
            for j in 1..=k {
                s += self.ring[(self.ring_pos + SUBBLOCKS_SHORT - j) % SUBBLOCKS_SHORT];
            }
            s / k as f64
        };
        let m_energy = mean_last(SUBBLOCKS_MOMENTARY);
        self.momentary = energy_to_lufs(m_energy);
        self.short_term = energy_to_lufs(mean_last(SUBBLOCKS_SHORT));
        if self.filled >= SUBBLOCKS_MOMENTARY {
            self.max_momentary = self.max_momentary.max(self.momentary);
            // Bloco de porta (400 ms, passo de 100 ms = 75 % de sobreposição).
            let l = -0.691 + 10.0 * m_energy.max(1e-300).log10();
            if l > ABS_GATE_LUFS {
                let idx = (((l - HIST_MIN) / HIST_STEP) as usize).min(HIST_BINS - 1);
                self.hist_count[idx] += 1;
                self.hist_energy[idx] += m_energy;
            }
        }
    }

    /// Loudness *momentary* (400 ms), em LUFS (silêncio → −144).
    pub fn momentary_lufs(&self) -> f32 {
        self.momentary
    }

    /// Loudness *short-term* (3 s), em LUFS.
    pub fn short_term_lufs(&self) -> f32 {
        self.short_term
    }

    /// Maior *momentary* desde o último `reset`.
    pub fn max_momentary_lufs(&self) -> f32 {
        self.max_momentary
    }

    /// Loudness integrado com portas absoluta e relativa; `None` se nenhum bloco passou.
    /// Custo O(8000) sem alocação — chamar da thread de controle, não por amostra.
    pub fn integrated_lufs(&self) -> Option<f32> {
        let (mut count, mut energy) = (0u64, 0.0f64);
        for (c, e) in self.hist_count.iter().zip(self.hist_energy.iter()) {
            count += c;
            energy += e;
        }
        if count == 0 {
            return None;
        }
        let rel_gate = -0.691 + 10.0 * (energy / count as f64).log10() + REL_GATE_LU;
        let (mut count2, mut energy2) = (0u64, 0.0f64);
        for (c, e) in self.hist_count.iter().zip(self.hist_energy.iter()) {
            if *c == 0 {
                continue;
            }
            // Loudness média da classe (precisão sub-0,01 LU na comparação com a porta).
            let l = -0.691 + 10.0 * (e / *c as f64).log10();
            if l > rel_gate {
                count2 += c;
                energy2 += e;
            }
        }
        if count2 == 0 {
            return None;
        }
        Some(energy_to_lufs(energy2 / count2 as f64))
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}

impl Default for LoudnessMeter {
    fn default() -> Self {
        Self::new()
    }
}

analyzer_processor!(LoudnessMeter);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{noise, sine};
    use crate::util::db_to_lin;

    const SR: f32 = 48_000.0;

    fn feed<M: Processor>(m: &mut M, l: &[f32], r: &[f32]) {
        let mut l = l.to_vec();
        let mut r = r.to_vec();
        crate::signals::process_in_blocks(m, &mut l, &mut r, 512);
    }

    // ---- Pico / RMS / clipping ----

    #[test]
    fn peak_meter_holds_then_decays() {
        let mut m = PeakMeter::new(500.0, 20.0);
        m.prepare(SR, 128);
        m.analyze(&[0.5, -1.0, 0.25], &[0.1, 0.2, 0.3]);
        assert!((m.level_db()[0]).abs() < 1e-6);
        assert!((m.hold_db()[1] - lin_to_db(0.3)).abs() < 1e-5);
        // 250 ms de silêncio: hold mantido, nível decaiu 5 dB.
        let z = vec![0.0f32; 12_000];
        m.analyze(&z, &z);
        assert!((m.level_db()[0] + 5.0).abs() < 0.01, "{:?}", m.level_db());
        assert!(m.hold_db()[0].abs() < 1e-6);
        // Mais 500 ms: hold expirou e acompanha o nível.
        let z = vec![0.0f32; 24_000];
        m.analyze(&z, &z);
        assert!(m.hold_db()[0] < -14.0);
        assert!(m.max_db()[0].abs() < 1e-6, "máximo absoluto preservado");
    }

    #[test]
    fn rms_of_full_scale_sine_is_minus_3db() {
        let mut m = RmsMeter::new(300.0);
        m.prepare(SR, 128);
        let x = sine(1_000.0, 1.0, SR, 48_000, 0.0);
        let y = sine(1_000.0, 0.5, SR, 48_000, 0.0);
        feed(&mut m, &x, &y);
        let [l, r] = m.rms_db();
        assert!((l + 3.0103).abs() < 0.01, "{l}");
        assert!((r + 9.0309).abs() < 0.01, "{r}");
    }

    #[test]
    fn clip_counter_counts_events_and_samples() {
        let mut c = ClipCounter::default();
        c.analyze(&[0.5, 1.0, 1.2, 0.3, -1.0], &[0.0, 0.0, 0.0, 0.0, f32::NAN]);
        assert_eq!(c.events(), 3);
        assert_eq!(c.clipped_samples(), 4);
        c.reset();
        assert_eq!(c.events(), 0);
    }

    // ---- Correlação ----

    #[test]
    fn correlation_identical_inverted_and_uncorrelated() {
        let x = noise(48_000, 0.5, 11);
        let inv: Vec<f32> = x.iter().map(|v| -v).collect();
        let other = noise(48_000, 0.5, 99);
        let mut m = CorrelationMeter::new();
        m.prepare(SR, 128);
        m.analyze(&x, &x);
        assert!((m.correlation() - 1.0).abs() < 1e-6);
        m.reset();
        m.analyze(&x, &inv);
        assert!((m.correlation() + 1.0).abs() < 1e-6);
        m.reset();
        m.analyze(&x, &other);
        assert!(m.correlation().abs() < 0.2, "{}", m.correlation());
        m.reset();
        m.analyze(&x, &[0.0; 48_000]);
        assert_eq!(m.correlation(), 0.0);
    }

    // ---- True peak ----

    #[test]
    fn true_peak_detects_intersample_peak() {
        // Seno em fs/4 com fase de 45°: amostras em ±0,707·A, pico real A.
        let x = sine(SR / 4.0, 1.0, SR, 4_800, std::f64::consts::FRAC_PI_4);
        let sample_peak = crate::signals::peak_db(&x);
        assert!((sample_peak + 3.01).abs() < 0.01);
        let mut m = TruePeakMeter::new();
        m.prepare(SR, 128);
        feed(&mut m, &x, &x);
        let tp = m.max_db();
        assert!(
            tp > sample_peak + 2.5,
            "true peak {tp} vs amostral {sample_peak}"
        );
        assert!(tp.abs() < 0.5, "≈ 0 dBTP: {tp}");
    }

    #[test]
    fn true_peak_of_low_frequency_matches_sample_peak() {
        let x = sine(1_000.0, 0.5, SR, 48_000, 0.0);
        let mut m = TruePeakMeter::new();
        m.analyze(&x, &x);
        assert!((m.true_peak_db()[0] - lin_to_db(0.5)).abs() < 0.05);
    }

    #[test]
    fn true_peak_phases_have_unity_dc_gain() {
        for p in true_peak_phases() {
            assert!((p.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        }
    }

    // ---- Loudness ----

    #[test]
    fn k_weighting_matches_bs1770_table_at_48k() {
        // ITU-R BS.1770-4, Anexo 1, Tabelas 1 e 2.
        let (pre, rlb) = k_weighting_coeffs(48_000.0);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-8;
        assert!(close(pre.b0, 1.535_124_859_586_97));
        assert!(close(pre.b1, -2.691_696_189_406_38));
        assert!(close(pre.b2, 1.198_392_810_852_85));
        assert!(close(pre.a1, -1.690_659_293_182_41));
        assert!(close(pre.a2, 0.732_480_774_215_85));
        assert!(close(rlb.a1, -1.990_047_454_833_98));
        assert!(close(rlb.a2, 0.990_072_250_366_21));
    }

    /// EBU Tech 3341 (v4), caso 1: seno de 1 kHz a −23 dBFS nos dois canais por 20 s →
    /// M, S e I = −23,0 ± 0,1 LUFS.
    ///
    /// Derivação: a BS.1770 define o offset −0,691 para que um seno de 997 Hz a 0 dBFS em **um**
    /// canal meça −3,01 LKFS (ganho K em 1 kHz ≈ +0,691 dB, energia média A²/2 → −3,01 dB).
    /// Com o mesmo sinal em L e R (pesos 1,0) a energia dobra (+3,01 dB) → 0 LUFS a 0 dBFS;
    /// logo −23 dBFS ⇒ −23 LUFS (1 kHz vs 997 Hz difere < 0,01 dB).
    #[test]
    fn ebu_3341_case1_sine_minus_23_dbfs_reads_minus_23_lufs() {
        let x = sine(1_000.0, db_to_lin(-23.0), SR, 20 * 48_000, 0.0);
        let mut m = LoudnessMeter::new();
        m.prepare(SR, 512);
        feed(&mut m, &x, &x);
        assert!(
            (m.momentary_lufs() + 23.0).abs() < 0.1,
            "M = {}",
            m.momentary_lufs()
        );
        assert!(
            (m.short_term_lufs() + 23.0).abs() < 0.1,
            "S = {}",
            m.short_term_lufs()
        );
        let i = m.integrated_lufs().unwrap();
        assert!((i + 23.0).abs() < 0.1, "I = {i}");
    }

    #[test]
    fn mono_997hz_full_scale_reads_minus_3_01() {
        // BS.1770-4: 997 Hz a 0 dBFS num canal → −3,01 LKFS.
        let x = sine(997.0, 1.0, SR, 2 * 48_000, 0.0);
        let z = vec![0.0f32; x.len()];
        let mut m = LoudnessMeter::new();
        m.prepare(SR, 512);
        feed(&mut m, &x, &z);
        assert!(
            (m.momentary_lufs() + 3.01).abs() < 0.05,
            "{}",
            m.momentary_lufs()
        );
    }

    /// EBU Tech 3341, caso 4: −72/−36/−23/−36/−72 dBFS (10/10/60/10/10 s) → I = −23,0 ± 0,1.
    /// Exercita as portas absoluta (−72 descartado) e relativa (−36 descartado).
    #[test]
    fn ebu_3341_case4_gating() {
        let mut x = Vec::new();
        for (db, secs) in [
            (-72.0, 10),
            (-36.0, 10),
            (-23.0, 60),
            (-36.0, 10),
            (-72.0, 10),
        ] {
            x.extend(sine(1_000.0, db_to_lin(db), SR, secs * 48_000, 0.0));
        }
        let mut m = LoudnessMeter::new();
        m.prepare(SR, 512);
        feed(&mut m, &x, &x);
        let i = m.integrated_lufs().unwrap();
        assert!((i + 23.0).abs() < 0.1, "I = {i}");
    }

    #[test]
    fn silence_has_no_integrated_loudness_and_works_at_44k1() {
        let mut m = LoudnessMeter::new();
        m.prepare(44_100.0, 512);
        let z = vec![0.0f32; 44_100];
        feed(&mut m, &z, &z);
        assert_eq!(m.integrated_lufs(), None);
        assert_eq!(m.momentary_lufs(), DB_FLOOR);
        let x = sine(1_000.0, db_to_lin(-20.0), 44_100.0, 5 * 44_100, 0.0);
        feed(&mut m, &x, &x);
        assert!(
            (m.short_term_lufs() + 20.0).abs() < 0.1,
            "{}",
            m.short_term_lufs()
        );
    }
}
