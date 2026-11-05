//! Alturas grafadas (nome + acidente + oitava) e armaduras de clave com grafia enarmônica.

use crate::error::NotationError;
use serde::{Deserialize, Serialize};
use shared_kernel::music::{MidiNote, PitchClass};

/// Nome de nota (grau diatônico natural).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Step {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Step {
    pub const ALL: [Step; 7] = [
        Step::C,
        Step::D,
        Step::E,
        Step::F,
        Step::G,
        Step::A,
        Step::B,
    ];

    /// Semitons acima de Dó.
    pub const fn natural_semitone(self) -> i32 {
        match self {
            Self::C => 0,
            Self::D => 2,
            Self::E => 4,
            Self::F => 5,
            Self::G => 7,
            Self::A => 9,
            Self::B => 11,
        }
    }

    pub const fn letter(self) -> char {
        match self {
            Self::C => 'C',
            Self::D => 'D',
            Self::E => 'E',
            Self::F => 'F',
            Self::G => 'G',
            Self::A => 'A',
            Self::B => 'B',
        }
    }

    pub fn from_letter(c: char) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|s| s.letter() == c.to_ascii_uppercase())
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Altura grafada. `alter` em semitons (−2 = dobrado bemol … +2 = dobrado sustenido).
/// Oitava científica (C4 = Dó central = MIDI 60). Representa a altura real (concerto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pitch {
    pub step: Step,
    #[serde(default)]
    pub alter: i8,
    pub octave: i8,
}

impl Pitch {
    pub fn new(step: Step, alter: i8, octave: i8) -> Result<Self, NotationError> {
        if !(-2..=2).contains(&alter) {
            return Err(NotationError::InvalidPitch("acidente fora de −2…+2"));
        }
        let p = Self {
            step,
            alter,
            octave,
        };
        let m = p.midi();
        if !(0..=127).contains(&m) {
            return Err(NotationError::PitchOutOfRange(m));
        }
        Ok(p)
    }

    /// Número MIDI (pode sair de 0–127 se construído sem validação).
    pub fn midi(&self) -> i32 {
        (self.octave as i32 + 1) * 12 + self.step.natural_semitone() + self.alter as i32
    }

    pub fn midi_note(&self) -> Option<MidiNote> {
        let m = self.midi();
        (0..=127).contains(&m).then_some(MidiNote(m as u8))
    }

    pub fn pitch_class(&self) -> PitchClass {
        PitchClass::new(self.midi().rem_euclid(12) as u8).expect("rem_euclid(12) < 12")
    }

    /// Grafa um número MIDI segundo a armadura (diatônicas pela armadura; cromáticas pela direção da
    /// armadura: sustenidos em tons de sustenidos/Dó, bemóis em tons de bemóis).
    pub fn from_midi(midi: u8, key: &KeySignature) -> Self {
        let pc = (midi % 12) as i32;
        let (step, alter) = key.spell(pc);
        let octave = (midi as i32 - step.natural_semitone() - alter as i32).div_euclid(12) - 1;
        Self {
            step,
            alter,
            octave: octave as i8,
        }
    }

    /// Nome legível, ex.: "C#4", "Bb3".
    pub fn name(&self) -> String {
        let acc = match self.alter {
            -2 => "bb",
            -1 => "b",
            1 => "#",
            2 => "##",
            _ => "",
        };
        format!("{}{}{}", self.step.letter(), acc, self.octave)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Major,
    Minor,
}

/// Armadura: `fifths` > 0 sustenidos, < 0 bemóis (−7…+7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeySignature {
    pub fifths: i8,
    #[serde(default)]
    pub mode: Mode,
}

const SHARP_ORDER: [Step; 7] = [
    Step::F,
    Step::C,
    Step::G,
    Step::D,
    Step::A,
    Step::E,
    Step::B,
];

impl KeySignature {
    pub fn new(fifths: i8, mode: Mode) -> Result<Self, NotationError> {
        if !(-7..=7).contains(&fifths) {
            return Err(NotationError::InvalidKey(fifths));
        }
        Ok(Self { fifths, mode })
    }

    pub const fn c_major() -> Self {
        Self {
            fifths: 0,
            mode: Mode::Major,
        }
    }

    /// Acidente que a armadura aplica a um nome de nota.
    pub fn step_alter(&self, step: Step) -> i8 {
        let n = self.fifths.unsigned_abs().min(7) as usize;
        if self.fifths > 0 && SHARP_ORDER[..n].contains(&step) {
            1
        } else if self.fifths < 0 && SHARP_ORDER[7 - n..].contains(&step) {
            -1
        } else {
            0
        }
    }

    /// Tônica (maior: ciclo de quintas a partir de Dó; menor: relativa, 3 semitons abaixo).
    pub fn tonic(&self) -> PitchClass {
        let major = (7 * self.fifths as i32).rem_euclid(12) as u8;
        let pc = PitchClass::new(major).expect("< 12");
        match self.mode {
            Mode::Major => pc,
            Mode::Minor => pc.transpose(9),
        }
    }

    /// As 7 classes de altura diatônicas da armadura.
    pub fn diatonic_pitch_classes(&self) -> Vec<PitchClass> {
        Step::ALL
            .iter()
            .map(|s| {
                PitchClass::new(
                    (s.natural_semitone() + self.step_alter(*s) as i32).rem_euclid(12) as u8,
                )
                .expect("< 12")
            })
            .collect()
    }

    /// Escolhe (nome, acidente) para uma classe de altura.
    pub fn spell(&self, pc: i32) -> (Step, i8) {
        let pc = pc.rem_euclid(12);
        // 1) diatônica
        for s in Step::ALL {
            let a = self.step_alter(s);
            if (s.natural_semitone() + a as i32).rem_euclid(12) == pc {
                return (s, a);
            }
        }
        // 2) cromática: grau elevado (tons de sustenido/Dó) ou abaixado (tons de bemol)
        let dir: i8 = if self.fifths >= 0 { 1 } else { -1 };
        for wanted in [dir, -dir] {
            for s in Step::ALL {
                let a = self.step_alter(s) + wanted;
                if (-2..=2).contains(&a) && (s.natural_semitone() + a as i32).rem_euclid(12) == pc {
                    return (s, a);
                }
            }
        }
        // 3) fallback (nunca alcançado na prática)
        let natural = Step::ALL
            .into_iter()
            .rev()
            .find(|s| s.natural_semitone() <= pc)
            .unwrap_or(Step::C);
        (natural, (pc - natural.natural_semitone()) as i8)
    }

    /// Armadura de destino ao transpor `semitones`: mesma tônica deslocada, com menos acidentes
    /// (empate F#/Gb resolvido pelo sinal da armadura original).
    pub fn transposed(&self, semitones: i32) -> Self {
        let target = self.tonic().transpose(semitones);
        let mut best: Option<i8> = None;
        for f in -7i8..=7 {
            let k = Self {
                fifths: f,
                mode: self.mode,
            };
            if k.tonic() != target {
                continue;
            }
            best = match best {
                None => Some(f),
                Some(b) if f.abs() < b.abs() => Some(f),
                Some(b) if f.abs() == b.abs() && (f < 0) == (self.fifths < 0) => Some(f),
                Some(b) => Some(b),
            };
        }
        Self {
            fifths: best.unwrap_or(0),
            mode: self.mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn midi_of_pitches() {
        assert_eq!(Pitch::new(Step::C, 0, 4).unwrap().midi(), 60);
        assert_eq!(Pitch::new(Step::B, 1, 3).unwrap().midi(), 60);
        assert_eq!(Pitch::new(Step::A, 0, 4).unwrap().midi(), 69);
        assert!(Pitch::new(Step::C, 3, 4).is_err());
        assert!(Pitch::new(Step::G, 0, 10).is_err());
        assert_eq!(Pitch::new(Step::B, -1, 3).unwrap().name(), "Bb3");
    }

    #[test]
    fn key_signature_alters_and_tonic() {
        let d = KeySignature::new(2, Mode::Major).unwrap();
        assert_eq!(d.step_alter(Step::F), 1);
        assert_eq!(d.step_alter(Step::C), 1);
        assert_eq!(d.step_alter(Step::G), 0);
        assert_eq!(d.tonic().value(), 2);
        let bb = KeySignature::new(-2, Mode::Major).unwrap();
        assert_eq!(bb.step_alter(Step::B), -1);
        assert_eq!(bb.step_alter(Step::E), -1);
        assert_eq!(bb.step_alter(Step::A), 0);
        assert_eq!(bb.tonic().value(), 10);
        let am = KeySignature::new(0, Mode::Minor).unwrap();
        assert_eq!(am.tonic().value(), 9);
        assert!(KeySignature::new(8, Mode::Major).is_err());
    }

    #[test]
    fn spelling_follows_key() {
        let eb = KeySignature::new(-3, Mode::Major).unwrap();
        assert_eq!(Pitch::from_midi(63, &eb).name(), "Eb4");
        assert_eq!(Pitch::from_midi(66, &eb).name(), "Gb4");
        let e = KeySignature::new(4, Mode::Major).unwrap();
        assert_eq!(Pitch::from_midi(63, &e).name(), "D#4");
        let cs = KeySignature::new(7, Mode::Major).unwrap();
        assert_eq!(Pitch::from_midi(60, &cs).name(), "B#3");
        assert_eq!(Pitch::from_midi(60, &cs).midi(), 60);
        let cb = KeySignature::new(-7, Mode::Major).unwrap();
        assert_eq!(Pitch::from_midi(59, &cb).name(), "Cb4");
        assert_eq!(Pitch::from_midi(59, &cb).midi(), 59);
    }

    #[test]
    fn transposed_key_prefers_fewest_accidentals() {
        let c = KeySignature::c_major();
        assert_eq!(c.transposed(2).fifths, 2); // D
        assert_eq!(c.transposed(1).fifths, -5); // Db (não C#)
        assert_eq!(c.transposed(3).fifths, -3); // Eb
        let am = KeySignature::new(0, Mode::Minor).unwrap();
        assert_eq!(am.transposed(2).fifths, 2); // Bm
    }

    proptest! {
        #[test]
        fn spelling_preserves_midi(midi in 0u8..=127, fifths in -7i8..=7, minor in any::<bool>()) {
            let k = KeySignature::new(fifths, if minor { Mode::Minor } else { Mode::Major }).unwrap();
            let p = Pitch::from_midi(midi, &k);
            prop_assert_eq!(p.midi(), midi as i32);
            prop_assert!((-2..=2).contains(&p.alter));
        }

        #[test]
        fn diatonic_notes_use_key_accidentals(fifths in -7i8..=7, idx in 0usize..7, oct in 1i8..7) {
            let k = KeySignature::new(fifths, Mode::Major).unwrap();
            let step = Step::ALL[idx];
            let written = Pitch { step, alter: k.step_alter(step), octave: oct };
            let m = written.midi();
            prop_assume!((0..=127).contains(&m));
            prop_assert_eq!(Pitch::from_midi(m as u8, &k), written);
        }
    }
}
