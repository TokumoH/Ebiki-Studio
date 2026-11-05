//! Correção de afinação em tempo real por escala (estilo "auto-tune").
//!
//! Fluxo por amostra: altura detectada (YIN dentro do [`PsolaShifter`]) → nota permitida mais
//! próxima na escala (com histerese) → correção desejada em cents (reduzida por `humanize`)
//! → suavização exponencial com constante `retune_ms` → razão para o shifter.

use crate::detector::finite_or;
use crate::shifter::{PitchShifter, PsolaConfig, PsolaShifter};
use crate::tuner::{A4_RANGE_HZ, DEFAULT_A4_HZ};
use shared_kernel::music::{MidiNote, PitchClass};

/// Conjunto de classes de altura permitidas (bit `n` = classe `n`, 0 = Dó).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScaleMask(u16);

/// Intervalos da escala maior.
pub const MAJOR: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Intervalos da escala menor natural.
pub const NATURAL_MINOR: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];

impl ScaleMask {
    pub const CHROMATIC: ScaleMask = ScaleMask(0x0FFF);

    /// Máscara a partir de bits (bits acima de 11 são descartados).
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & 0x0FFF)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub fn from_pitch_classes(pcs: impl IntoIterator<Item = PitchClass>) -> Self {
        Self(pcs.into_iter().fold(0, |m, pc| m | (1 << pc.value())))
    }

    /// Escala a partir da tônica (`key`) e dos intervalos (semitons, 0–11).
    pub fn from_key(key: PitchClass, intervals: &[u8]) -> Self {
        Self::from_pitch_classes(intervals.iter().map(|i| key.transpose(*i as i32)))
    }

    pub fn major(key: PitchClass) -> Self {
        Self::from_key(key, &MAJOR)
    }

    pub fn natural_minor(key: PitchClass) -> Self {
        Self::from_key(key, &NATURAL_MINOR)
    }

    pub fn contains(self, pc: PitchClass) -> bool {
        self.0 & (1 << pc.value()) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Nota permitida (inteira, em MIDI) mais próxima de `midi` fracionário.
    pub fn nearest(self, midi: f32) -> Option<i32> {
        if self.is_empty() || !midi.is_finite() {
            return None;
        }
        let center = midi.round() as i32;
        (center - 6..=center + 6)
            .filter(|n| self.0 & (1 << n.rem_euclid(12)) != 0)
            .min_by(|a, b| {
                let da = (*a as f32 - midi).abs();
                let db = (*b as f32 - midi).abs();
                da.total_cmp(&db)
            })
    }
}

/// Ajustes do corretor (valores fora da faixa são limitados).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrectorSettings {
    pub enabled: bool,
    pub scale: ScaleMask,
    /// Constante de tempo da correção (0 = instantânea; típico 10–200 ms).
    pub retune_ms: f32,
    /// 0 = corrige tudo; 1 = preserva todo o desvio (sem correção).
    pub humanize: f32,
    pub a4_hz: f32,
    /// Histerese para trocar de nota-alvo (cents).
    pub hysteresis_cents: f32,
    /// Confiança mínima da detecção para corrigir.
    pub min_confidence: f32,
}

impl Default for CorrectorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            scale: ScaleMask::CHROMATIC,
            retune_ms: 20.0,
            humanize: 0.0,
            a4_hz: DEFAULT_A4_HZ,
            hysteresis_cents: 15.0,
            min_confidence: 0.7,
        }
    }
}

impl CorrectorSettings {
    pub fn sanitized(self) -> Self {
        Self {
            enabled: self.enabled,
            scale: self.scale,
            retune_ms: finite_or(self.retune_ms, 20.0).clamp(0.0, 1000.0),
            humanize: finite_or(self.humanize, 0.0).clamp(0.0, 1.0),
            a4_hz: finite_or(self.a4_hz, DEFAULT_A4_HZ).clamp(A4_RANGE_HZ.0, A4_RANGE_HZ.1),
            hysteresis_cents: finite_or(self.hysteresis_cents, 15.0).clamp(0.0, 50.0),
            min_confidence: finite_or(self.min_confidence, 0.7).clamp(0.0, 1.0),
        }
    }
}

/// Corretor de afinação de tempo real (sem alocação após `new`/`prepare`).
#[derive(Debug, Clone)]
pub struct PitchCorrector {
    shifter: PsolaShifter,
    settings: CorrectorSettings,
    sample_rate: f32,
    coeff: f32,
    smoothed_cents: f32,
    target: Option<i32>,
}

impl PitchCorrector {
    pub fn new(settings: CorrectorSettings, sample_rate: f32) -> Self {
        Self::with_shifter_config(settings, PsolaConfig::default(), sample_rate)
    }

    pub fn with_shifter_config(
        settings: CorrectorSettings,
        shifter: PsolaConfig,
        sample_rate: f32,
    ) -> Self {
        let shifter = PsolaShifter::new(shifter, sample_rate);
        let mut c = Self {
            sample_rate: shifter.sample_rate(),
            shifter,
            settings: settings.sanitized(),
            coeff: 1.0,
            smoothed_cents: 0.0,
            target: None,
        };
        c.update_coeff();
        c
    }

    pub fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        self.shifter.prepare(sample_rate, max_block);
        self.sample_rate = self.shifter.sample_rate();
        self.update_coeff();
        self.reset();
    }

    pub fn reset(&mut self) {
        self.shifter.reset();
        self.smoothed_cents = 0.0;
        self.target = None;
    }

    /// Troca os ajustes em tempo real (sem alocação).
    pub fn set_settings(&mut self, settings: CorrectorSettings) {
        self.settings = settings.sanitized();
        self.update_coeff();
    }

    pub fn settings(&self) -> CorrectorSettings {
        self.settings
    }

    fn update_coeff(&mut self) {
        let samples = self.settings.retune_ms * self.sample_rate / 1000.0;
        self.coeff = if samples <= 1.0 {
            1.0
        } else {
            1.0 - (-1.0 / samples).exp()
        };
    }

    pub fn latency_samples(&self) -> usize {
        self.shifter.latency_samples()
    }

    /// Correção aplicada no momento (cents).
    pub fn current_correction_cents(&self) -> f32 {
        self.smoothed_cents
    }

    /// Nota-alvo atual.
    pub fn target_note(&self) -> Option<MidiNote> {
        self.target
            .filter(|n| (0..=127).contains(n))
            .map(|n| MidiNote(n as u8))
    }

    /// Frequência detectada na entrada (Hz), se vozeada.
    pub fn detected_hz(&self) -> Option<f32> {
        self.shifter
            .last_estimate()
            .pitch(self.settings.min_confidence)
    }

    fn desired_cents(&mut self) -> f32 {
        let s = self.settings;
        let Some(hz) = s.enabled.then(|| self.detected_hz()).flatten() else {
            self.target = None;
            return 0.0;
        };
        let midi = 69.0 + 12.0 * (hz / s.a4_hz).log2();
        let Some(candidate) = s.scale.nearest(midi) else {
            self.target = None;
            return 0.0;
        };
        let keep = self.target.filter(|cur| {
            s.scale.bits() & (1 << cur.rem_euclid(12)) != 0
                && (*cur as f32 - midi).abs()
                    <= (candidate as f32 - midi).abs() + s.hysteresis_cents / 100.0
        });
        let target = keep.unwrap_or(candidate);
        self.target = Some(target);
        (target as f32 - midi) * 100.0 * (1.0 - s.humanize)
    }

    /// Processa um bloco mono (`output.len()` amostras de `input`).
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (x, y) in input.iter().zip(output.iter_mut()) {
            let desired = self.desired_cents();
            self.smoothed_cents += (desired - self.smoothed_cents) * self.coeff;
            let ratio = 2f32.powf(self.smoothed_cents / 1200.0);
            *y = self.shifter.process_sample(*x, ratio);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::{cents_between, PitchDetector, YinConfig, YinDetector};
    use crate::signal;

    const SR: f32 = 48_000.0;

    fn c() -> PitchClass {
        PitchClass::parse("C").unwrap()
    }

    #[test]
    fn scale_mask_nearest_note() {
        let cmaj = ScaleMask::major(c());
        assert_eq!(cmaj.bits(), 0b1010_1011_0101);
        assert!(cmaj.contains(PitchClass::parse("A").unwrap()));
        assert!(!cmaj.contains(PitchClass::parse("A#").unwrap()));
        assert_eq!(cmaj.nearest(69.47), Some(69)); // A sustenido-ish → A
        assert_eq!(cmaj.nearest(70.6), Some(71)); // entre A# e B → B
        assert_eq!(cmaj.nearest(60.8), Some(60)); // C#↓ → C (mais perto que D)
        assert_eq!(cmaj.nearest(61.2), Some(62));
        assert_eq!(ScaleMask::from_bits(0).nearest(60.0), None);
        assert_eq!(ScaleMask::CHROMATIC.nearest(f32::NAN), None);
        let aminor = ScaleMask::natural_minor(PitchClass::parse("A").unwrap());
        assert_eq!(aminor, cmaj, "Lá menor natural = Dó maior");
    }

    fn run(settings: CorrectorSettings, hz: f32, secs: f32) -> (Vec<f32>, PitchCorrector) {
        let mut pc = PitchCorrector::new(settings, SR);
        let sig = signal::sine(hz, SR, (SR * secs) as usize, 0.5);
        let mut out = vec![0.0; sig.len()];
        for (i, block) in sig.chunks(128).enumerate() {
            pc.process(block, &mut out[i * 128..i * 128 + block.len()]);
        }
        (out, pc)
    }

    fn detect_tail(sig: &[f32]) -> f32 {
        let mut yin = YinDetector::new(YinConfig::default(), SR);
        let n = yin.frame_len();
        let s = sig.len() - n - 1000;
        yin.detect(&sig[s..s + n]).frequency_hz
    }

    #[test]
    fn pulls_sharp_a_to_440_in_c_major() {
        let settings = CorrectorSettings {
            scale: ScaleMask::major(c()),
            retune_ms: 0.0,
            ..Default::default()
        };
        let (out, pc) = run(settings, 452.0, 1.0);
        let f = detect_tail(&out);
        assert!(cents_between(440.0, f).abs() < 5.0, "saída {f} Hz");
        assert_eq!(pc.target_note(), Some(MidiNote(69)));
        assert!((pc.current_correction_cents() + 46.58).abs() < 1.0);
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn humanize_and_disable_reduce_correction() {
        let half = CorrectorSettings {
            scale: ScaleMask::major(c()),
            retune_ms: 0.0,
            humanize: 0.5,
            ..Default::default()
        };
        let (out, _) = run(half, 452.0, 1.0);
        let f = detect_tail(&out);
        // Metade do desvio preservada: ~ +23 cents.
        assert!((cents_between(440.0, f) - 23.3).abs() < 5.0, "{f}");

        let off = CorrectorSettings {
            enabled: false,
            ..half
        };
        let (out, _) = run(off, 452.0, 0.5);
        assert!(cents_between(452.0, detect_tail(&out)).abs() < 5.0);
    }

    #[test]
    fn retune_speed_smooths_the_correction() {
        let slow = CorrectorSettings {
            scale: ScaleMask::major(c()),
            retune_ms: 200.0,
            ..Default::default()
        };
        let mut pc = PitchCorrector::new(slow, SR);
        let sig = signal::sine(452.0, SR, 9600, 0.5); // 200 ms
        let mut out = vec![0.0; sig.len()];
        pc.process(&sig, &mut out);
        let partial = pc.current_correction_cents();
        assert!(
            partial < -5.0 && partial > -40.0,
            "após ~1 constante de tempo, correção parcial: {partial}"
        );
    }

    #[test]
    fn settings_are_sanitized() {
        let s = CorrectorSettings {
            retune_ms: f32::NAN,
            humanize: 7.0,
            a4_hz: 1000.0,
            hysteresis_cents: -1.0,
            min_confidence: f32::INFINITY,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.retune_ms, 20.0);
        assert_eq!(s.humanize, 1.0);
        assert_eq!(s.a4_hz, 466.0);
        assert_eq!(s.hysteresis_cents, 0.0);
        assert_eq!(s.min_confidence, 0.7);
    }
}
