//! Pitch shift em tempo real (porta `PitchShifter`) e implementação PSOLA síncrona ao período.
//!
//! Algoritmo (TD-PSOLA com grãos reamostrados):
//! 1. Um detector YIN estima o período `P` da entrada a cada `detect_hop` amostras
//!    (sem altura confiável → período padrão, comportamento granular).
//! 2. Marcas de síntese avançam `P / razão` (o novo período). A marca de análise seguinte é
//!    `a + m·P` (múltiplo inteiro do período do grão anterior) mais próxima da nova marca de
//!    síntese — mantém coerência de fase e preserva o tempo (com razão 1, `a = s` sempre).
//! 3. Para cada par (`s`, `a`) soma-se um grão com janela Hann de meia largura `P / razão`,
//!    lido em `x(a + t·razão)` (interpolação cúbica). A soma das janelas é acumulada e usada
//!    para normalizar a saída (exata mesmo quando o período muda entre grãos).
//!
//! A latência é **constante** e igual a [`PitchShifter::latency_samples`]: com razão 1 a saída
//! é a entrada atrasada exatamente dessa quantidade. Reamostrar o grão desloca também os
//! formantes (aceitável para correções de até ±2 semitons; preservação de formantes fica para
//! uma volta futura — ver ADR-0016).

use crate::detector::{finite_or, PitchDetector, PitchEstimate, YinConfig, YinDetector};

/// Menor/maior razão de altura aceitas (±2 oitavas).
pub const RATIO_RANGE: (f32, f32) = (0.25, 4.0);

/// Porta de pitch shift (AI-ready: um shifter neural pode implementar a mesma trait).
///
/// Contrato: `prepare` aloca; `process*` não aloca, não bloqueia e não entra em pânico.
pub trait PitchShifter: Send {
    fn prepare(&mut self, sample_rate: f32, max_block: usize);
    fn reset(&mut self);
    /// Latência constante introduzida (amostras).
    fn latency_samples(&self) -> usize;
    /// Processa com razão constante no bloco (2,0 = uma oitava acima).
    fn process(&mut self, input: &[f32], output: &mut [f32], ratio: f32);
    /// Processa com razão por amostra (`ratios.len() >= input.len()`; falta → última razão).
    fn process_varying(&mut self, input: &[f32], output: &mut [f32], ratios: &[f32]);
}

/// Configuração do PSOLA.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PsolaConfig {
    /// Detector usado para o período.
    pub yin: YinConfig,
    /// Menor razão suportada sem truncar grãos (define a latência).
    pub min_ratio: f32,
    /// Período usado em trechos não vozeados (ms).
    pub unvoiced_period_ms: f32,
    /// Intervalo entre detecções (ms).
    pub detect_interval_ms: f32,
    /// Confiança mínima para usar o período detectado.
    pub min_confidence: f32,
}

impl Default for PsolaConfig {
    fn default() -> Self {
        Self {
            yin: YinConfig::default(),
            min_ratio: 0.5,
            unvoiced_period_ms: 6.0,
            detect_interval_ms: 5.0,
            min_confidence: 0.6,
        }
    }
}

/// Converte semitons em razão de frequência.
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    2f32.powf(semitones / 12.0)
}

/// Pitch shifter PSOLA de latência constante, sem alocação após `prepare`.
#[derive(Debug, Clone)]
pub struct PsolaShifter {
    config: PsolaConfig,
    sample_rate: f32,
    detector: YinDetector,
    max_period: f64,
    min_ratio: f64,
    latency: usize,
    input: Vec<f32>,
    acc: Vec<f32>,
    wsum: Vec<f32>,
    mask: usize,
    written: u64,
    synth_mark: f64,
    analysis_mark: f64,
    period: f64,
    unvoiced_period: f64,
    detect_buf: Vec<f32>,
    detect_hop: usize,
    since_detect: usize,
    estimate: PitchEstimate,
    last_ratio: f32,
}

impl PsolaShifter {
    pub fn new(config: PsolaConfig, sample_rate: f32) -> Self {
        let mut s = Self {
            config,
            sample_rate: 0.0,
            detector: YinDetector::new(config.yin, sample_rate),
            max_period: 1.0,
            min_ratio: 0.5,
            latency: 0,
            input: Vec::new(),
            acc: Vec::new(),
            wsum: Vec::new(),
            mask: 0,
            written: 0,
            synth_mark: 0.0,
            analysis_mark: 0.0,
            period: 1.0,
            unvoiced_period: 1.0,
            detect_buf: Vec::new(),
            detect_hop: 1,
            since_detect: 0,
            estimate: PitchEstimate::UNVOICED,
            last_ratio: 1.0,
        };
        s.prepare(sample_rate, 0);
        s
    }

    /// Última estimativa de altura da entrada (atualizada a cada `detect_interval_ms`).
    pub fn last_estimate(&self) -> PitchEstimate {
        self.estimate
    }

    /// Período atual usado nos grãos (amostras).
    pub fn current_period(&self) -> f32 {
        self.period as f32
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Processa uma amostra com razão constante.
    #[inline]
    pub fn process_sample(&mut self, x: f32, ratio: f32) -> f32 {
        self.process_sample_with(x, |_| ratio)
    }

    /// Processa uma amostra; `ratio_at(posição)` dá a razão desejada para o grão centrado na
    /// posição de entrada indicada (útil em render offline, onde a curva de razão é conhecida).
    pub fn process_sample_with<F: Fn(f64) -> f32>(&mut self, x: f32, ratio_at: F) -> f32 {
        let idx = self.written;
        self.input[(idx as usize) & self.mask] = if x.is_finite() { x } else { 0.0 };
        self.written += 1;

        self.since_detect += 1;
        if self.since_detect >= self.detect_hop {
            self.since_detect = 0;
            self.update_period();
        }

        self.place_grains(idx as f64, &ratio_at);

        let out = idx as i64 - self.latency as i64;
        if out < 0 {
            return 0.0;
        }
        let pos = (out as usize) & self.mask;
        let w = self.wsum[pos];
        let y = if w > 1e-6 {
            self.acc[pos] / w.max(0.5)
        } else {
            0.0
        };
        self.acc[pos] = 0.0;
        self.wsum[pos] = 0.0;
        y
    }

    fn update_period(&mut self) {
        let n = self.detect_buf.len();
        let end = self.written as i64;
        for (i, slot) in self.detect_buf.iter_mut().enumerate() {
            let p = end - n as i64 + i as i64;
            *slot = if p >= 0 {
                self.input[(p as usize) & self.mask]
            } else {
                0.0
            };
        }
        self.estimate = self.detector.detect(&self.detect_buf);
        self.period = match self.estimate.pitch(self.config.min_confidence) {
            Some(hz) => (self.sample_rate as f64 / hz as f64).clamp(2.0, self.max_period),
            None => self.unvoiced_period,
        };
    }

    fn place_grains<F: Fn(f64) -> f32>(&mut self, newest: f64, ratio_at: &F) {
        let p = self.period;
        loop {
            let s = self.synth_mark;
            let a = self.analysis_mark;
            // O grão lê x(a ± P): precisa da entrada até a + P + 2 (interpolação cúbica).
            if a + p + 2.0 > newest {
                break;
            }
            let r = clamp_ratio(ratio_at(s)).max(self.min_ratio as f32) as f64;
            self.last_ratio = r as f32;
            let half = p / r;
            let k0 = (s - half).ceil() as i64;
            let k1 = (s + half).floor() as i64;
            let inv_half = std::f64::consts::PI / half;
            for k in k0.max(0)..=k1 {
                let t = k as f64 - s;
                let w = (0.5 + 0.5 * (t * inv_half).cos()) as f32;
                let v = self.read_input(a + t * r);
                let pos = (k as usize) & self.mask;
                self.acc[pos] += w * v;
                self.wsum[pos] += w;
            }
            // Próxima marca de síntese: um novo período adiante. Próxima marca de análise: um
            // múltiplo inteiro do período *deste* grão a partir de `a`, o mais perto possível
            // de `s` (coerência de fase + preservação do tempo; com razão 1, a = s sempre).
            let next_s = s + half;
            let m = ((next_s - a) / p).round().max(0.0);
            self.analysis_mark = a + m * p;
            self.synth_mark = next_s;
        }
    }

    #[inline]
    fn read_input(&self, pos: f64) -> f32 {
        let i = pos.floor();
        let f = (pos - i) as f32;
        let i = i as i64;
        let at = |k: i64| -> f32 {
            if k < 0 || k >= self.written as i64 {
                0.0
            } else {
                self.input[(k as usize) & self.mask]
            }
        };
        let (y0, y1, y2, y3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        // Catmull-Rom.
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
        ((c3 * f + c2) * f + c1) * f + y1
    }
}

#[inline]
pub(crate) fn clamp_ratio(r: f32) -> f32 {
    finite_or(r, 1.0).clamp(RATIO_RANGE.0, RATIO_RANGE.1)
}

impl PitchShifter for PsolaShifter {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        let sr = finite_or(sample_rate, 48_000.0).clamp(8_000.0, 384_000.0);
        self.sample_rate = sr;
        self.detector = YinDetector::new(self.config.yin, sr);
        let yin = self.detector.config();
        self.max_period = (sr / yin.min_hz) as f64;
        self.min_ratio = finite_or(self.config.min_ratio, 0.5).clamp(RATIO_RANGE.0, 1.0) as f64;
        // Grão aceito quando a + P + 2 <= n, com a >= s − P/2; a saída lê s − P/razão.
        // Logo L >= 1,5·Pmax + Pmax/razão_min + 3.
        self.latency =
            (1.5 * self.max_period + self.max_period / self.min_ratio).ceil() as usize + 4;
        let size = (4 * (self.latency + self.max_period as usize + 8)).next_power_of_two();
        self.input = vec![0.0; size];
        self.acc = vec![0.0; size];
        self.wsum = vec![0.0; size];
        self.mask = size - 1;
        self.detect_buf = vec![0.0; self.detector.frame_len()];
        self.detect_hop = ((finite_or(self.config.detect_interval_ms, 5.0).clamp(0.5, 50.0) * sr
            / 1000.0) as usize)
            .max(1);
        self.unvoiced_period =
            (finite_or(self.config.unvoiced_period_ms, 6.0).clamp(1.0, 20.0) as f64 * sr as f64
                / 1000.0)
                .min(self.max_period);
        self.reset();
    }

    fn reset(&mut self) {
        self.input.fill(0.0);
        self.acc.fill(0.0);
        self.wsum.fill(0.0);
        self.written = 0;
        self.synth_mark = 0.0;
        self.analysis_mark = 0.0;
        self.period = self.unvoiced_period;
        self.since_detect = 0;
        self.estimate = PitchEstimate::UNVOICED;
        self.last_ratio = 1.0;
    }

    fn latency_samples(&self) -> usize {
        self.latency
    }

    fn process(&mut self, input: &[f32], output: &mut [f32], ratio: f32) {
        for (x, y) in input.iter().zip(output.iter_mut()) {
            *y = self.process_sample(*x, ratio);
        }
    }

    fn process_varying(&mut self, input: &[f32], output: &mut [f32], ratios: &[f32]) {
        let mut last = self.last_ratio;
        for (i, (x, y)) in input.iter().zip(output.iter_mut()).enumerate() {
            if let Some(r) = ratios.get(i) {
                last = *r;
            }
            *y = self.process_sample(*x, last);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::cents_between;
    use crate::signal;

    const SR: f32 = 48_000.0;

    fn detect_tail(sig: &[f32]) -> f32 {
        let mut yin = YinDetector::new(YinConfig::default(), SR);
        let n = yin.frame_len();
        let start = sig.len() - n - 2000;
        let est = yin.detect(&sig[start..start + n]);
        assert!(est.voiced, "saída deveria ser periódica");
        est.frequency_hz
    }

    fn shift(sig: &[f32], ratio: f32) -> (Vec<f32>, usize) {
        let mut s = PsolaShifter::new(PsolaConfig::default(), SR);
        let mut out = vec![0.0; sig.len()];
        for (i, block) in sig.chunks(256).enumerate() {
            s.process(block, &mut out[i * 256..i * 256 + block.len()], ratio);
        }
        (out, s.latency_samples())
    }

    #[test]
    fn octave_up_on_220_hz_sine_yields_440_hz() {
        let sig = signal::sine(220.0, SR, 48_000, 0.5);
        let (out, _) = shift(&sig, semitones_to_ratio(12.0));
        assert!(out.iter().all(|v| v.is_finite()));
        let f = detect_tail(&out);
        assert!(cents_between(440.0, f).abs() < 5.0, "obtido {f} Hz");
        let peak = out[out.len() / 2..]
            .iter()
            .fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(
            (0.35..=0.65).contains(&peak),
            "amplitude preservada: {peak}"
        );
    }

    #[test]
    fn shifts_down_and_by_small_intervals() {
        for (semi, f0) in [
            (-12.0f32, 440.0f32),
            (-3.0, 330.0),
            (1.0, 196.0),
            (5.0, 110.0),
        ] {
            let sig = signal::sine(f0, SR, 48_000, 0.5);
            let (out, _) = shift(&sig, semitones_to_ratio(semi));
            let expected = f0 * semitones_to_ratio(semi);
            let f = detect_tail(&out);
            assert!(
                cents_between(expected, f).abs() < 5.0,
                "{f0} Hz {semi:+} st: esperado {expected}, obtido {f}"
            );
        }
    }

    #[test]
    fn unity_ratio_is_a_pure_delay_of_reported_latency() {
        let sig = signal::sawtooth(150.0, SR, 20_000, 0.5);
        let (out, lat) = shift(&sig, 1.0);
        let mut max_err = 0.0f32;
        for i in (lat + 4000)..sig.len() {
            max_err = max_err.max((out[i] - sig[i - lat]).abs());
        }
        assert!(max_err < 1e-3, "erro máximo {max_err}");
        assert!(out[..lat].iter().all(|v| *v == 0.0));
        // Latência ≤ ~60 ms a 48 kHz com piso de 60 Hz.
        assert!(lat < 3000, "latência {lat}");
    }

    #[test]
    fn varying_ratio_and_invalid_input_stay_finite() {
        let mut s = PsolaShifter::new(PsolaConfig::default(), SR);
        let mut sig = signal::sine(300.0, SR, 9600, 0.5);
        sig[100] = f32::NAN;
        sig[200] = f32::INFINITY;
        let ratios: Vec<f32> = (0..sig.len())
            .map(|i| 1.0 + 0.3 * (i as f32 / 900.0).sin())
            .collect();
        let mut out = vec![0.0; sig.len()];
        s.process_varying(&sig, &mut out, &ratios);
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 2.0));
        let mut out2 = vec![0.0; 64];
        s.process(&sig[..64], &mut out2, f32::NAN);
        assert!(out2.iter().all(|v| v.is_finite()));
        s.reset();
        assert_eq!(s.last_estimate(), PitchEstimate::UNVOICED);
    }
}
