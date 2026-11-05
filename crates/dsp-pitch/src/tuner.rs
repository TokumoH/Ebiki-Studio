//! Afinador: frequência → nota mais próxima + desvio em cents, e processador em tempo real.

use crate::detector::{finite_or, PitchDetector, PitchEstimate, YinConfig, YinDetector};
use shared_kernel::music::{MidiNote, PitchClass};

/// Referência padrão do Lá4.
pub const DEFAULT_A4_HZ: f32 = 440.0;
/// Faixa aceita para a referência do Lá4 (barroco 415 Hz até 466 Hz).
pub const A4_RANGE_HZ: (f32, f32) = (415.0, 466.0);

/// Leitura do afinador.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TunerReading {
    pub frequency_hz: f32,
    /// Nota mais próxima.
    pub note: MidiNote,
    /// Desvio em cents em relação à nota mais próxima (−50 … +50).
    pub cents: f32,
    pub confidence: f32,
}

impl TunerReading {
    pub fn pitch_class(&self) -> PitchClass {
        self.note.pitch_class()
    }

    /// Nome da nota com oitava, ex.: `"A4"`, `"C#3"` (sem alocação: nome + oitava separados).
    pub fn name(&self, prefer_flats: bool) -> (&'static str, i32) {
        (self.pitch_class().name(prefer_flats), self.note.octave())
    }

    /// Dentro da tolerância (cents) para considerar "afinado".
    pub fn in_tune(&self, tolerance_cents: f32) -> bool {
        self.cents.abs() <= tolerance_cents
    }
}

/// Conversor frequência → nota com referência configurável.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuner {
    a4_hz: f32,
}

impl Default for Tuner {
    fn default() -> Self {
        Self {
            a4_hz: DEFAULT_A4_HZ,
        }
    }
}

impl Tuner {
    /// Cria com referência `a4_hz` limitada a [`A4_RANGE_HZ`] (NaN → 440).
    pub fn new(a4_hz: f32) -> Self {
        Self {
            a4_hz: finite_or(a4_hz, DEFAULT_A4_HZ).clamp(A4_RANGE_HZ.0, A4_RANGE_HZ.1),
        }
    }

    pub fn a4_hz(&self) -> f32 {
        self.a4_hz
    }

    /// Nota MIDI fracionária da frequência.
    pub fn midi_float(&self, hz: f32) -> Option<f32> {
        MidiNote::from_frequency(hz as f64, self.a4_hz as f64).map(|m| m as f32)
    }

    /// Lê uma frequência. `None` para frequência inválida ou fora da faixa MIDI.
    pub fn read(&self, hz: f32) -> Option<TunerReading> {
        self.read_with_confidence(hz, 1.0)
    }

    fn read_with_confidence(&self, hz: f32, confidence: f32) -> Option<TunerReading> {
        let m = self.midi_float(hz)?;
        let nearest = m.round();
        if !(0.0..=127.0).contains(&nearest) {
            return None;
        }
        Some(TunerReading {
            frequency_hz: hz,
            note: MidiNote(nearest as u8),
            cents: (m - nearest) * 100.0,
            confidence,
        })
    }

    /// Frequência de uma nota nesta referência.
    pub fn frequency(&self, note: MidiNote) -> f32 {
        note.frequency(self.a4_hz as f64) as f32
    }
}

/// Afinador de tempo real: recebe blocos, analisa a cada `hop` amostras.
///
/// Sem alocação em [`TunerProcessor::process`].
#[derive(Debug, Clone)]
pub struct TunerProcessor {
    tuner: Tuner,
    detector: YinDetector,
    ring: Vec<f32>,
    frame: Vec<f32>,
    write: usize,
    filled: usize,
    hop: usize,
    since_last: usize,
    min_confidence: f32,
    last: Option<TunerReading>,
    last_estimate: PitchEstimate,
}

impl TunerProcessor {
    /// `hop` em amostras (0 → ~10 ms).
    pub fn new(sample_rate: f32, a4_hz: f32, config: YinConfig, hop: usize) -> Self {
        let detector = YinDetector::new(config, sample_rate);
        let n = detector.frame_len();
        let hop = if hop == 0 {
            (detector.sample_rate() / 100.0) as usize
        } else {
            hop
        };
        Self {
            tuner: Tuner::new(a4_hz),
            detector,
            ring: vec![0.0; n],
            frame: vec![0.0; n],
            write: 0,
            filled: 0,
            hop: hop.max(1),
            since_last: 0,
            min_confidence: 0.8,
            last: None,
            last_estimate: PitchEstimate::UNVOICED,
        }
    }

    /// Troca a referência (sem alocação).
    pub fn set_a4(&mut self, a4_hz: f32) {
        self.tuner = Tuner::new(a4_hz);
    }

    pub fn tuner(&self) -> Tuner {
        self.tuner
    }

    /// Latência da leitura em amostras (tamanho do quadro de análise).
    pub fn latency_samples(&self) -> usize {
        self.ring.len()
    }

    pub fn reset(&mut self) {
        self.ring.fill(0.0);
        self.write = 0;
        self.filled = 0;
        self.since_last = 0;
        self.last = None;
        self.last_estimate = PitchEstimate::UNVOICED;
    }

    /// Consome um bloco de entrada (mono).
    pub fn process(&mut self, input: &[f32]) {
        let n = self.ring.len();
        for &x in input {
            self.ring[self.write] = if x.is_finite() { x } else { 0.0 };
            self.write = (self.write + 1) % n;
            self.filled = (self.filled + 1).min(n);
            self.since_last += 1;
            if self.since_last >= self.hop && self.filled == n {
                self.since_last = 0;
                let (tail, head) = self.ring.split_at(self.write);
                self.frame[..head.len()].copy_from_slice(head);
                self.frame[head.len()..].copy_from_slice(tail);
                let est = self.detector.detect(&self.frame);
                self.last_estimate = est;
                self.last = est
                    .pitch(self.min_confidence)
                    .and_then(|hz| self.tuner.read_with_confidence(hz, est.confidence));
            }
        }
    }

    /// Última leitura vozeada (ou `None` em silêncio/ruído).
    pub fn reading(&self) -> Option<TunerReading> {
        self.last
    }

    pub fn last_estimate(&self) -> PitchEstimate {
        self.last_estimate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal;

    #[test]
    fn reads_nearest_note_and_cents() {
        let t = Tuner::default();
        let r = t.read(440.0).unwrap();
        assert_eq!(r.note, MidiNote(69));
        assert!(r.cents.abs() < 1e-3);
        assert_eq!(r.name(false), ("A", 4));
        // 452 Hz ≈ +46,58 cents acima de A4.
        let r = t.read(452.0).unwrap();
        assert_eq!(r.note, MidiNote(69));
        assert!((r.cents - 46.58).abs() < 0.05, "{}", r.cents);
        // 10 cents abaixo de E2 (82,407 Hz).
        let e2 = 82.4069 * 2f32.powf(-10.0 / 1200.0);
        let r = t.read(e2).unwrap();
        assert_eq!(r.note, MidiNote(40));
        assert!((r.cents + 10.0).abs() < 0.05);
        assert!(!r.in_tune(5.0) && r.in_tune(10.5));
        assert!(t.read(0.0).is_none() && t.read(f32::NAN).is_none());
        assert!(t.read(1e9).is_none());
    }

    #[test]
    fn reference_is_configurable_and_clamped() {
        let t = Tuner::new(432.0);
        let r = t.read(432.0).unwrap();
        assert_eq!(r.note, MidiNote(69));
        assert!(r.cents.abs() < 1e-3);
        // Com A4 = 432, 440 Hz está ~ +31,8 cents.
        assert!((Tuner::new(432.0).read(440.0).unwrap().cents - 31.77).abs() < 0.05);
        assert_eq!(Tuner::new(300.0).a4_hz(), 415.0);
        assert_eq!(Tuner::new(f32::NAN).a4_hz(), 440.0);
        assert!((Tuner::new(415.0).frequency(MidiNote(69)) - 415.0).abs() < 1e-3);
    }

    #[test]
    fn processor_tracks_a_live_signal() {
        let sr = 48_000.0;
        let mut p = TunerProcessor::new(sr, 440.0, YinConfig::default(), 0);
        let sig = signal::sine(329.63 * 2f32.powf(7.0 / 1200.0), sr, 24_000, 0.5); // E4 +7 cents
        for block in sig.chunks(256) {
            p.process(block);
        }
        let r = p.reading().expect("leitura vozeada");
        assert_eq!(r.note, MidiNote(64));
        assert!((r.cents - 7.0).abs() < 2.0, "{}", r.cents);
        p.process(&vec![0.0; 4000]);
        assert!(p.reading().is_none(), "silêncio limpa a leitura");
        p.reset();
        assert!(p.reading().is_none());
    }
}
