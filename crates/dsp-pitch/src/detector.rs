//! Detecção de altura (porta `PitchDetector`, ADR-0011) e implementação YIN.
//!
//! YIN (de Cheveigné & Kawahara, 2002): função diferença, diferença normalizada pela média
//! cumulativa (CMND), limiar absoluto, busca do mínimo local e interpolação parabólica.
//! Escolhido por ser robusto para voz e cordas (60–1500 Hz), sem FFT e com custo previsível.

/// Estimativa de altura de um quadro de análise.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PitchEstimate {
    /// Frequência fundamental estimada em Hz (0 quando não há sinal).
    pub frequency_hz: f32,
    /// Confiança 0–1 (1 − CMND no período escolhido).
    pub confidence: f32,
    /// `true` quando o limiar absoluto do detector foi atingido (quadro periódico/vozeado).
    pub voiced: bool,
}

impl PitchEstimate {
    pub const UNVOICED: PitchEstimate = PitchEstimate {
        frequency_hz: 0.0,
        confidence: 0.0,
        voiced: false,
    };

    /// Frequência apenas se o quadro é vozeado e a confiança alcança `min_confidence`.
    pub fn pitch(&self, min_confidence: f32) -> Option<f32> {
        (self.voiced && self.confidence >= min_confidence && self.frequency_hz > 0.0)
            .then_some(self.frequency_hz)
    }
}

/// Porta de detecção de altura (AI-ready: um modelo neural pode implementar a mesma trait).
///
/// Contrato: `prepare` aloca; `detect` não aloca, não bloqueia e não entra em pânico.
pub trait PitchDetector: Send {
    /// Prepara buffers para a taxa de amostragem.
    fn prepare(&mut self, sample_rate: f32);
    /// Número de amostras que um quadro de análise precisa (`detect` usa as primeiras).
    fn frame_len(&self) -> usize;
    /// Estima a altura do quadro. Quadro mais curto que `frame_len` → não vozeado.
    fn detect(&mut self, frame: &[f32]) -> PitchEstimate;
}

/// Parâmetros do YIN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YinConfig {
    /// Menor frequência detectável (Hz).
    pub min_hz: f32,
    /// Maior frequência detectável (Hz).
    pub max_hz: f32,
    /// Limiar absoluto da CMND (típico 0,10–0,20).
    pub threshold: f32,
    /// RMS abaixo do qual o quadro é silêncio.
    pub silence_rms: f32,
}

impl Default for YinConfig {
    fn default() -> Self {
        Self {
            min_hz: 60.0,
            max_hz: 1500.0,
            threshold: 0.15,
            silence_rms: 1e-4,
        }
    }
}

impl YinConfig {
    fn sanitized(self) -> Self {
        let min_hz = finite_or(self.min_hz, 60.0).clamp(20.0, 2000.0);
        let max_hz = finite_or(self.max_hz, 1500.0).clamp(min_hz * 1.5, 5000.0);
        Self {
            min_hz,
            max_hz,
            threshold: finite_or(self.threshold, 0.15).clamp(0.01, 0.9),
            silence_rms: finite_or(self.silence_rms, 1e-4).clamp(0.0, 1.0),
        }
    }
}

pub(crate) fn finite_or(v: f32, default: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        default
    }
}

/// Detector YIN sem alocação após `prepare`.
#[derive(Debug, Clone)]
pub struct YinDetector {
    config: YinConfig,
    sample_rate: f32,
    tau_min: usize,
    tau_max: usize,
    window: usize,
    diff: Vec<f32>,
    cmnd: Vec<f32>,
}

impl YinDetector {
    /// Cria e prepara o detector.
    pub fn new(config: YinConfig, sample_rate: f32) -> Self {
        let mut d = Self {
            config: config.sanitized(),
            sample_rate: 0.0,
            tau_min: 2,
            tau_max: 2,
            window: 2,
            diff: Vec::new(),
            cmnd: Vec::new(),
        };
        d.prepare(sample_rate);
        d
    }

    pub fn config(&self) -> YinConfig {
        self.config
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}

impl PitchDetector for YinDetector {
    fn prepare(&mut self, sample_rate: f32) {
        let sr = finite_or(sample_rate, 48_000.0).clamp(8_000.0, 384_000.0);
        self.sample_rate = sr;
        self.tau_max = (sr / self.config.min_hz).ceil() as usize + 2;
        self.tau_min = ((sr / self.config.max_hz).floor() as usize).max(2);
        self.window = self.tau_max;
        self.diff = vec![0.0; self.tau_max + 2];
        self.cmnd = vec![0.0; self.tau_max + 2];
    }

    fn frame_len(&self) -> usize {
        self.window + self.tau_max + 1
    }

    fn detect(&mut self, frame: &[f32]) -> PitchEstimate {
        if frame.len() < self.frame_len() {
            return PitchEstimate::UNVOICED;
        }
        let w = self.window;
        let x = &frame[..self.frame_len()];

        // Silêncio / sinal inválido.
        let mut energy = 0.0f64;
        for &s in &x[..w] {
            if !s.is_finite() {
                return PitchEstimate::UNVOICED;
            }
            energy += (s as f64) * (s as f64);
        }
        let rms = (energy / w as f64).sqrt() as f32;
        if rms <= self.config.silence_rms {
            return PitchEstimate::UNVOICED;
        }

        // Passo 2: função diferença.
        self.diff[0] = 0.0;
        for tau in 1..=self.tau_max {
            let mut acc = 0.0f32;
            let (a, b) = (&x[..w], &x[tau..tau + w]);
            for j in 0..w {
                let d = a[j] - b[j];
                acc += d * d;
            }
            self.diff[tau] = acc;
        }

        // Passo 3: diferença normalizada pela média cumulativa.
        self.cmnd[0] = 1.0;
        let mut running = 0.0f32;
        for tau in 1..=self.tau_max {
            running += self.diff[tau];
            self.cmnd[tau] = if running > 0.0 {
                self.diff[tau] * tau as f32 / running
            } else {
                1.0
            };
        }

        // Passo 4: limiar absoluto + mínimo local; senão, mínimo global.
        let upper = self.tau_max - 1;
        let mut chosen = None;
        let mut tau = self.tau_min;
        while tau <= upper {
            if self.cmnd[tau] < self.config.threshold {
                while tau < upper && self.cmnd[tau + 1] < self.cmnd[tau] {
                    tau += 1;
                }
                chosen = Some(tau);
                break;
            }
            tau += 1;
        }
        let voiced = chosen.is_some();
        let tau = chosen.unwrap_or_else(|| {
            let mut best = self.tau_min;
            for t in self.tau_min..=upper {
                if self.cmnd[t] < self.cmnd[best] {
                    best = t;
                }
            }
            best
        });

        // Passo 5: interpolação parabólica sobre a função diferença.
        let refined = if tau > 1 && tau < self.tau_max {
            let (s0, s1, s2) = (self.diff[tau - 1], self.diff[tau], self.diff[tau + 1]);
            let denom = s0 - 2.0 * s1 + s2;
            if denom.abs() > f32::EPSILON {
                tau as f32 + (0.5 * (s0 - s2) / denom).clamp(-1.0, 1.0)
            } else {
                tau as f32
            }
        } else {
            tau as f32
        };

        let confidence = (1.0 - self.cmnd[tau]).clamp(0.0, 1.0);
        PitchEstimate {
            frequency_hz: self.sample_rate / refined,
            confidence,
            voiced,
        }
    }
}

/// Erro em cents entre duas frequências.
pub fn cents_between(reference_hz: f32, hz: f32) -> f32 {
    1200.0 * (hz / reference_hz).log2()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal;

    const SR: f32 = 48_000.0;

    fn detect_mid(sig: &[f32]) -> PitchEstimate {
        let mut yin = YinDetector::new(YinConfig::default(), SR);
        let n = yin.frame_len();
        let start = sig.len() / 2 - n / 2;
        yin.detect(&sig[start..start + n])
    }

    #[test]
    fn detects_sines_within_5_cents() {
        for f in [82.41f32, 110.0, 220.0, 440.0, 1000.0, 1400.0] {
            let est = detect_mid(&signal::sine(f, SR, 8192, 0.8));
            assert!(est.voiced, "{f} Hz deveria ser vozeado");
            let err = cents_between(f, est.frequency_hz).abs();
            assert!(
                err < 5.0,
                "{f} Hz: estimado {} ({err} cents)",
                est.frequency_hz
            );
            assert!(est.confidence > 0.9);
        }
    }

    #[test]
    fn detects_sawtooth_fundamental() {
        for f in [98.0f32, 196.0, 330.0] {
            let est = detect_mid(&signal::sawtooth(f, SR, 8192, 0.5));
            let err = cents_between(f, est.frequency_hz).abs();
            assert!(err < 5.0, "dente de serra {f} Hz → {}", est.frequency_hz);
        }
    }

    #[test]
    fn noise_has_low_confidence_and_silence_is_unvoiced() {
        let est = detect_mid(&signal::noise(8192, 0.5, 1234));
        assert!(!est.voiced);
        assert!(est.confidence < 0.5, "confiança {}", est.confidence);
        assert_eq!(detect_mid(&vec![0.0; 8192]), PitchEstimate::UNVOICED);
        let mut nan = signal::sine(220.0, SR, 8192, 0.5);
        nan[4100] = f32::NAN;
        assert!(!detect_mid(&nan).voiced);
    }

    #[test]
    fn short_frame_is_unvoiced_and_config_is_sanitized() {
        let mut yin = YinDetector::new(
            YinConfig {
                min_hz: f32::NAN,
                max_hz: -3.0,
                threshold: 9.0,
                silence_rms: f32::INFINITY,
            },
            f32::NAN,
        );
        assert!(yin.config().min_hz >= 20.0);
        assert!(yin.config().max_hz > yin.config().min_hz);
        assert!(yin.config().threshold <= 0.9);
        assert_eq!(yin.detect(&[0.1; 10]), PitchEstimate::UNVOICED);
        assert_eq!(
            PitchEstimate {
                frequency_hz: 100.0,
                confidence: 0.4,
                voiced: true
            }
            .pitch(0.5),
            None
        );
    }
}
