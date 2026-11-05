//! Acordes: qualidade, tensões, baixo (acorde invertido) e cifra popular.
//!
//! A forma canônica de escrita segue a cifra brasileira: alterações e tensões entre parênteses
//! (`C7(9)`, `F#m7(b5)`, `Cm(maj7)`, `Gmaj7(#5)`), baixo após barra (`G/B`). A leitura aceita as
//! variantes comuns (`C9`, `Cm7b5`, `Cø`, `C7M`, `Cadd9`, `C6/9`, `C°7`, `C+`…).

use super::error::HarmonyError;
use serde::{Deserialize, Serialize};
use shared_kernel::music::PitchClass;
use std::collections::BTreeSet;
use std::fmt;

/// Qualidade (notas estruturais) de um acorde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    Sus2,
    Sus4,
    Power,
    Major6,
    Minor6,
    Major7,
    Dominant7,
    Minor7,
    HalfDiminished7,
    Diminished7,
    MinorMajor7,
    AugmentedMajor7,
    Augmented7,
    Dominant7Sus4,
}

impl ChordQuality {
    pub const ALL: [ChordQuality; 18] = [
        Self::Major,
        Self::Minor,
        Self::Diminished,
        Self::Augmented,
        Self::Sus2,
        Self::Sus4,
        Self::Power,
        Self::Major6,
        Self::Minor6,
        Self::Major7,
        Self::Dominant7,
        Self::Minor7,
        Self::HalfDiminished7,
        Self::Diminished7,
        Self::MinorMajor7,
        Self::AugmentedMajor7,
        Self::Augmented7,
        Self::Dominant7Sus4,
    ];

    /// Intervalos (semitons a partir da fundamental), em ordem de empilhamento.
    pub const fn intervals(self) -> &'static [u8] {
        match self {
            Self::Major => &[0, 4, 7],
            Self::Minor => &[0, 3, 7],
            Self::Diminished => &[0, 3, 6],
            Self::Augmented => &[0, 4, 8],
            Self::Sus2 => &[0, 2, 7],
            Self::Sus4 => &[0, 5, 7],
            Self::Power => &[0, 7],
            Self::Major6 => &[0, 4, 7, 9],
            Self::Minor6 => &[0, 3, 7, 9],
            Self::Major7 => &[0, 4, 7, 11],
            Self::Dominant7 => &[0, 4, 7, 10],
            Self::Minor7 => &[0, 3, 7, 10],
            Self::HalfDiminished7 => &[0, 3, 6, 10],
            Self::Diminished7 => &[0, 3, 6, 9],
            Self::MinorMajor7 => &[0, 3, 7, 11],
            Self::AugmentedMajor7 => &[0, 4, 8, 11],
            Self::Augmented7 => &[0, 4, 8, 10],
            Self::Dominant7Sus4 => &[0, 5, 7, 10],
        }
    }

    /// Código estável (persistência).
    pub const fn code(self) -> &'static str {
        match self {
            Self::Major => "maj",
            Self::Minor => "min",
            Self::Diminished => "dim",
            Self::Augmented => "aug",
            Self::Sus2 => "sus2",
            Self::Sus4 => "sus4",
            Self::Power => "5",
            Self::Major6 => "6",
            Self::Minor6 => "m6",
            Self::Major7 => "maj7",
            Self::Dominant7 => "7",
            Self::Minor7 => "m7",
            Self::HalfDiminished7 => "m7b5",
            Self::Diminished7 => "dim7",
            Self::MinorMajor7 => "mmaj7",
            Self::AugmentedMajor7 => "augmaj7",
            Self::Augmented7 => "aug7",
            Self::Dominant7Sus4 => "7sus4",
        }
    }

    pub fn from_code(code: &str) -> Option<ChordQuality> {
        Self::ALL.into_iter().find(|q| q.code() == code)
    }

    /// Tem sétima (ou sexta) além da tríade?
    pub fn has_seventh(self) -> bool {
        matches!(
            self,
            Self::Major7
                | Self::Dominant7
                | Self::Minor7
                | Self::HalfDiminished7
                | Self::Diminished7
                | Self::MinorMajor7
                | Self::AugmentedMajor7
                | Self::Augmented7
                | Self::Dominant7Sus4
        )
    }

    /// Terça menor (grau romano em minúsculas)?
    pub fn is_minor_like(self) -> bool {
        self.intervals().get(1) == Some(&3)
    }

    /// Função de dominante (tríade maior ou com sétima menor e terça maior)?
    pub fn is_dominant_like(self) -> bool {
        matches!(
            self,
            Self::Dominant7 | Self::Augmented7 | Self::Dominant7Sus4
        )
    }

    /// Detecta a qualidade a partir de um conjunto de intervalos (ordem e oitavas ignoradas).
    pub fn from_intervals(intervals: &[u8]) -> Option<ChordQuality> {
        let set: BTreeSet<u8> = intervals.iter().map(|i| i % 12).collect();
        Self::ALL.into_iter().find(|q| {
            let qs: BTreeSet<u8> = q.intervals().iter().copied().collect();
            qs == set
        })
    }

    /// Partes da cifra: sufixo base e alteração (escrita entre parênteses).
    pub(crate) const fn cifra_parts(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Major => ("", None),
            Self::Minor => ("m", None),
            Self::Diminished => ("dim", None),
            Self::Augmented => ("aug", None),
            Self::Sus2 => ("sus2", None),
            Self::Sus4 => ("sus4", None),
            Self::Power => ("5", None),
            Self::Major6 => ("6", None),
            Self::Minor6 => ("m6", None),
            Self::Major7 => ("maj7", None),
            Self::Dominant7 => ("7", None),
            Self::Minor7 => ("m7", None),
            Self::HalfDiminished7 => ("m7", Some("b5")),
            Self::Diminished7 => ("dim7", None),
            Self::MinorMajor7 => ("m", Some("maj7")),
            Self::AugmentedMajor7 => ("maj7", Some("#5")),
            Self::Augmented7 => ("7", Some("#5")),
            Self::Dominant7Sus4 => ("7sus4", None),
        }
    }

    /// Partes do sufixo em graus romanos (a caixa do numeral já indica a terça).
    pub(crate) const fn roman_parts(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Major | Self::Minor => ("", None),
            Self::Diminished => ("°", None),
            Self::Augmented => ("+", None),
            Self::Sus2 => ("sus2", None),
            Self::Sus4 => ("sus4", None),
            Self::Power => ("5", None),
            Self::Major6 | Self::Minor6 => ("6", None),
            Self::Major7 | Self::MinorMajor7 => ("maj7", None),
            Self::Dominant7 | Self::Minor7 => ("7", None),
            Self::HalfDiminished7 => ("ø7", None),
            Self::Diminished7 => ("°7", None),
            Self::AugmentedMajor7 => ("+maj7", None),
            Self::Augmented7 => ("+7", None),
            Self::Dominant7Sus4 => ("7sus4", None),
        }
    }
}

/// Tensão (nota acrescentada acima da estrutura).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tension {
    FlatNine,
    Nine,
    SharpNine,
    Eleven,
    SharpEleven,
    FlatThirteen,
    Thirteen,
}

impl Tension {
    pub const ALL: [Tension; 7] = [
        Self::FlatNine,
        Self::Nine,
        Self::SharpNine,
        Self::Eleven,
        Self::SharpEleven,
        Self::FlatThirteen,
        Self::Thirteen,
    ];

    pub const fn semitones(self) -> u8 {
        match self {
            Self::FlatNine => 1,
            Self::Nine => 2,
            Self::SharpNine => 3,
            Self::Eleven => 5,
            Self::SharpEleven => 6,
            Self::FlatThirteen => 8,
            Self::Thirteen => 9,
        }
    }

    /// Símbolo na cifra e código estável.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::FlatNine => "b9",
            Self::Nine => "9",
            Self::SharpNine => "#9",
            Self::Eleven => "11",
            Self::SharpEleven => "#11",
            Self::FlatThirteen => "b13",
            Self::Thirteen => "13",
        }
    }
}

/// Acorde absoluto (fundamental concreta).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chord {
    pub root: PitchClass,
    pub quality: ChordQuality,
    #[serde(default)]
    pub tensions: BTreeSet<Tension>,
    #[serde(default)]
    pub bass: Option<PitchClass>,
}

impl Chord {
    pub fn new(root: PitchClass, quality: ChordQuality) -> Self {
        Self {
            root,
            quality,
            tensions: BTreeSet::new(),
            bass: None,
        }
    }

    pub fn with_tension(mut self, t: Tension) -> Self {
        self.tensions.insert(t);
        self
    }

    /// Define o baixo; baixo igual à fundamental é normalizado para `None`.
    pub fn with_bass(mut self, bass: Option<PitchClass>) -> Self {
        self.bass = bass.filter(|b| *b != self.root);
        self
    }

    /// Classes de altura: estrutura, depois tensões (sem repetição). O baixo não é incluído.
    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        let mut out: Vec<PitchClass> = Vec::with_capacity(7);
        let all = self
            .quality
            .intervals()
            .iter()
            .copied()
            .chain(self.tensions.iter().map(|t| t.semitones()));
        for i in all {
            let p = self.root.transpose(i as i32);
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    pub fn transpose(&self, semitones: i32) -> Chord {
        Chord {
            root: self.root.transpose(semitones),
            quality: self.quality,
            tensions: self.tensions.clone(),
            bass: self.bass.map(|b| b.transpose(semitones)),
        }
    }

    /// Identifica um acorde a partir de classes de altura; a primeira é o baixo.
    /// Tenta primeiro o baixo como fundamental; senão gera acorde com baixo (inversão).
    pub fn identify(notes: &[PitchClass]) -> Option<Chord> {
        let bass = *notes.first()?;
        let mut candidates: Vec<PitchClass> = vec![bass];
        for n in notes {
            if !candidates.contains(n) {
                candidates.push(*n);
            }
        }
        candidates.iter().find_map(|root| {
            let iv: Vec<u8> = notes.iter().map(|n| root.interval_to(*n)).collect();
            ChordQuality::from_intervals(&iv).map(|q| Chord::new(*root, q).with_bass(Some(bass)))
        })
    }

    /// Lê cifra popular: `C`, `Am7`, `G/B`, `F#m7(b5)`, `Bbmaj7`, `Dsus4`, `C7(9)`, `C9`, `C6/9`…
    pub fn parse(text: &str) -> Result<Chord, HarmonyError> {
        let err = || HarmonyError::InvalidChord(text.to_owned());
        let s = text.trim().replace("6/9", "6(9)");
        let (body, bass) = match s.rfind('/') {
            Some(i) => {
                let b = PitchClass::parse(&s[i + 1..]).ok_or_else(err)?;
                (&s[..i], Some(b))
            }
            None => (s.as_str(), None),
        };
        let mut chars = body.char_indices();
        let (_, letter) = chars.next().ok_or_else(err)?;
        if !matches!(letter.to_ascii_uppercase(), 'A'..='G') {
            return Err(err());
        }
        let mut root_end = letter.len_utf8();
        if let Some((i, c)) = chars.next() {
            if matches!(c, '#' | 'b' | '♯' | '♭') {
                root_end = i + c.len_utf8();
            }
        }
        let root = PitchClass::parse(&body[..root_end]).ok_or_else(err)?;
        let (quality, tensions) =
            parse_suffix(&body[root_end..], ChordQuality::Major).ok_or_else(err)?;
        Ok(Chord {
            root,
            quality,
            tensions,
            bass: None,
        }
        .with_bass(bass))
    }

    /// Escreve em cifra canônica.
    pub fn symbol(&self, prefer_flats: bool) -> String {
        let (base, alter) = self.quality.cifra_parts();
        let mut s = String::from(self.root.name(prefer_flats));
        s.push_str(base);
        s.push_str(&paren_list(alter, &self.tensions));
        if let Some(b) = self.bass {
            s.push('/');
            s.push_str(b.name(prefer_flats));
        }
        s
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.symbol(false))
    }
}

/// `(alteração,tensões…)` ou vazio.
pub(crate) fn paren_list(alter: Option<&str>, tensions: &BTreeSet<Tension>) -> String {
    let items: Vec<&str> = alter
        .into_iter()
        .chain(tensions.iter().map(|t| t.symbol()))
        .collect();
    if items.is_empty() {
        String::new()
    } else {
        format!("({})", items.join(","))
    }
}

const TOKENS: &[&str] = &[
    "maj13", "maj11", "maj9", "maj7", "Maj7", "mmaj7", "mMaj7", "add13", "add11", "add9", "add4",
    "add2", "sus4", "sus2", "dim7", "min", "maj", "aug", "dim", "sus", "7M", "M7", "°7", "º7",
    "o7", "ø7", "Ø7", "Δ7", "b13", "#11", "13", "11", "b9", "#9", "b5", "#5", "+5", "ø", "Ø", "°",
    "º", "Δ", "m", "M", "-", "+", "9", "7", "6", "5", "o", "(", ")", ",", " ",
];

/// Lê o sufixo de qualidade/tensões a partir de uma qualidade inicial (`Major` para cifra;
/// `Major`/`Minor` conforme a caixa do numeral romano). `None` se inválido.
pub(crate) fn parse_suffix(
    suffix: &str,
    start: ChordQuality,
) -> Option<(ChordQuality, BTreeSet<Tension>)> {
    use ChordQuality as Q;
    let mut q = start;
    let mut tensions = BTreeSet::new();
    let mut depth: i32 = 0;
    let mut rest = suffix;
    let mut first = true;
    while !rest.is_empty() {
        let tok = TOKENS
            .iter()
            .filter(|t| rest.starts_with(**t))
            .max_by_key(|t| t.len())?;
        rest = &rest[tok.len()..];
        let in_parens = depth > 0;
        let seventh = |q: Q| match q {
            Q::Major => Some(Q::Dominant7),
            Q::Minor => Some(Q::Minor7),
            Q::Diminished => Some(Q::Diminished7),
            Q::Augmented => Some(Q::Augmented7),
            Q::Sus4 => Some(Q::Dominant7Sus4),
            _ => None,
        };
        let major_seventh = |q: Q| match q {
            Q::Major => Some(Q::Major7),
            Q::Minor => Some(Q::MinorMajor7),
            Q::Augmented => Some(Q::AugmentedMajor7),
            _ => None,
        };
        let sharp_five = |q: Q| match q {
            Q::Major | Q::Augmented => Some(Q::Augmented),
            Q::Dominant7 | Q::Augmented7 => Some(Q::Augmented7),
            Q::Major7 | Q::AugmentedMajor7 => Some(Q::AugmentedMajor7),
            _ => None,
        };
        let ext = |tensions: &mut BTreeSet<Tension>, n: &str| {
            tensions.insert(Tension::Nine);
            match n {
                "11" => {
                    tensions.insert(Tension::Eleven);
                }
                "13" => {
                    tensions.insert(Tension::Thirteen);
                }
                _ => {}
            }
        };
        match *tok {
            "(" => depth += 1,
            ")" => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            "," | " " => {}
            "M" | "maj" => {}
            "m" | "min" | "-" => {
                q = match q {
                    Q::Major | Q::Minor => Q::Minor,
                    _ => return None,
                }
            }
            "maj7" | "Maj7" | "7M" | "M7" | "Δ" | "Δ7" => q = major_seventh(q)?,
            "mmaj7" | "mMaj7" => {
                q = match q {
                    Q::Major | Q::Minor => Q::MinorMajor7,
                    _ => return None,
                }
            }
            "maj9" | "maj11" | "maj13" => {
                q = major_seventh(q)?;
                ext(&mut tensions, &tok[3..]);
            }
            "7" => q = seventh(q)?,
            "9" | "11" | "13" => {
                if !in_parens && matches!(q, Q::Major | Q::Minor | Q::Sus4) {
                    q = seventh(q)?;
                    ext(&mut tensions, tok);
                } else {
                    tensions.insert(match *tok {
                        "9" => Tension::Nine,
                        "11" => Tension::Eleven,
                        _ => Tension::Thirteen,
                    });
                }
            }
            "b9" => {
                tensions.insert(Tension::FlatNine);
            }
            "#9" => {
                tensions.insert(Tension::SharpNine);
            }
            "#11" => {
                tensions.insert(Tension::SharpEleven);
            }
            "b13" => {
                tensions.insert(Tension::FlatThirteen);
            }
            "add9" | "add2" => {
                tensions.insert(Tension::Nine);
            }
            "add11" | "add4" => {
                tensions.insert(Tension::Eleven);
            }
            "add13" => {
                tensions.insert(Tension::Thirteen);
            }
            "6" => {
                q = match q {
                    Q::Major => Q::Major6,
                    Q::Minor => Q::Minor6,
                    _ => return None,
                }
            }
            "5" => {
                if q != Q::Major || !tensions.is_empty() || !first {
                    return None;
                }
                q = Q::Power;
            }
            "b5" => {
                q = match q {
                    Q::Minor7 | Q::HalfDiminished7 => Q::HalfDiminished7,
                    Q::Minor | Q::Diminished => Q::Diminished,
                    _ => return None,
                }
            }
            "#5" | "+5" | "+" | "aug" => q = sharp_five(q)?,
            "dim" | "°" | "º" | "o" => {
                q = match q {
                    Q::Major | Q::Minor => Q::Diminished,
                    _ => return None,
                }
            }
            "dim7" | "°7" | "º7" | "o7" => {
                q = match q {
                    Q::Major | Q::Minor | Q::Diminished => Q::Diminished7,
                    _ => return None,
                }
            }
            "ø" | "Ø" | "ø7" | "Ø7" => {
                q = match q {
                    Q::Major | Q::Minor | Q::Minor7 => Q::HalfDiminished7,
                    _ => return None,
                }
            }
            "sus4" | "sus" => {
                q = match q {
                    Q::Major => Q::Sus4,
                    Q::Dominant7 => Q::Dominant7Sus4,
                    _ => return None,
                }
            }
            "sus2" => {
                q = match q {
                    Q::Major => Q::Sus2,
                    _ => return None,
                }
            }
            _ => return None,
        }
        if !matches!(*tok, "(" | ")" | "," | " ") {
            first = false;
        }
    }
    (depth == 0).then_some((q, tensions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn pc(s: &str) -> PitchClass {
        PitchClass::parse(s).unwrap()
    }

    fn chord(s: &str) -> Chord {
        Chord::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn quality_detection_from_intervals() {
        use ChordQuality as Q;
        let cases: [(&[u8], Q); 11] = [
            (&[0, 4, 7], Q::Major),
            (&[0, 3, 7], Q::Minor),
            (&[0, 3, 6], Q::Diminished),
            (&[0, 4, 8], Q::Augmented),
            (&[0, 4, 7, 11], Q::Major7),
            (&[0, 4, 7, 10], Q::Dominant7),
            (&[0, 3, 7, 10], Q::Minor7),
            (&[0, 3, 6, 10], Q::HalfDiminished7),
            (&[0, 3, 6, 9], Q::Diminished7),
            (&[0, 3, 7, 11], Q::MinorMajor7),
            (&[0, 4, 8, 11], Q::AugmentedMajor7),
        ];
        for (iv, q) in cases {
            assert_eq!(ChordQuality::from_intervals(iv), Some(q), "{iv:?}");
        }
        assert_eq!(ChordQuality::from_intervals(&[7, 0, 16]), Some(Q::Major));
        assert_eq!(ChordQuality::from_intervals(&[0, 1, 2]), None);
        for q in ChordQuality::ALL {
            assert_eq!(ChordQuality::from_code(q.code()), Some(q));
        }
    }

    #[test]
    fn parses_popular_cifra() {
        use ChordQuality as Q;
        assert_eq!(chord("C"), Chord::new(pc("C"), Q::Major));
        assert_eq!(chord("Am7"), Chord::new(pc("A"), Q::Minor7));
        assert_eq!(
            chord("G/B"),
            Chord::new(pc("G"), Q::Major).with_bass(Some(pc("B")))
        );
        assert_eq!(chord("F#m7(b5)"), Chord::new(pc("F#"), Q::HalfDiminished7));
        assert_eq!(chord("F#m7b5"), Chord::new(pc("F#"), Q::HalfDiminished7));
        assert_eq!(chord("F#ø"), Chord::new(pc("F#"), Q::HalfDiminished7));
        assert_eq!(chord("Bbmaj7"), Chord::new(pc("Bb"), Q::Major7));
        assert_eq!(chord("Bb7M"), Chord::new(pc("Bb"), Q::Major7));
        assert_eq!(chord("Dsus4"), Chord::new(pc("D"), Q::Sus4));
        assert_eq!(
            chord("C7(9)"),
            Chord::new(pc("C"), Q::Dominant7).with_tension(Tension::Nine)
        );
        assert_eq!(chord("C9"), chord("C7(9)"));
        assert_eq!(chord("Cadd9"), chord("C(9)"));
        assert_eq!(
            chord("C6/9"),
            Chord::new(pc("C"), Q::Major6).with_tension(Tension::Nine)
        );
        assert_eq!(chord("Cm(maj7)"), Chord::new(pc("C"), Q::MinorMajor7));
        assert_eq!(chord("Cm7M"), Chord::new(pc("C"), Q::MinorMajor7));
        assert_eq!(chord("C°7"), Chord::new(pc("C"), Q::Diminished7));
        assert_eq!(chord("Cdim"), Chord::new(pc("C"), Q::Diminished));
        assert_eq!(chord("C+"), Chord::new(pc("C"), Q::Augmented));
        assert_eq!(chord("C7(#5)"), Chord::new(pc("C"), Q::Augmented7));
        assert_eq!(chord("Cmaj7(#5)"), Chord::new(pc("C"), Q::AugmentedMajor7));
        assert_eq!(chord("C7sus4"), Chord::new(pc("C"), Q::Dominant7Sus4));
        assert_eq!(chord("E5"), Chord::new(pc("E"), Q::Power));
        assert_eq!(
            chord("G13"),
            Chord::new(pc("G"), Q::Dominant7)
                .with_tension(Tension::Nine)
                .with_tension(Tension::Thirteen)
        );
        assert_eq!(
            chord("C/C"),
            Chord::new(pc("C"), Q::Major),
            "baixo = fundamental"
        );
        for bad in [
            "", "H", "Cx", "C7(9", "C)", "C/H", "Cm7sus2", "C5(b5)", "Cdimm",
        ] {
            assert!(Chord::parse(bad).is_err(), "{bad} deveria falhar");
        }
    }

    #[test]
    fn formats_canonical_cifra() {
        assert_eq!(chord("F#m7b5").symbol(false), "F#m7(b5)");
        assert_eq!(chord("Bb7M").symbol(true), "Bbmaj7");
        assert_eq!(chord("C9").symbol(false), "C7(9)");
        assert_eq!(chord("G/B").symbol(false), "G/B");
        assert_eq!(chord("CmMaj7").symbol(false), "Cm(maj7)");
        assert_eq!(chord("C7(b9,#11)").symbol(false), "C7(b9,#11)");
        assert_eq!(chord("A#").symbol(true), "Bb");
    }

    #[test]
    fn pitch_classes_and_identify() {
        let c7 = chord("C7(9)");
        let names: Vec<_> = c7.pitch_classes().iter().map(|p| p.name(false)).collect();
        assert_eq!(names, ["C", "E", "G", "A#", "D"]);
        let g_b = Chord::identify(&[pc("B"), pc("D"), pc("G")]).unwrap();
        assert_eq!(g_b, chord("G/B"));
        let am = Chord::identify(&[pc("A"), pc("C"), pc("E")]).unwrap();
        assert_eq!(am, chord("Am"));
        assert!(Chord::identify(&[]).is_none());
        assert!(Chord::identify(&[pc("C"), pc("C#"), pc("D")]).is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let c = chord("F#m7(b5,11)/C");
        let json = serde_json::to_string(&c).unwrap();
        let back: Chord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    fn arb_chord() -> impl Strategy<Value = Chord> {
        (
            0u8..12,
            prop::sample::select(ChordQuality::ALL.to_vec()),
            prop::collection::btree_set(prop::sample::select(Tension::ALL.to_vec()), 0..3),
            prop::option::of(0u8..12),
        )
            .prop_map(|(r, q, t, b)| {
                let mut c = Chord::new(PitchClass::new(r).unwrap(), q);
                c.tensions = t;
                c.with_bass(b.and_then(PitchClass::new))
            })
    }

    proptest! {
        #[test]
        fn prop_cifra_roundtrip(c in arb_chord(), flats in any::<bool>()) {
            let text = c.symbol(flats);
            let back = Chord::parse(&text).map_err(|e| TestCaseError::fail(format!("{text}: {e}")))?;
            prop_assert_eq!(back, c);
        }

        #[test]
        fn prop_transpose_preserves_quality(c in arb_chord(), s in -24i32..24) {
            let t = c.transpose(s);
            prop_assert_eq!(t.quality, c.quality);
            prop_assert_eq!(t.transpose(-s), c);
        }
    }
}
