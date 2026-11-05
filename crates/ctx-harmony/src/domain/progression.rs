//! Progressões harmônicas em **graus relativos** + ritmo harmônico (duração em tempos).

use super::chord::Chord;
use super::error::{validate_name, HarmonyError};
use super::roman::RomanChord;
use super::scale::{Key, Mode};
use serde::{Deserialize, Serialize};
use shared_kernel::define_id;
use std::collections::BTreeSet;

define_id!(
    /// Identificador de progressão cadastrada pelo usuário.
    ProgressionId
);

pub const MAX_STEPS: usize = 64;
pub const MAX_STEP_BEATS: u16 = 64;

/// Gênero musical (vocabulário compartilhado com a Biblioteca por código).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Genre {
    Pop,
    Rock,
    Mpb,
    Samba,
    BossaNova,
    Sertanejo,
    Forro,
    Gospel,
    Jazz,
    Blues,
    Funk,
    Reggae,
    RnbSoul,
    Edm,
    Ballad,
}

impl Genre {
    pub const ALL: [Genre; 15] = [
        Self::Pop,
        Self::Rock,
        Self::Mpb,
        Self::Samba,
        Self::BossaNova,
        Self::Sertanejo,
        Self::Forro,
        Self::Gospel,
        Self::Jazz,
        Self::Blues,
        Self::Funk,
        Self::Reggae,
        Self::RnbSoul,
        Self::Edm,
        Self::Ballad,
    ];

    /// Código estável (tabela `genre` do `library.db`).
    pub const fn code(self) -> &'static str {
        match self {
            Self::Pop => "pop",
            Self::Rock => "rock",
            Self::Mpb => "mpb",
            Self::Samba => "samba",
            Self::BossaNova => "bossa_nova",
            Self::Sertanejo => "sertanejo",
            Self::Forro => "forro",
            Self::Gospel => "gospel",
            Self::Jazz => "jazz",
            Self::Blues => "blues",
            Self::Funk => "funk",
            Self::Reggae => "reggae",
            Self::RnbSoul => "rnb_soul",
            Self::Edm => "edm",
            Self::Ballad => "ballad",
        }
    }

    pub fn from_code(code: &str) -> Result<Genre, HarmonyError> {
        Self::ALL
            .into_iter()
            .find(|g| g.code() == code)
            .ok_or_else(|| HarmonyError::UnknownGenre(code.to_owned()))
    }

    /// Nome na interface (PT-BR).
    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::Pop => "Pop",
            Self::Rock => "Rock",
            Self::Mpb => "MPB",
            Self::Samba => "Samba",
            Self::BossaNova => "Bossa nova",
            Self::Sertanejo => "Sertanejo",
            Self::Forro => "Forró",
            Self::Gospel => "Gospel",
            Self::Jazz => "Jazz",
            Self::Blues => "Blues",
            Self::Funk => "Funk",
            Self::Reggae => "Reggae",
            Self::RnbSoul => "R&B / Soul",
            Self::Edm => "Eletrônica (EDM)",
            Self::Ballad => "Balada",
        }
    }
}

/// Identidade de uma progressão: embutida (slug estável) ou do usuário (UUID v7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "origin", content = "id")]
pub enum ProgressionKey {
    Builtin(String),
    User(ProgressionId),
}

impl ProgressionKey {
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }
}

/// Notação de exibição alternável (RF-5.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Notation {
    /// Cifra popular (`C`, `Am7`, `G/B`).
    Cifra,
    /// Graus romanos (`I`, `vi7`, `V/7`).
    Roman,
}

/// Um acorde da progressão e quanto dura (em tempos de semínima).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionStep {
    pub chord: RomanChord,
    pub beats: u16,
}

/// Progressão harmônica relativa a um modo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progression {
    pub key: ProgressionKey,
    pub name: String,
    pub mode: Mode,
    pub genres: BTreeSet<Genre>,
    /// Progressão "universal" (funciona em praticamente qualquer estilo).
    pub universal: bool,
    /// Tempos por compasso (numerador; denominador semínima).
    pub beats_per_bar: u8,
    pub steps: Vec<ProgressionStep>,
}

impl Progression {
    /// Lê passos no formato `"I:4 V:4 vi:4 IV:4"` (`:tempos` opcional; padrão = 1 compasso).
    /// Os passos são normalizados para o modo (qualidade diatônica não é sobrescrita).
    pub fn parse_steps(
        text: &str,
        mode: Mode,
        beats_per_bar: u8,
    ) -> Result<Vec<ProgressionStep>, HarmonyError> {
        text.split_whitespace()
            .filter(|t| *t != "|")
            .map(|tok| {
                let (chord_txt, beats) = match tok.rsplit_once(':') {
                    Some((c, b)) => (
                        c,
                        b.parse::<u16>()
                            .map_err(|_| HarmonyError::InvalidDuration)?,
                    ),
                    None => (tok, beats_per_bar as u16),
                };
                let chord = RomanChord::parse(chord_txt)?.normalized(mode);
                Ok(ProgressionStep { chord, beats })
            })
            .collect()
    }

    /// Cria uma progressão do usuário já validada.
    pub fn new_user(
        name: &str,
        mode: Mode,
        genres: BTreeSet<Genre>,
        beats_per_bar: u8,
        steps_text: &str,
    ) -> Result<Progression, HarmonyError> {
        let p = Progression {
            key: ProgressionKey::User(ProgressionId::new()),
            name: validate_name(name)?,
            mode,
            genres,
            universal: false,
            beats_per_bar,
            steps: Self::parse_steps(steps_text, mode, beats_per_bar)?,
        };
        p.validate()?;
        Ok(p)
    }

    /// Regras de validade (nome, tamanho, durações, compasso, passos resolvíveis).
    pub fn validate(&self) -> Result<(), HarmonyError> {
        if validate_name(&self.name)? != self.name {
            return Err(HarmonyError::InvalidName);
        }
        if !(1..=16).contains(&self.beats_per_bar) {
            return Err(HarmonyError::InvalidBeatsPerBar);
        }
        if self.steps.is_empty() {
            return Err(HarmonyError::EmptyProgression);
        }
        if self.steps.len() > MAX_STEPS {
            return Err(HarmonyError::TooManySteps);
        }
        for s in &self.steps {
            if !(1..=MAX_STEP_BEATS).contains(&s.beats) {
                return Err(HarmonyError::InvalidDuration);
            }
            s.chord.validate()?;
        }
        Ok(())
    }

    pub fn total_beats(&self) -> u32 {
        self.steps.iter().map(|s| s.beats as u32).sum()
    }

    /// Texto canônico dos passos (`"I:4 V:4 …"`), inverso de [`Progression::parse_steps`].
    pub fn steps_text(&self) -> String {
        self.steps
            .iter()
            .map(|s| format!("{}:{}", s.chord.format(self.mode), s.beats))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Acordes absolutos na tônica dada (o modo é o da progressão).
    pub fn chords_in(&self, tonic_key: Key) -> Result<Vec<Chord>, HarmonyError> {
        let key = Key::new(tonic_key.tonic, self.mode);
        self.steps.iter().map(|s| s.chord.resolve(key)).collect()
    }

    /// Símbolos para exibição na notação escolhida (RF-5.7).
    pub fn format(&self, tonic_key: Key, notation: Notation) -> Result<Vec<String>, HarmonyError> {
        match notation {
            Notation::Roman => Ok(self
                .steps
                .iter()
                .map(|s| s.chord.format(self.mode))
                .collect()),
            Notation::Cifra => {
                let key = Key::new(tonic_key.tonic, self.mode);
                self.steps
                    .iter()
                    .map(|s| {
                        let chord = s.chord.resolve(key)?;
                        Ok(chord.symbol(s.chord.spelling_flats(key)))
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::roman::QualitySpec;

    fn key(s: &str) -> Key {
        Key::parse(s).unwrap()
    }

    #[test]
    fn parse_steps_normalizes_and_defaults_to_one_bar() {
        let steps = Progression::parse_steps("I V7:2 bVII:8 | vi", Mode::Ionian, 4).unwrap();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].beats, 4);
        assert_eq!(steps[0].chord.quality, QualitySpec::Diatonic);
        assert_eq!(steps[1].chord.quality, QualitySpec::DiatonicSeventh);
        assert_eq!(steps[1].beats, 2);
        assert_eq!(steps[2].beats, 8);
        assert!(Progression::parse_steps("I:x", Mode::Ionian, 4).is_err());
        assert!(Progression::parse_steps("Q", Mode::Ionian, 4).is_err());
    }

    #[test]
    fn user_progression_validation() {
        let p = Progression::new_user(
            "  Minha  ",
            Mode::Ionian,
            BTreeSet::from([Genre::Pop]),
            4,
            "I vi IV V",
        )
        .unwrap();
        assert_eq!(p.name, "Minha");
        assert!(p.key.is_user());
        assert_eq!(p.total_beats(), 16);
        assert_eq!(
            Progression::new_user("x", Mode::Ionian, BTreeSet::new(), 4, ""),
            Err(HarmonyError::EmptyProgression)
        );
        assert_eq!(
            Progression::new_user("x", Mode::Ionian, BTreeSet::new(), 4, "I:0"),
            Err(HarmonyError::InvalidDuration)
        );
        assert_eq!(
            Progression::new_user("x", Mode::Ionian, BTreeSet::new(), 0, "I:4"),
            Err(HarmonyError::InvalidBeatsPerBar)
        );
        let many = vec!["I"; 65].join(" ");
        assert_eq!(
            Progression::new_user("x", Mode::Ionian, BTreeSet::new(), 4, &many),
            Err(HarmonyError::TooManySteps)
        );
        assert_eq!(
            Progression::new_user("", Mode::Ionian, BTreeSet::new(), 4, "I"),
            Err(HarmonyError::InvalidName)
        );
    }

    #[test]
    fn notation_toggle_and_steps_text_roundtrip() {
        let p = Progression::new_user(
            "Teste",
            Mode::Ionian,
            BTreeSet::new(),
            4,
            "I:4 V/7:4 vi:4 V7/V:2 V7:2",
        )
        .unwrap();
        assert_eq!(
            p.format(key("C"), Notation::Cifra).unwrap(),
            ["C", "G/B", "Am", "D7", "G7"]
        );
        assert_eq!(
            p.format(key("F"), Notation::Cifra).unwrap(),
            ["F", "C/E", "Dm", "G7", "C7"]
        );
        assert_eq!(
            p.format(key("C"), Notation::Roman).unwrap(),
            ["I", "V/7", "vi", "V7/V", "V7"]
        );
        let text = p.steps_text();
        assert_eq!(text, "I:4 V/7:4 vi:4 V7/V:2 V7:2");
        assert_eq!(
            Progression::parse_steps(&text, p.mode, p.beats_per_bar).unwrap(),
            p.steps
        );
    }

    #[test]
    fn genre_codes_roundtrip() {
        for g in Genre::ALL {
            assert_eq!(Genre::from_code(g.code()).unwrap(), g);
        }
        assert!(Genre::from_code("polka").is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let p =
            Progression::new_user("Teste", Mode::Aeolian, BTreeSet::new(), 4, "i iv V7 i").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Progression = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
