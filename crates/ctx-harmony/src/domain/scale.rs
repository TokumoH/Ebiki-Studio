//! Escalas e modos como **conjuntos de intervalos** e tonalidade (`Key`).
//!
//! Cada modo é uma rotação de uma das três famílias de escala de sete notas (diatônica, menor
//! harmônica, menor melódica). Os intervalos de um modo são **derivados** da família + rotação,
//! exatamente como na view `v_mode_interval` do `library.db`.

use super::error::HarmonyError;
use super::roman::Degree;
use serde::{Deserialize, Serialize};
use shared_kernel::music::{MidiNote, PitchClass};
use std::fmt;

/// Família de escala heptatônica da qual os modos são rotações.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaleFamily {
    Diatonic,
    HarmonicMinor,
    MelodicMinor,
}

impl ScaleFamily {
    pub const ALL: [ScaleFamily; 3] = [Self::Diatonic, Self::HarmonicMinor, Self::MelodicMinor];

    /// Intervalos (em semitons a partir da tônica) da rotação 0 da família.
    pub const fn intervals(self) -> [u8; 7] {
        match self {
            Self::Diatonic => [0, 2, 4, 5, 7, 9, 11],
            Self::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11],
            Self::MelodicMinor => [0, 2, 3, 5, 7, 9, 11],
        }
    }

    /// Código estável (persistência).
    pub const fn code(self) -> &'static str {
        match self {
            Self::Diatonic => "diatonic",
            Self::HarmonicMinor => "harmonic_minor",
            Self::MelodicMinor => "melodic_minor",
        }
    }
}

/// Modo (escala) suportado. Inclui maior/menores, os 7 modos gregos e os modos das menores
/// harmônica e melódica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Ionian,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
    HarmonicMinor,
    LocrianNatural6,
    IonianAugmented,
    DorianSharp4,
    PhrygianDominant,
    LydianSharp2,
    SuperLocrianDoubleFlat7,
    MelodicMinor,
    DorianFlat2,
    LydianAugmented,
    LydianDominant,
    MixolydianFlat6,
    LocrianNatural2,
    Altered,
}

impl Mode {
    /// Escala maior (= jônio).
    pub const MAJOR: Mode = Mode::Ionian;
    /// Menor natural (= eólio).
    pub const NATURAL_MINOR: Mode = Mode::Aeolian;

    pub const ALL: [Mode; 21] = [
        Self::Ionian,
        Self::Dorian,
        Self::Phrygian,
        Self::Lydian,
        Self::Mixolydian,
        Self::Aeolian,
        Self::Locrian,
        Self::HarmonicMinor,
        Self::LocrianNatural6,
        Self::IonianAugmented,
        Self::DorianSharp4,
        Self::PhrygianDominant,
        Self::LydianSharp2,
        Self::SuperLocrianDoubleFlat7,
        Self::MelodicMinor,
        Self::DorianFlat2,
        Self::LydianAugmented,
        Self::LydianDominant,
        Self::MixolydianFlat6,
        Self::LocrianNatural2,
        Self::Altered,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    pub fn family(self) -> ScaleFamily {
        ScaleFamily::ALL[self.index() / 7]
    }

    /// Rotação (0–6) dentro da família.
    pub fn rotation(self) -> u8 {
        (self.index() % 7) as u8
    }

    /// Intervalos do modo derivados de família + rotação.
    pub fn intervals(self) -> [u8; 7] {
        let base = self.family().intervals();
        let r = self.rotation() as usize;
        let mut out = [0u8; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = (base[(i + r) % 7] + 12 - base[r]) % 12;
        }
        out
    }

    /// Terça menor sobre a tônica?
    pub fn is_minor(self) -> bool {
        self.intervals()[2] == 3
    }

    /// Modo paralelo usado para empréstimo modal (maior ↔ menor natural).
    pub fn parallel(self) -> Mode {
        if self.is_minor() {
            Mode::Ionian
        } else {
            Mode::Aeolian
        }
    }

    /// Código estável (persistência).
    pub const fn code(self) -> &'static str {
        match self {
            Self::Ionian => "ionian",
            Self::Dorian => "dorian",
            Self::Phrygian => "phrygian",
            Self::Lydian => "lydian",
            Self::Mixolydian => "mixolydian",
            Self::Aeolian => "aeolian",
            Self::Locrian => "locrian",
            Self::HarmonicMinor => "harmonic_minor",
            Self::LocrianNatural6 => "locrian_natural6",
            Self::IonianAugmented => "ionian_augmented",
            Self::DorianSharp4 => "dorian_sharp4",
            Self::PhrygianDominant => "phrygian_dominant",
            Self::LydianSharp2 => "lydian_sharp2",
            Self::SuperLocrianDoubleFlat7 => "super_locrian_bb7",
            Self::MelodicMinor => "melodic_minor",
            Self::DorianFlat2 => "dorian_flat2",
            Self::LydianAugmented => "lydian_augmented",
            Self::LydianDominant => "lydian_dominant",
            Self::MixolydianFlat6 => "mixolydian_flat6",
            Self::LocrianNatural2 => "locrian_natural2",
            Self::Altered => "altered",
        }
    }

    /// Lê o código estável; aceita também "major" e "minor".
    pub fn from_code(code: &str) -> Result<Mode, HarmonyError> {
        let c = code.trim().to_ascii_lowercase();
        match c.as_str() {
            "major" => return Ok(Mode::Ionian),
            "minor" | "natural_minor" => return Ok(Mode::Aeolian),
            _ => {}
        }
        Self::ALL
            .into_iter()
            .find(|m| m.code() == c)
            .ok_or(HarmonyError::UnknownMode(code.to_owned()))
    }

    /// Nome para a interface (PT-BR).
    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::Ionian => "maior (jônio)",
            Self::Dorian => "dórico",
            Self::Phrygian => "frígio",
            Self::Lydian => "lídio",
            Self::Mixolydian => "mixolídio",
            Self::Aeolian => "menor natural (eólio)",
            Self::Locrian => "lócrio",
            Self::HarmonicMinor => "menor harmônica",
            Self::LocrianNatural6 => "lócrio ♮6",
            Self::IonianAugmented => "jônio aumentado",
            Self::DorianSharp4 => "dórico ♯4",
            Self::PhrygianDominant => "frígio dominante",
            Self::LydianSharp2 => "lídio ♯2",
            Self::SuperLocrianDoubleFlat7 => "superlócrio ♭♭7",
            Self::MelodicMinor => "menor melódica",
            Self::DorianFlat2 => "dórico ♭2",
            Self::LydianAugmented => "lídio aumentado",
            Self::LydianDominant => "lídio dominante",
            Self::MixolydianFlat6 => "mixolídio ♭6",
            Self::LocrianNatural2 => "lócrio ♮2",
            Self::Altered => "alterada (superlócrio)",
        }
    }
}

/// Tonalidade: tônica + modo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    pub tonic: PitchClass,
    pub mode: Mode,
}

/// Tônicas maiores (relativas) que se escrevem com bemóis.
const FLAT_SIGNATURES: [u8; 6] = [5, 10, 3, 8, 1, 6];

impl Key {
    pub const fn new(tonic: PitchClass, mode: Mode) -> Self {
        Self { tonic, mode }
    }

    pub const fn major(tonic: PitchClass) -> Self {
        Self::new(tonic, Mode::Ionian)
    }

    pub const fn minor(tonic: PitchClass) -> Self {
        Self::new(tonic, Mode::Aeolian)
    }

    /// As sete classes de altura da escala, a partir da tônica.
    pub fn scale(&self) -> [PitchClass; 7] {
        let iv = self.mode.intervals();
        let mut out = [self.tonic; 7];
        for (slot, i) in out.iter_mut().zip(iv) {
            *slot = self.tonic.transpose(i as i32);
        }
        out
    }

    /// Classe de altura de um grau (com acidente relativo ao modo).
    pub fn degree_pitch(&self, degree: Degree) -> PitchClass {
        let iv = self.mode.intervals()[(degree.step() - 1) as usize];
        self.tonic.transpose(iv as i32 + degree.accidental() as i32)
    }

    /// Grau natural (1–7) da classe de altura, se pertencer à escala.
    pub fn degree_of(&self, pc: PitchClass) -> Option<u8> {
        self.scale()
            .iter()
            .position(|p| *p == pc)
            .map(|i| i as u8 + 1)
    }

    pub fn contains(&self, pc: PitchClass) -> bool {
        self.degree_of(pc).is_some()
    }

    /// Notas fora da escala (para destacar na Notação). Mantém a ordem de entrada.
    pub fn out_of_scale(&self, notes: &[MidiNote]) -> Vec<MidiNote> {
        notes
            .iter()
            .copied()
            .filter(|n| !self.contains(n.pitch_class()))
            .collect()
    }

    /// Índices (na entrada) das notas fora da escala.
    pub fn out_of_scale_indices(&self, notes: &[MidiNote]) -> Vec<usize> {
        notes
            .iter()
            .enumerate()
            .filter(|(_, n)| !self.contains(n.pitch_class()))
            .map(|(i, _)| i)
            .collect()
    }

    /// A grafia preferida usa bemóis? (pela armadura relativa maior).
    pub fn prefer_flats(&self) -> bool {
        let relative_major = match self.mode.family() {
            ScaleFamily::Diatonic => {
                let iv = ScaleFamily::Diatonic.intervals()[self.mode.rotation() as usize];
                self.tonic.transpose(-(iv as i32))
            }
            fam => {
                let iv = fam.intervals()[self.mode.rotation() as usize];
                self.tonic.transpose(3 - iv as i32)
            }
        };
        FLAT_SIGNATURES.contains(&relative_major.value())
    }

    pub fn transpose(self, semitones: i32) -> Key {
        Key::new(self.tonic.transpose(semitones), self.mode)
    }

    /// Lê "C", "Am", "F#m", "Bb dorian", "A harmonic_minor".
    pub fn parse(s: &str) -> Result<Key, HarmonyError> {
        let err = || HarmonyError::InvalidKey(s.to_owned());
        let mut parts = s.split_whitespace();
        let head = parts.next().ok_or_else(err)?;
        let mode_part = parts.next();
        if parts.next().is_some() {
            return Err(err());
        }
        let (tonic_txt, minor) = match head.strip_suffix('m') {
            Some(t) if !t.is_empty() => (t, true),
            _ => (head, false),
        };
        let tonic = PitchClass::parse(tonic_txt).ok_or_else(err)?;
        let mode = match (mode_part, minor) {
            (Some(_), true) => return Err(err()),
            (Some(code), false) => Mode::from_code(code)?,
            (None, true) => Mode::Aeolian,
            (None, false) => Mode::Ionian,
        };
        Ok(Key::new(tonic, mode))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.tonic.name(self.prefer_flats()),
            self.mode.name_pt()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(s: &str) -> PitchClass {
        PitchClass::parse(s).unwrap()
    }

    #[test]
    fn greek_modes_are_rotations_of_major() {
        assert_eq!(Mode::Ionian.intervals(), [0, 2, 4, 5, 7, 9, 11]);
        assert_eq!(Mode::Dorian.intervals(), [0, 2, 3, 5, 7, 9, 10]);
        assert_eq!(Mode::Phrygian.intervals(), [0, 1, 3, 5, 7, 8, 10]);
        assert_eq!(Mode::Lydian.intervals(), [0, 2, 4, 6, 7, 9, 11]);
        assert_eq!(Mode::Mixolydian.intervals(), [0, 2, 4, 5, 7, 9, 10]);
        assert_eq!(Mode::Aeolian.intervals(), [0, 2, 3, 5, 7, 8, 10]);
        assert_eq!(Mode::Locrian.intervals(), [0, 1, 3, 5, 6, 8, 10]);
    }

    #[test]
    fn minor_family_modes() {
        assert_eq!(Mode::HarmonicMinor.intervals(), [0, 2, 3, 5, 7, 8, 11]);
        assert_eq!(Mode::MelodicMinor.intervals(), [0, 2, 3, 5, 7, 9, 11]);
        assert_eq!(Mode::PhrygianDominant.intervals(), [0, 1, 4, 5, 7, 8, 10]);
        assert_eq!(Mode::LydianDominant.intervals(), [0, 2, 4, 6, 7, 9, 10]);
        assert_eq!(Mode::Altered.intervals(), [0, 1, 3, 4, 6, 8, 10]);
    }

    #[test]
    fn every_mode_has_seven_distinct_ascending_intervals() {
        for m in Mode::ALL {
            let iv = m.intervals();
            assert_eq!(iv[0], 0, "{m:?}");
            assert!(iv.windows(2).all(|w| w[0] < w[1]), "{m:?}");
            assert_eq!(Mode::from_code(m.code()).unwrap(), m);
        }
        assert!(Mode::from_code("xyz").is_err());
        assert_eq!(Mode::from_code("minor").unwrap(), Mode::Aeolian);
    }

    #[test]
    fn key_scale_degrees_and_out_of_scale() {
        let a_minor = Key::minor(pc("A"));
        let names: Vec<_> = a_minor.scale().iter().map(|p| p.name(false)).collect();
        assert_eq!(names, ["A", "B", "C", "D", "E", "F", "G"]);
        assert_eq!(a_minor.degree_of(pc("E")), Some(5));
        assert_eq!(a_minor.degree_of(pc("G#")), None);
        let notes = [MidiNote(69), MidiNote(68), MidiNote(72), MidiNote(66)];
        assert_eq!(
            a_minor.out_of_scale(&notes),
            vec![MidiNote(68), MidiNote(66)]
        );
        assert_eq!(a_minor.out_of_scale_indices(&notes), vec![1, 3]);
        let a_harm = Key::new(pc("A"), Mode::HarmonicMinor);
        assert!(a_harm.out_of_scale(&[MidiNote(68)]).is_empty());
    }

    #[test]
    fn spelling_preference_follows_signature() {
        assert!(!Key::major(pc("C")).prefer_flats());
        assert!(Key::major(pc("F")).prefer_flats());
        assert!(Key::minor(pc("D")).prefer_flats());
        assert!(!Key::minor(pc("E")).prefer_flats());
        assert!(
            Key::new(pc("C"), Mode::Dorian).prefer_flats(),
            "C dórico = Bb maior"
        );
        assert!(Key::new(pc("C"), Mode::HarmonicMinor).prefer_flats());
        assert!(!Key::new(pc("A"), Mode::HarmonicMinor).prefer_flats());
    }

    #[test]
    fn key_parse_and_display() {
        assert_eq!(Key::parse("Am").unwrap(), Key::minor(pc("A")));
        assert_eq!(Key::parse("F#m").unwrap(), Key::minor(pc("F#")));
        assert_eq!(Key::parse("C").unwrap(), Key::major(pc("C")));
        assert_eq!(
            Key::parse("Bb dorian").unwrap(),
            Key::new(pc("Bb"), Mode::Dorian)
        );
        assert!(Key::parse("Am dorian").is_err());
        assert!(Key::parse("H").is_err());
        assert!(Key::parse("").is_err());
        assert_eq!(Key::parse("Bb").unwrap().to_string(), "Bb maior (jônio)");
    }
}
