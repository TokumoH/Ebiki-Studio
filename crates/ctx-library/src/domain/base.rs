//! Entidade [`Base`] e seus objetos de valor.

use super::error::{validate_name, LibraryError, MAX_PROGRESSION_TEXT, MAX_TAG_LEN};
use super::generator::{BassPattern, DrumPattern, StylePattern};
use serde::{Deserialize, Serialize};
use shared_kernel::define_id;
use shared_kernel::music::PitchClass;
use shared_kernel::tempo::{MAX_BPM, MIN_BPM};
use std::collections::BTreeSet;

define_id!(
    /// Identificador de base instrumental.
    BaseId
);

/// Modo simplificado da tonalidade de uma base (metadado de busca).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyMode {
    Major,
    Minor,
}

impl KeyMode {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
        }
    }

    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::Major => "maior",
            Self::Minor => "menor",
        }
    }
}

/// Tonalidade de uma base: tônica + maior/menor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseKey {
    pub tonic: PitchClass,
    pub mode: KeyMode,
}

impl BaseKey {
    pub const fn new(tonic: PitchClass, mode: KeyMode) -> Self {
        Self { tonic, mode }
    }

    /// Tônica da relativa (maior ↔ menor).
    pub fn relative_tonic(self) -> PitchClass {
        match self.mode {
            KeyMode::Major => self.tonic.transpose(-3),
            KeyMode::Minor => self.tonic.transpose(3),
        }
    }
}

/// Fórmula de compasso validada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "MeterRaw")]
pub struct Meter {
    numerator: u8,
    denominator: u8,
}

#[derive(Deserialize)]
struct MeterRaw {
    numerator: u8,
    denominator: u8,
}

impl TryFrom<MeterRaw> for Meter {
    type Error = LibraryError;
    fn try_from(r: MeterRaw) -> Result<Self, Self::Error> {
        Meter::new(r.numerator, r.denominator)
    }
}

impl Meter {
    pub const FOUR_FOUR: Meter = Meter {
        numerator: 4,
        denominator: 4,
    };
    pub const THREE_FOUR: Meter = Meter {
        numerator: 3,
        denominator: 4,
    };

    pub fn new(numerator: u8, denominator: u8) -> Result<Self, LibraryError> {
        if !(1..=16).contains(&numerator) || ![2, 4, 8, 16].contains(&denominator) {
            return Err(LibraryError::InvalidMeter);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const fn numerator(self) -> u8 {
        self.numerator
    }

    pub const fn denominator(self) -> u8 {
        self.denominator
    }

    /// Ticks de um tempo (unidade do denominador).
    pub fn beat_ticks(self) -> i64 {
        shared_kernel::PPQ * 4 / self.denominator as i64
    }

    pub fn bar_ticks(self) -> i64 {
        self.beat_ticks() * self.numerator as i64
    }
}

/// Caminho relativo seguro dentro da pasta da biblioteca (sem `..`, sem raiz, separador `/`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AudioPath(String);

impl AudioPath {
    pub fn new(p: impl Into<String>) -> Result<Self, LibraryError> {
        let p: String = p.into();
        let bad = p.is_empty()
            || p.starts_with('/')
            || p.contains('\\')
            || p.contains(':')
            || p.chars().any(char::is_control)
            || p.split('/')
                .any(|seg| seg.is_empty() || seg == "." || seg == "..");
        if bad {
            return Err(LibraryError::InvalidPath(p));
        }
        Ok(Self(p))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AudioPath {
    type Error = LibraryError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        AudioPath::new(s)
    }
}

impl From<AudioPath> for String {
    fn from(p: AudioPath) -> String {
        p.0
    }
}

/// Tag livre normalizada (minúsculas, espaços colapsados, 1–32 caracteres, sem vírgula).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Tag(String);

impl Tag {
    pub fn new(raw: &str) -> Result<Self, LibraryError> {
        let norm = raw
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let len = norm.chars().count();
        if len == 0 || len > MAX_TAG_LEN || norm.contains(',') || norm.chars().any(char::is_control)
        {
            return Err(LibraryError::InvalidTag(raw.to_owned()));
        }
        Ok(Self(norm))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Tag {
    type Error = LibraryError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Tag::new(&s)
    }
}

impl From<Tag> for String {
    fn from(t: Tag) -> String {
        t.0
    }
}

/// Código de gênero (vocabulário aberto compartilhado com Harmony pela tabela `genre`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GenreCode(String);

impl GenreCode {
    pub fn new(code: &str) -> Result<Self, LibraryError> {
        let ok = code.len() <= 32
            && code.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && code
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !ok {
            return Err(LibraryError::InvalidGenre(code.to_owned()));
        }
        Ok(Self(code.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for GenreCode {
    type Error = LibraryError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        GenreCode::new(&s)
    }
}

impl From<GenreCode> for String {
    fn from(g: GenreCode) -> String {
        g.0
    }
}

/// Instrumentos presentes na base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Instrument {
    Drums,
    Percussion,
    Bass,
    AcousticGuitar,
    NylonGuitar,
    ElectricGuitar,
    Cavaquinho,
    ViolaCaipira,
    Piano,
    Keys,
    Organ,
    Synth,
    Strings,
    Brass,
    Woodwinds,
    Accordion,
    Pandeiro,
    Zabumba,
    Triangle,
    Other,
}

impl Instrument {
    pub const ALL: [Instrument; 20] = [
        Self::Drums,
        Self::Percussion,
        Self::Bass,
        Self::AcousticGuitar,
        Self::NylonGuitar,
        Self::ElectricGuitar,
        Self::Cavaquinho,
        Self::ViolaCaipira,
        Self::Piano,
        Self::Keys,
        Self::Organ,
        Self::Synth,
        Self::Strings,
        Self::Brass,
        Self::Woodwinds,
        Self::Accordion,
        Self::Pandeiro,
        Self::Zabumba,
        Self::Triangle,
        Self::Other,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Drums => "drums",
            Self::Percussion => "percussion",
            Self::Bass => "bass",
            Self::AcousticGuitar => "acoustic_guitar",
            Self::NylonGuitar => "nylon_guitar",
            Self::ElectricGuitar => "electric_guitar",
            Self::Cavaquinho => "cavaquinho",
            Self::ViolaCaipira => "viola_caipira",
            Self::Piano => "piano",
            Self::Keys => "keys",
            Self::Organ => "organ",
            Self::Synth => "synth",
            Self::Strings => "strings",
            Self::Brass => "brass",
            Self::Woodwinds => "woodwinds",
            Self::Accordion => "accordion",
            Self::Pandeiro => "pandeiro",
            Self::Zabumba => "zabumba",
            Self::Triangle => "triangle",
            Self::Other => "other",
        }
    }

    pub fn from_code(code: &str) -> Result<Instrument, LibraryError> {
        Self::ALL
            .into_iter()
            .find(|i| i.code() == code)
            .ok_or_else(|| LibraryError::UnknownInstrument(code.to_owned()))
    }

    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::Drums => "Bateria",
            Self::Percussion => "Percussão",
            Self::Bass => "Baixo",
            Self::AcousticGuitar => "Violão de aço",
            Self::NylonGuitar => "Violão de nylon",
            Self::ElectricGuitar => "Guitarra",
            Self::Cavaquinho => "Cavaquinho",
            Self::ViolaCaipira => "Viola caipira",
            Self::Piano => "Piano",
            Self::Keys => "Teclado / piano elétrico",
            Self::Organ => "Órgão",
            Self::Synth => "Sintetizador",
            Self::Strings => "Cordas",
            Self::Brass => "Metais",
            Self::Woodwinds => "Madeiras",
            Self::Accordion => "Sanfona",
            Self::Pandeiro => "Pandeiro",
            Self::Zabumba => "Zabumba",
            Self::Triangle => "Triângulo",
            Self::Other => "Outro",
        }
    }
}

/// Base de áudio (CC0 incluída ou do usuário).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBase {
    pub path: AudioPath,
    /// Progressão anotada (graus, ex.: `"I:4 V:4 vi:4 IV:4"`), opcional.
    #[serde(default)]
    pub progression: Option<String>,
}

/// Receita de base gerada pelo app (a progressão é em graus; a tonalidade é a da base).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRecipe {
    pub progression: String,
    pub bars: u16,
    pub style: StylePattern,
    pub drums: DrumPattern,
    pub bass: BassPattern,
}

/// Tipo da base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum BaseKind {
    /// Áudio livre (CC0) incluído no aplicativo — somente leitura.
    BundledCc0(AudioBase),
    /// Gerada pelo app a partir de progressão + padrões rítmicos.
    Generated(GenerationRecipe),
    /// Áudio importado pelo usuário.
    UserAudio(AudioBase),
}

/// Discriminante do tipo (filtro e coluna `base.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BaseKindTag {
    BundledCc0,
    Generated,
    UserAudio,
}

impl BaseKindTag {
    pub const fn code(self) -> &'static str {
        match self {
            Self::BundledCc0 => "bundled_cc0",
            Self::Generated => "generated",
            Self::UserAudio => "user_audio",
        }
    }
}

impl BaseKind {
    pub fn tag(&self) -> BaseKindTag {
        match self {
            Self::BundledCc0(_) => BaseKindTag::BundledCc0,
            Self::Generated(_) => BaseKindTag::Generated,
            Self::UserAudio(_) => BaseKindTag::UserAudio,
        }
    }

    pub fn audio(&self) -> Option<&AudioBase> {
        match self {
            Self::BundledCc0(a) | Self::UserAudio(a) => Some(a),
            Self::Generated(_) => None,
        }
    }
}

/// Base instrumental.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    pub id: BaseId,
    pub name: String,
    pub kind: BaseKind,
    pub bpm: f64,
    #[serde(default)]
    pub key: Option<BaseKey>,
    pub meter: Meter,
    #[serde(default)]
    pub genre: Option<GenreCode>,
    #[serde(default)]
    pub instruments: BTreeSet<Instrument>,
    #[serde(default)]
    pub tags: BTreeSet<Tag>,
}

fn validate_progression_text(t: &str, required: bool) -> Result<(), LibraryError> {
    let len = t.chars().count();
    let bad = len > MAX_PROGRESSION_TEXT
        || t.chars().any(char::is_control)
        || (required && t.trim().is_empty());
    if bad {
        return Err(LibraryError::InvalidProgressionText);
    }
    Ok(())
}

impl Base {
    /// Cria e valida uma base nova (id gerado).
    pub fn new(
        name: &str,
        kind: BaseKind,
        bpm: f64,
        key: Option<BaseKey>,
        meter: Meter,
    ) -> Result<Base, LibraryError> {
        let b = Base {
            id: BaseId::new(),
            name: validate_name(name)?,
            kind,
            bpm,
            key,
            meter,
            genre: None,
            instruments: BTreeSet::new(),
            tags: BTreeSet::new(),
        };
        b.validate()?;
        Ok(b)
    }

    /// Progressão da base: a da receita (gerada) ou a anotada (áudio).
    pub fn progression(&self) -> Option<&str> {
        match &self.kind {
            BaseKind::Generated(r) => Some(r.progression.as_str()),
            BaseKind::BundledCc0(a) | BaseKind::UserAudio(a) => a.progression.as_deref(),
        }
    }

    /// Regras de validade.
    pub fn validate(&self) -> Result<(), LibraryError> {
        if validate_name(&self.name)? != self.name {
            return Err(LibraryError::InvalidName);
        }
        if !self.bpm.is_finite() || !(MIN_BPM..=MAX_BPM).contains(&self.bpm) {
            return Err(LibraryError::InvalidBpm);
        }
        match &self.kind {
            BaseKind::BundledCc0(a) | BaseKind::UserAudio(a) => {
                if let Some(p) = &a.progression {
                    validate_progression_text(p, false)?;
                }
            }
            BaseKind::Generated(r) => {
                if self.key.is_none() {
                    return Err(LibraryError::GeneratedNeedsKey);
                }
                validate_progression_text(&r.progression, true)?;
                if !(1..=256).contains(&r.bars) {
                    return Err(LibraryError::InvalidBars);
                }
                r.style.check_meter(self.meter)?;
                r.drums.check_meter(self.meter)?;
                r.bass.check_meter(self.meter)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(s: &str) -> PitchClass {
        PitchClass::parse(s).unwrap()
    }

    fn audio(path: &str) -> BaseKind {
        BaseKind::UserAudio(AudioBase {
            path: AudioPath::new(path).unwrap(),
            progression: Some("I V vi IV".into()),
        })
    }

    fn recipe(style: StylePattern) -> BaseKind {
        BaseKind::Generated(GenerationRecipe {
            progression: "I:4 IV:4 V:4 I:4".into(),
            bars: 8,
            style,
            drums: style.default_drums(),
            bass: style.default_bass(),
        })
    }

    #[test]
    fn value_objects_validate() {
        assert!(Meter::new(3, 4).is_ok());
        assert!(Meter::new(0, 4).is_err());
        assert!(Meter::new(4, 3).is_err());
        assert_eq!(Meter::new(6, 8).unwrap().bar_ticks(), 6 * 480);
        assert_eq!(Meter::THREE_FOUR.bar_ticks(), 3 * 960);
        assert!(serde_json::from_str::<Meter>(r#"{"numerator":4,"denominator":5}"#).is_err());
        assert!(AudioPath::new("bases/pop/a.wav").is_ok());
        for bad in ["", "/abs.wav", "../x.wav", "a//b.wav", "c:\\x.wav", "a/./b"] {
            assert!(AudioPath::new(bad).is_err(), "{bad}");
        }
        assert_eq!(Tag::new("  Lo   Fi ").unwrap().as_str(), "lo fi");
        assert!(Tag::new("a,b").is_err());
        assert!(Tag::new("").is_err());
        assert!(Tag::new(&"x".repeat(33)).is_err());
        assert!(GenreCode::new("bossa_nova").is_ok());
        for bad in ["", "Pop", "1pop", "pop-rock"] {
            assert!(GenreCode::new(bad).is_err(), "{bad}");
        }
        for i in Instrument::ALL {
            assert_eq!(Instrument::from_code(i.code()).unwrap(), i);
        }
        assert!(Instrument::from_code("kazoo").is_err());
        let am = BaseKey::new(pc("A"), KeyMode::Minor);
        assert_eq!(am.relative_tonic(), pc("C"));
    }

    #[test]
    fn base_validation_rules() {
        let c = Some(BaseKey::new(pc("C"), KeyMode::Major));
        assert!(Base::new("Pop", audio("a.wav"), 120.0, None, Meter::FOUR_FOUR).is_ok());
        assert_eq!(
            Base::new("Pop", audio("a.wav"), f64::NAN, None, Meter::FOUR_FOUR),
            Err(LibraryError::InvalidBpm)
        );
        assert_eq!(
            Base::new("Pop", audio("a.wav"), 1000.0, None, Meter::FOUR_FOUR),
            Err(LibraryError::InvalidBpm)
        );
        assert_eq!(
            Base::new("", audio("a.wav"), 100.0, None, Meter::FOUR_FOUR),
            Err(LibraryError::InvalidName)
        );
        assert_eq!(
            Base::new(
                "Gerada",
                recipe(StylePattern::PopStrum),
                100.0,
                None,
                Meter::FOUR_FOUR
            ),
            Err(LibraryError::GeneratedNeedsKey)
        );
        assert!(matches!(
            Base::new(
                "Valsa",
                recipe(StylePattern::Waltz),
                100.0,
                c,
                Meter::FOUR_FOUR
            ),
            Err(LibraryError::PatternMeterMismatch {
                expected: 3,
                actual: 4,
                ..
            })
        ));
        let ok = Base::new(
            "Valsa",
            recipe(StylePattern::Waltz),
            100.0,
            c,
            Meter::THREE_FOUR,
        )
        .unwrap();
        assert_eq!(ok.progression(), Some("I:4 IV:4 V:4 I:4"));
        let mut bad_bars = ok.clone();
        if let BaseKind::Generated(r) = &mut bad_bars.kind {
            r.bars = 0;
        }
        assert_eq!(bad_bars.validate(), Err(LibraryError::InvalidBars));
        let mut long_prog = Base::new("P", audio("b.wav"), 90.0, None, Meter::FOUR_FOUR).unwrap();
        long_prog.kind = BaseKind::UserAudio(AudioBase {
            path: AudioPath::new("b.wav").unwrap(),
            progression: Some("I ".repeat(200)),
        });
        assert_eq!(
            long_prog.validate(),
            Err(LibraryError::InvalidProgressionText)
        );
    }

    #[test]
    fn serde_roundtrip() {
        let mut b = Base::new(
            "Samba",
            recipe(StylePattern::SambaComping),
            98.0,
            Some(BaseKey::new(pc("D"), KeyMode::Major)),
            Meter::FOUR_FOUR,
        )
        .unwrap();
        b.tags.insert(Tag::new("roda").unwrap());
        b.instruments.insert(Instrument::Pandeiro);
        b.genre = Some(GenreCode::new("samba").unwrap());
        let json = serde_json::to_string(&b).unwrap();
        let back: Base = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }
}
