//! Primitivas de teoria musical compartilhadas (Harmony, Notation, Tuning).

use serde::{Deserialize, Serialize};

/// Classe de altura (0 = Dó … 11 = Si).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct PitchClass(u8);

const SHARP_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const FLAT_NAMES: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];

impl PitchClass {
    pub fn new(v: u8) -> Option<Self> {
        (v < 12).then_some(Self(v))
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    /// Transpõe por um número de semitons (pode ser negativo).
    pub fn transpose(self, semitones: i32) -> Self {
        Self((self.0 as i32 + semitones).rem_euclid(12) as u8)
    }

    /// Distância ascendente em semitons até `other` (0–11).
    pub fn interval_to(self, other: PitchClass) -> u8 {
        ((other.0 as i32 - self.0 as i32).rem_euclid(12)) as u8
    }

    pub fn name(self, prefer_flats: bool) -> &'static str {
        if prefer_flats {
            FLAT_NAMES[self.0 as usize]
        } else {
            SHARP_NAMES[self.0 as usize]
        }
    }

    /// Lê "C", "C#", "Db", "Bb", "E#"… (case-insensitive na letra).
    pub fn parse(s: &str) -> Option<Self> {
        let mut chars = s.trim().chars();
        let letter = chars.next()?.to_ascii_uppercase();
        let base: i32 = match letter {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let mut offset = 0;
        for c in chars {
            match c {
                '#' | '♯' => offset += 1,
                'b' | '♭' => offset -= 1,
                _ => return None,
            }
        }
        Some(Self((base + offset).rem_euclid(12) as u8))
    }
}

impl TryFrom<u8> for PitchClass {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, String> {
        PitchClass::new(v).ok_or_else(|| format!("classe de altura inválida: {v}"))
    }
}

impl From<PitchClass> for u8 {
    fn from(p: PitchClass) -> u8 {
        p.0
    }
}

/// Nota MIDI (0–127). Dó central = 60.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MidiNote(pub u8);

impl MidiNote {
    pub fn pitch_class(self) -> PitchClass {
        PitchClass(self.0 % 12)
    }

    /// Oitava científica (C4 = 60).
    pub fn octave(self) -> i32 {
        self.0 as i32 / 12 - 1
    }

    /// Frequência em Hz com A4 = `a4_hz`.
    pub fn frequency(self, a4_hz: f64) -> f64 {
        a4_hz * 2f64.powf((self.0 as f64 - 69.0) / 12.0)
    }

    /// Nota fracionária (MIDI) a partir de frequência.
    pub fn from_frequency(hz: f64, a4_hz: f64) -> Option<f64> {
        (hz > 0.0 && hz.is_finite()).then(|| 69.0 + 12.0 * (hz / a4_hz).log2())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_class_parse_and_transpose() {
        assert_eq!(PitchClass::parse("Bb").unwrap().value(), 10);
        assert_eq!(PitchClass::parse("c#").unwrap().value(), 1);
        assert_eq!(PitchClass::parse("Cb").unwrap().value(), 11);
        assert!(PitchClass::parse("H").is_none());
        assert!(PitchClass::parse("C#x").is_none());
        let c = PitchClass::parse("C").unwrap();
        assert_eq!(c.transpose(-1).value(), 11);
        assert_eq!(c.transpose(14).value(), 2);
        assert_eq!(c.interval_to(PitchClass::parse("G").unwrap()), 7);
        assert_eq!(PitchClass::parse("G").unwrap().interval_to(c), 5);
    }

    #[test]
    fn midi_frequency() {
        assert!((MidiNote(69).frequency(440.0) - 440.0).abs() < 1e-9);
        assert!((MidiNote(60).frequency(440.0) - 261.6256).abs() < 1e-3);
        assert_eq!(MidiNote(60).octave(), 4);
        assert!((MidiNote::from_frequency(880.0, 440.0).unwrap() - 81.0).abs() < 1e-9);
        assert!(MidiNote::from_frequency(0.0, 440.0).is_none());
    }
}
