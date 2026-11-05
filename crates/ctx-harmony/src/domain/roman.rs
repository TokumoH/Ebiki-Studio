//! Graus romanos relativos a um modo (ADR-0007, ADR-0018).
//!
//! - O numeral indica o grau da **escala do modo** (em Lá menor, `VII` = Sol; `bVII` em Dó maior = Sib).
//! - A caixa indica a terça (maiúsculo = maior, minúsculo = menor); o sufixo usa a mesma gramática da
//!   cifra (`7`, `maj7`, `°`, `ø7`, `+`, `sus4`, `(9)`…).
//! - `/V` (numeral) = dominante secundária do grau alvo (`V7/ii`); `/3` (arábico) = baixo no grau 3.
//! - Na forma normalizada, uma qualidade igual à diatônica vira [`QualitySpec::Diatonic`] (ou
//!   `DiatonicSeventh`): é o que vai para o banco como "sem sobrescrita de qualidade".

use super::chord::{paren_list, parse_suffix, Chord, ChordQuality, Tension};
use super::error::HarmonyError;
use super::field::diatonic_quality;
use super::scale::{Key, Mode};
use serde::{Deserialize, Serialize};
use shared_kernel::music::PitchClass;
use std::collections::BTreeSet;

const NUMERALS: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];

/// Grau da escala (1–7) com acidente (−2 a +2) relativo ao modo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "DegreeRaw")]
pub struct Degree {
    step: u8,
    accidental: i8,
}

#[derive(Deserialize)]
struct DegreeRaw {
    step: u8,
    accidental: i8,
}

impl TryFrom<DegreeRaw> for Degree {
    type Error = HarmonyError;
    fn try_from(r: DegreeRaw) -> Result<Self, Self::Error> {
        Degree::new(r.step, r.accidental)
    }
}

impl Degree {
    pub fn new(step: u8, accidental: i8) -> Result<Self, HarmonyError> {
        if !(1..=7).contains(&step) || !(-2..=2).contains(&accidental) {
            return Err(HarmonyError::InvalidDegree);
        }
        Ok(Self { step, accidental })
    }

    pub fn natural(step: u8) -> Result<Self, HarmonyError> {
        Self::new(step, 0)
    }

    /// Grau natural com `step` limitado a 1–7 (uso interno com constantes).
    pub(crate) fn nat(step: u8) -> Self {
        Self {
            step: step.clamp(1, 7),
            accidental: 0,
        }
    }

    pub const fn step(self) -> u8 {
        self.step
    }

    pub const fn accidental(self) -> i8 {
        self.accidental
    }

    fn accidental_text(self) -> String {
        let c = if self.accidental < 0 { "b" } else { "#" };
        c.repeat(self.accidental.unsigned_abs() as usize)
    }

    /// Grau da tonalidade que soa a classe de altura (prefere bemol, ou sustenido se pedido).
    pub fn of_pitch(key: Key, pc: PitchClass, prefer_sharp: bool) -> Degree {
        let order: [i8; 5] = if prefer_sharp {
            [0, 1, -1, 2, -2]
        } else {
            [0, -1, 1, -2, 2]
        };
        let scale = key.scale();
        for acc in order {
            if let Some(i) = scale.iter().position(|p| p.transpose(acc as i32) == pc) {
                return Degree {
                    step: i as u8 + 1,
                    accidental: acc,
                };
            }
        }
        // Inalcançável para escalas heptatônicas (saltos ≤ 3 semitons); mantém total.
        Degree {
            step: 1,
            accidental: key.tonic.interval_to(pc).min(2) as i8,
        }
    }
}

/// Qualidade de um passo: derivada do campo harmônico ou explícita.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QualitySpec {
    /// Tríade diatônica do grau.
    Diatonic,
    /// Tétrade diatônica do grau.
    DiatonicSeventh,
    /// Qualidade explícita (empréstimos, dominantes secundárias, cores).
    Explicit(ChordQuality),
}

/// Acorde relativo à tonalidade, em graus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RomanChord {
    pub degree: Degree,
    pub quality: QualitySpec,
    #[serde(default)]
    pub tensions: BTreeSet<Tension>,
    /// Baixo em grau da tonalidade (acorde invertido), ex.: `V/7` = Sol/Si em Dó.
    #[serde(default)]
    pub bass: Option<Degree>,
    /// Alvo de dominante secundária, ex.: `V7/ii`.
    #[serde(default)]
    pub of: Option<Degree>,
}

fn parse_accidentals(s: &str) -> (i8, &str) {
    let mut acc = 0i8;
    let mut rest = s;
    loop {
        let mut chars = rest.chars();
        match chars.next() {
            Some('b' | '♭') => acc -= 1,
            Some('#' | '♯') => acc += 1,
            _ => return (acc, rest),
        }
        rest = chars.as_str();
    }
}

/// Lê `[acidentes]NUMERAL` e retorna (grau, minúsculo?, resto).
fn parse_numeral(s: &str) -> Option<(Degree, bool, &str)> {
    let (acc, rest) = parse_accidentals(s);
    let len = rest
        .find(|c: char| !matches!(c, 'I' | 'V' | 'i' | 'v'))
        .unwrap_or(rest.len());
    let numeral = &rest[..len];
    let lower = numeral.chars().all(|c| c.is_ascii_lowercase());
    let upper = numeral.chars().all(|c| c.is_ascii_uppercase());
    if numeral.is_empty() || !(lower || upper) {
        return None;
    }
    let up = numeral.to_ascii_uppercase();
    let step = NUMERALS.iter().position(|n| *n == up)? as u8 + 1;
    Some((Degree::new(step, acc).ok()?, lower, &rest[len..]))
}

impl RomanChord {
    pub fn new(degree: Degree, quality: QualitySpec) -> Self {
        Self {
            degree,
            quality,
            tensions: BTreeSet::new(),
            bass: None,
            of: None,
        }
    }

    /// Lê um grau romano (qualidade sempre explícita; use [`RomanChord::normalized`]).
    pub fn parse(text: &str) -> Result<RomanChord, HarmonyError> {
        let err = || HarmonyError::InvalidRoman(text.to_owned());
        let s = text.trim();
        let (main, after) = match s.split_once('/') {
            Some((m, a)) => (m, Some(a)),
            None => (s, None),
        };
        let (degree, lower, suffix) = parse_numeral(main).ok_or_else(err)?;
        let start = if lower {
            ChordQuality::Minor
        } else {
            ChordQuality::Major
        };
        let (quality, tensions) = parse_suffix(suffix, start).ok_or_else(err)?;
        let mut rc = RomanChord::new(degree, QualitySpec::Explicit(quality));
        rc.tensions = tensions;
        if let Some(a) = after {
            let (acc, digits) = parse_accidentals(a);
            if let Ok(step) = digits.parse::<u8>() {
                rc.bass = Some(Degree::new(step, acc).map_err(|_| err())?);
            } else {
                let (target, _, rest) = parse_numeral(a).ok_or_else(err)?;
                if !rest.is_empty() {
                    return Err(err());
                }
                rc.of = Some(target);
            }
        }
        Ok(rc)
    }

    /// Regras de consistência do passo.
    pub fn validate(&self) -> Result<(), HarmonyError> {
        let diatonic = matches!(
            self.quality,
            QualitySpec::Diatonic | QualitySpec::DiatonicSeventh
        );
        if diatonic && self.degree.accidental() != 0 && self.of.is_none() {
            return Err(HarmonyError::InvalidStep(
                "grau alterado precisa de qualidade explícita".into(),
            ));
        }
        if self.bass.is_some() && self.of.is_some() {
            return Err(HarmonyError::InvalidStep(
                "não combine baixo e dominante secundária no mesmo acorde".into(),
            ));
        }
        Ok(())
    }

    /// Qualidade concreta no modo (derivando a diatônica quando necessário).
    pub fn explicit_quality(&self, mode: Mode) -> ChordQuality {
        let reference = if self.of.is_some() {
            Mode::Ionian
        } else {
            mode
        };
        match self.quality {
            QualitySpec::Explicit(q) => q,
            QualitySpec::Diatonic => diatonic_quality(reference, self.degree.step(), false),
            QualitySpec::DiatonicSeventh => diatonic_quality(reference, self.degree.step(), true),
        }
    }

    /// Forma normalizada: qualidade igual à diatônica vira `Diatonic`/`DiatonicSeventh`.
    pub fn normalized(mut self, mode: Mode) -> RomanChord {
        if self.of.is_none() && self.degree.accidental() == 0 {
            if let QualitySpec::Explicit(q) = self.quality {
                if q == diatonic_quality(mode, self.degree.step(), false) {
                    self.quality = QualitySpec::Diatonic;
                } else if q == diatonic_quality(mode, self.degree.step(), true) {
                    self.quality = QualitySpec::DiatonicSeventh;
                }
            }
        }
        self
    }

    /// Resolve o acorde absoluto na tonalidade.
    pub fn resolve(&self, key: Key) -> Result<Chord, HarmonyError> {
        self.validate()?;
        let quality = self.explicit_quality(key.mode);
        let root = match self.of {
            Some(target) => Key::major(key.degree_pitch(target)).degree_pitch(self.degree),
            None => key.degree_pitch(self.degree),
        };
        let mut chord = Chord::new(root, quality);
        chord.tensions = self.tensions.clone();
        Ok(chord.with_bass(self.bass.map(|b| key.degree_pitch(b))))
    }

    /// Grafia preferida do acorde resolvido: bemóis para graus abaixados, sustenidos para
    /// elevados; senão a preferência da tonalidade.
    pub fn spelling_flats(&self, key: Key) -> bool {
        match self.degree.accidental() {
            a if a < 0 => true,
            a if a > 0 => false,
            _ => key.prefer_flats(),
        }
    }

    /// Análise de um acorde absoluto em graus (qualidade explícita, sem dominante secundária).
    pub fn analyze(chord: &Chord, key: Key) -> RomanChord {
        let prefer_sharp = matches!(
            chord.quality,
            ChordQuality::Diminished | ChordQuality::Diminished7
        );
        let mut rc = RomanChord::new(
            Degree::of_pitch(key, chord.root, prefer_sharp),
            QualitySpec::Explicit(chord.quality),
        );
        rc.tensions = chord.tensions.clone();
        rc.bass = chord.bass.map(|b| Degree::of_pitch(key, b, false));
        rc
    }

    /// Escreve em graus romanos para o modo.
    pub fn format(&self, mode: Mode) -> String {
        let q = self.explicit_quality(mode);
        let numeral = NUMERALS[(self.degree.step() - 1) as usize];
        let mut s = self.degree.accidental_text();
        if q.is_minor_like() {
            s.push_str(&numeral.to_ascii_lowercase());
        } else {
            s.push_str(numeral);
        }
        let (base, alter) = q.roman_parts();
        s.push_str(base);
        s.push_str(&paren_list(alter, &self.tensions));
        if let Some(b) = self.bass {
            s.push('/');
            s.push_str(&b.accidental_text());
            s.push_str(&b.step().to_string());
        }
        if let Some(t) = self.of {
            s.push('/');
            s.push_str(&t.accidental_text());
            let n = NUMERALS[(t.step() - 1) as usize];
            let target_minor =
                t.accidental() == 0 && diatonic_quality(mode, t.step(), false).is_minor_like();
            if target_minor {
                s.push_str(&n.to_ascii_lowercase());
            } else {
                s.push_str(n);
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn key(s: &str) -> Key {
        Key::parse(s).unwrap()
    }

    fn resolve(r: &str, k: &str) -> String {
        let k = key(k);
        let rc = RomanChord::parse(r).unwrap_or_else(|e| panic!("{r}: {e}"));
        rc.resolve(k).unwrap().symbol(rc.spelling_flats(k))
    }

    #[test]
    fn resolves_major_key_numerals() {
        assert_eq!(resolve("I", "C"), "C");
        assert_eq!(resolve("vi", "C"), "Am");
        assert_eq!(resolve("V7", "C"), "G7");
        assert_eq!(resolve("ii7", "C"), "Dm7");
        assert_eq!(resolve("bVII", "C"), "Bb");
        assert_eq!(resolve("V/V", "C"), "D");
        assert_eq!(resolve("V7/ii", "C"), "A7");
        assert_eq!(resolve("viiø7", "C"), "Bm7(b5)");
        assert_eq!(resolve("#i°7", "C"), "C#dim7");
        assert_eq!(resolve("bII7", "F"), "Gb7");
        assert_eq!(resolve("bVI", "E"), "C");
        assert_eq!(resolve("Imaj7(9)", "C"), "Cmaj7(9)");
        assert_eq!(resolve("V/7", "C"), "G/B");
        assert_eq!(resolve("iv", "C"), "Fm");
        assert_eq!(resolve("V7sus4", "D"), "A7sus4");
    }

    #[test]
    fn resolves_minor_key_numerals() {
        assert_eq!(resolve("i", "Am"), "Am");
        assert_eq!(resolve("III", "Am"), "C");
        assert_eq!(resolve("VII", "Am"), "G");
        assert_eq!(resolve("VI", "Am"), "F");
        assert_eq!(resolve("iv", "Am"), "Dm");
        assert_eq!(resolve("ii°", "Am"), "Bdim");
        assert_eq!(resolve("V", "Am"), "E");
        assert_eq!(resolve("V7", "Am"), "E7");
        assert_eq!(resolve("imaj7", "Am"), "Am(maj7)");
        assert_eq!(resolve("i", "Dm"), "Dm");
        assert_eq!(resolve("VI", "Dm"), "Bb");
    }

    #[test]
    fn analyzes_and_formats_in_minor() {
        let am = key("Am");
        let fmt = |c: &str| RomanChord::analyze(&Chord::parse(c).unwrap(), am).format(am.mode);
        assert_eq!(fmt("Am"), "i");
        assert_eq!(fmt("C"), "III");
        assert_eq!(fmt("E7"), "V7");
        assert_eq!(fmt("Em"), "v");
        assert_eq!(fmt("G#dim7"), "#vii°7");
        assert_eq!(fmt("Bm7(b5)"), "iiø7");
        assert_eq!(fmt("F/A"), "VI/1");
        let ahm = key("A harmonic_minor");
        let r = RomanChord::analyze(&Chord::parse("G#dim").unwrap(), ahm);
        assert_eq!(r.format(ahm.mode), "vii°");
    }

    #[test]
    fn formats_secondary_and_borrowed() {
        let c = Mode::Ionian;
        assert_eq!(RomanChord::parse("V7/ii").unwrap().format(c), "V7/ii");
        assert_eq!(RomanChord::parse("V/V").unwrap().format(c), "V/V");
        assert_eq!(RomanChord::parse("bVII").unwrap().format(c), "bVII");
        assert_eq!(RomanChord::parse("bVImaj7").unwrap().format(c), "bVImaj7");
        assert_eq!(RomanChord::parse("ii7(9)").unwrap().format(c), "ii7(9)");
    }

    #[test]
    fn normalization_marks_diatonic_quality() {
        let m = Mode::Ionian;
        let n = |t: &str| RomanChord::parse(t).unwrap().normalized(m).quality;
        assert_eq!(n("vi"), QualitySpec::Diatonic);
        assert_eq!(n("V7"), QualitySpec::DiatonicSeventh);
        assert_eq!(n("ii7"), QualitySpec::DiatonicSeventh);
        assert_eq!(n("II7"), QualitySpec::Explicit(ChordQuality::Dominant7));
        assert_eq!(n("bVII"), QualitySpec::Explicit(ChordQuality::Major));
        let am = Mode::Aeolian;
        assert_eq!(
            RomanChord::parse("V7").unwrap().normalized(am).quality,
            QualitySpec::Explicit(ChordQuality::Dominant7)
        );
    }

    #[test]
    fn rejects_invalid_numerals() {
        for bad in [
            "", "X", "Vi", "VIII", "bbbV", "V/", "V/H", "V/8", "V/vi7", "Ixyz",
        ] {
            assert!(RomanChord::parse(bad).is_err(), "{bad} deveria falhar");
        }
        let mut r = RomanChord::new(Degree::new(7, -1).unwrap(), QualitySpec::Diatonic);
        assert!(r.validate().is_err());
        r.quality = QualitySpec::Explicit(ChordQuality::Major);
        assert!(r.validate().is_ok());
        r.bass = Some(Degree::natural(3).unwrap());
        r.of = Some(Degree::natural(5).unwrap());
        assert!(r.validate().is_err());
        assert!(Degree::new(0, 0).is_err());
        assert!(Degree::new(3, 3).is_err());
        assert!(serde_json::from_str::<Degree>(r#"{"step":9,"accidental":0}"#).is_err());
    }

    proptest! {
        /// Analisar e formatar em graus e voltar resolve o mesmo acorde, em qualquer tom e modo.
        #[test]
        fn prop_roman_roundtrip(root in 0u8..12, tonic in 0u8..12, mi in 0usize..21,
                                qi in 0usize..18, bass in prop::option::of(0u8..12)) {
            let key = Key::new(PitchClass::new(tonic).unwrap(), Mode::ALL[mi]);
            let chord = Chord::new(PitchClass::new(root).unwrap(), ChordQuality::ALL[qi])
                .with_bass(bass.and_then(PitchClass::new));
            let rc = RomanChord::analyze(&chord, key);
            let text = rc.format(key.mode);
            let back = RomanChord::parse(&text)
                .map_err(|e| TestCaseError::fail(format!("{text}: {e}")))?;
            prop_assert_eq!(back.resolve(key).unwrap(), chord.clone());
            prop_assert_eq!(back.normalized(key.mode).resolve(key).unwrap(), chord);
        }
    }
}
