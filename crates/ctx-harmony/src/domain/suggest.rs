//! Motor de sugestões harmônicas atrás da porta [`SuggestionProvider`] (pronta para IA — ADR-0011).
//!
//! [`RuleBasedSuggester`] combina harmonia funcional (tônica/subdominante/dominante), cadências,
//! dominantes secundárias, empréstimo modal (pontuação menor) e estatísticas de transição do
//! catálogo. Tudo é calculado em termos **relativos à tônica**, logo as sugestões são invariantes
//! por transposição.

use super::catalog::builtin_catalog;
use super::chord::{Chord, ChordQuality};
use super::error::HarmonyError;
use super::field::diatonic_quality;
use super::progression::{Genre, Progression};
use super::realize::{realize, RealizedChord, TimedChord, VoicingOptions};
use super::roman::{Degree, QualitySpec, RomanChord};
use super::scale::Key;
use serde::{Deserialize, Serialize};
use shared_kernel::music::PitchClass;
use std::collections::HashMap;

/// Por que um acorde foi sugerido (exibido na UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionReason {
    Diatonic,
    AuthenticCadence,
    PlagalCadence,
    DeceptiveCadence,
    DominantResolution,
    CircleOfFifths,
    TritoneSubstitution,
    SecondaryDominant,
    Borrowed,
}

impl SuggestionReason {
    pub const fn label_pt(self) -> &'static str {
        match self {
            Self::Diatonic => "Acorde do campo harmônico",
            Self::AuthenticCadence => "Cadência autêntica (V → I)",
            Self::PlagalCadence => "Cadência plagal (IV → I)",
            Self::DeceptiveCadence => "Cadência deceptiva (V → vi)",
            Self::DominantResolution => "Resolução de dominante",
            Self::CircleOfFifths => "Movimento por quintas",
            Self::TritoneSubstitution => "Resolução por substituto de trítono",
            Self::SecondaryDominant => "Dominante secundária",
            Self::Borrowed => "Empréstimo modal",
        }
    }
}

/// Sugestão de próximo acorde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordSuggestion {
    pub chord: Chord,
    pub roman: RomanChord,
    /// Pontuação normalizada em (0, 1]; o primeiro sempre tem 1.
    pub score: f32,
    pub reason: SuggestionReason,
}

/// Consulta de progressões por tom + gênero (RF-5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionQuery {
    pub key: Key,
    pub genre: Option<Genre>,
    pub limit: usize,
}

/// Progressão sugerida já resolvida no tom pedido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionSuggestion {
    pub progression: Progression,
    pub score: f32,
    pub chords: Vec<Chord>,
}

/// Porta de sugestões. Implementações futuras (ex.: IA local) substituem a baseada em regras.
pub trait SuggestionProvider: Send + Sync {
    /// (a) Tom + gênero → progressões ranqueadas.
    fn suggest_progressions(&self, query: &ProgressionQuery) -> Vec<ProgressionSuggestion>;

    /// (b) Acordes atuais (o último é o corrente) → próximos acordes ranqueados.
    fn suggest_next_chords(
        &self,
        key: Key,
        history: &[Chord],
        limit: usize,
    ) -> Vec<ChordSuggestion>;

    /// (c) Realização em notas MIDI + ticks (padrão: voicing fechado determinístico).
    fn realize(
        &self,
        chords: &[TimedChord],
        options: &VoicingOptions,
    ) -> Result<Vec<RealizedChord>, HarmonyError> {
        realize(chords, options)
    }
}

/// Acorde relativo à tônica: (semitons desde a tônica, família da tríade).
type Rel = (u8, ChordQuality);

/// Reduz a qualidade à família da tríade (estatística robusta: `G7 → C` conta como `G → C`).
fn triad_family(q: ChordQuality) -> ChordQuality {
    use ChordQuality as Q;
    match q {
        Q::Major | Q::Major6 | Q::Major7 | Q::Dominant7 | Q::Power => Q::Major,
        Q::Minor | Q::Minor6 | Q::Minor7 | Q::MinorMajor7 => Q::Minor,
        Q::Diminished | Q::HalfDiminished7 | Q::Diminished7 => Q::Diminished,
        Q::Augmented | Q::AugmentedMajor7 | Q::Augmented7 => Q::Augmented,
        Q::Sus2 | Q::Sus4 | Q::Dominant7Sus4 => Q::Sus4,
    }
}

fn rel(chord: &Chord, key: Key) -> Rel {
    (
        key.tonic.interval_to(chord.root),
        triad_family(chord.quality),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Function {
    Tonic,
    Subdominant,
    Dominant,
}

/// Peso de transição funcional `[de][para]` (T, S, D).
const FUNCTION_TABLE: [[f32; 3]; 3] = [
    [0.4, 0.8, 0.6], // T → T, S, D
    [0.5, 0.4, 0.9], // S → …
    [1.0, 0.2, 0.3], // D → …
];

fn fidx(f: Function) -> usize {
    match f {
        Function::Tonic => 0,
        Function::Subdominant => 1,
        Function::Dominant => 2,
    }
}

fn function_of(chord: &Chord, key: Key) -> Function {
    let degree = Degree::of_pitch(key, chord.root, false);
    if chord.quality.is_dominant_like() {
        return Function::Dominant;
    }
    if degree.accidental() == 0 {
        return match degree.step() {
            1 | 3 | 6 => Function::Tonic,
            2 | 4 => Function::Subdominant,
            _ => Function::Dominant,
        };
    }
    match key.tonic.interval_to(chord.root) {
        3 => Function::Tonic,
        1 | 8 | 10 | 5 => Function::Subdominant,
        _ => Function::Dominant,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Diatonic,
    DiatonicSeventh,
    Secondary,
    Borrowed,
}

struct Candidate {
    roman: RomanChord,
    chord: Chord,
    kind: Kind,
}

/// Motor de sugestões por regras + estatísticas do catálogo.
#[derive(Debug, Clone)]
pub struct RuleBasedSuggester {
    progressions: Vec<Progression>,
    /// (é menor?, de, para) → contagem.
    bigrams: HashMap<(bool, Rel, Rel), u32>,
    /// (é menor?, de) → total de saídas.
    outgoing: HashMap<(bool, Rel), u32>,
}

impl Default for RuleBasedSuggester {
    fn default() -> Self {
        Self::new(builtin_catalog().to_vec())
    }
}

impl RuleBasedSuggester {
    /// Usa as progressões dadas (catálogo + do usuário) como base de ranking e estatística.
    pub fn new(progressions: Vec<Progression>) -> Self {
        let mut bigrams = HashMap::new();
        let mut outgoing = HashMap::new();
        let Some(c) = PitchClass::new(0) else {
            unreachable!("0 é classe de altura válida")
        };
        for p in &progressions {
            let key = Key::new(c, p.mode);
            let Ok(chords) = p.chords_in(key) else {
                continue;
            };
            let minor = p.mode.is_minor();
            let rels: Vec<Rel> = chords.iter().map(|c| rel(c, key)).collect();
            if rels.len() < 2 {
                continue;
            }
            for i in 0..rels.len() {
                let (a, b) = (rels[i], rels[(i + 1) % rels.len()]);
                if a == b {
                    continue;
                }
                *bigrams.entry((minor, a, b)).or_insert(0) += 1;
                *outgoing.entry((minor, a)).or_insert(0) += 1;
            }
        }
        Self {
            progressions,
            bigrams,
            outgoing,
        }
    }

    /// Catálogo embutido + progressões do usuário.
    pub fn with_user(user: &[Progression]) -> Self {
        let mut all = builtin_catalog().to_vec();
        all.extend(user.iter().cloned());
        Self::new(all)
    }

    fn catalog_frequency(&self, minor: bool, from: Rel, to: Rel) -> f32 {
        let total = self.outgoing.get(&(minor, from)).copied().unwrap_or(0);
        if total == 0 {
            return 0.0;
        }
        let n = self.bigrams.get(&(minor, from, to)).copied().unwrap_or(0);
        n as f32 / total as f32
    }

    fn candidates(key: Key) -> Vec<Candidate> {
        let mut out: Vec<Candidate> = Vec::new();
        let push = |roman: RomanChord, kind: Kind, out: &mut Vec<Candidate>| {
            if let Ok(chord) = roman.resolve(key) {
                if !out.iter().any(|c| c.chord == chord) {
                    out.push(Candidate { roman, chord, kind });
                }
            }
        };
        let natural = Degree::nat;
        for step in 1..=7 {
            push(
                RomanChord::new(natural(step), QualitySpec::Diatonic),
                Kind::Diatonic,
                &mut out,
            );
        }
        for step in 1..=7 {
            push(
                RomanChord::new(natural(step), QualitySpec::DiatonicSeventh),
                Kind::DiatonicSeventh,
                &mut out,
            );
        }
        // Dominantes secundárias para graus com tríade maior ou menor.
        for target in 2..=6 {
            let q = diatonic_quality(key.mode, target, false);
            if matches!(q, ChordQuality::Major | ChordQuality::Minor) {
                let mut r =
                    RomanChord::new(natural(5), QualitySpec::Explicit(ChordQuality::Dominant7));
                r.of = Some(natural(target));
                push(r, Kind::Secondary, &mut out);
            }
        }
        // Empréstimo do modo paralelo (e dominante maior da menor harmônica).
        let parallel = Key::new(key.tonic, key.mode.parallel());
        for step in 1..=7 {
            let q = diatonic_quality(parallel.mode, step, false);
            let chord = Chord::new(parallel.degree_pitch(natural(step)), q);
            if q == ChordQuality::Diminished {
                continue;
            }
            push(RomanChord::analyze(&chord, key), Kind::Borrowed, &mut out);
        }
        if key.mode.is_minor() {
            for q in [ChordQuality::Major, ChordQuality::Dominant7] {
                push(
                    RomanChord::new(natural(5), QualitySpec::Explicit(q)),
                    Kind::Borrowed,
                    &mut out,
                );
            }
        }
        out
    }

    fn score(
        &self,
        key: Key,
        current: Option<&Chord>,
        previous: Option<&Chord>,
        cand: &Candidate,
    ) -> (f32, SuggestionReason) {
        let kind_mult = match cand.kind {
            Kind::Diatonic => 1.0,
            Kind::DiatonicSeventh => 0.95,
            Kind::Secondary => 0.6,
            Kind::Borrowed => 0.5,
        };
        let default_reason = match cand.kind {
            Kind::Diatonic | Kind::DiatonicSeventh => SuggestionReason::Diatonic,
            Kind::Secondary => SuggestionReason::SecondaryDominant,
            Kind::Borrowed => SuggestionReason::Borrowed,
        };
        let to_f = function_of(&cand.chord, key);
        let Some(x) = current else {
            let base = match to_f {
                Function::Tonic => 1.0,
                Function::Subdominant => 0.6,
                Function::Dominant => 0.5,
            };
            let tonic_bonus = if cand.chord.root == key.tonic {
                0.5
            } else {
                0.0
            };
            return ((base + tonic_bonus) * kind_mult, default_reason);
        };
        let from_f = function_of(x, key);
        let mut score = FUNCTION_TABLE[fidx(from_f)][fidx(to_f)] * kind_mult;
        let mut best: (f32, SuggestionReason) = (0.0, default_reason);
        let mut bonus = |b: f32, r: SuggestionReason, score: &mut f32| {
            *score += b;
            if b > best.0 {
                best = (b, r);
            }
        };
        let x_degree = Degree::of_pitch(key, x.root, false);
        let c_degree = Degree::of_pitch(key, cand.chord.root, false);
        let x_is_v =
            x_degree.step() == 5 && x_degree.accidental() == 0 && !x.quality.is_minor_like();
        let motion = x.root.interval_to(cand.chord.root);
        let c_is_tonic = cand.chord.root == key.tonic;
        if motion == 5 {
            bonus(0.35, SuggestionReason::CircleOfFifths, &mut score);
            if x.quality.is_dominant_like() || x_is_v {
                let r = if c_is_tonic {
                    SuggestionReason::AuthenticCadence
                } else {
                    SuggestionReason::DominantResolution
                };
                bonus(0.45, r, &mut score);
            }
        }
        if x.quality.is_dominant_like() && motion == 11 {
            bonus(0.3, SuggestionReason::TritoneSubstitution, &mut score);
        }
        if x_is_v && c_degree.step() == 6 && c_degree.accidental() == 0 {
            bonus(0.25, SuggestionReason::DeceptiveCadence, &mut score);
        }
        if x_degree.step() == 4 && c_is_tonic {
            let b = if x.quality.is_minor_like() && !key.mode.is_minor() {
                0.3
            } else {
                0.2
            };
            bonus(b, SuggestionReason::PlagalCadence, &mut score);
        }
        let minor = key.mode.is_minor();
        score += 0.6 * self.catalog_frequency(minor, rel(x, key), rel(&cand.chord, key));
        if previous == Some(&cand.chord) {
            score -= 0.1;
        }
        // Movimento por quintas só substitui o motivo "diatônico"; cadências sempre prevalecem.
        let reason = match best.1 {
            SuggestionReason::CircleOfFifths if default_reason != SuggestionReason::Diatonic => {
                default_reason
            }
            r => r,
        };
        (score.max(0.01), reason)
    }
}

impl SuggestionProvider for RuleBasedSuggester {
    fn suggest_progressions(&self, query: &ProgressionQuery) -> Vec<ProgressionSuggestion> {
        let mut out: Vec<ProgressionSuggestion> = self
            .progressions
            .iter()
            .filter_map(|p| {
                let compat = if p.mode == query.key.mode {
                    1.0
                } else if p.mode.is_minor() == query.key.mode.is_minor() {
                    0.5
                } else {
                    return None;
                };
                let genre_score = match query.genre {
                    Some(g) if p.genres.contains(&g) => 1.0,
                    Some(_) if p.universal => 0.5,
                    Some(_) => return None,
                    None if p.universal => 1.0,
                    None => 0.6,
                };
                // Progressões do usuário ganham um pequeno bônus de desempate.
                let user_bonus = if p.key.is_user() { 0.05 } else { 0.0 };
                let score: f32 = (0.6 * genre_score + 0.4 * compat) * 0.95 + user_bonus;
                let chords = p.chords_in(Key::new(query.key.tonic, p.mode)).ok()?;
                Some(ProgressionSuggestion {
                    progression: p.clone(),
                    score,
                    chords,
                })
            })
            .collect();
        out.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.progression.name.cmp(&b.progression.name))
        });
        out.truncate(query.limit);
        out
    }

    fn suggest_next_chords(
        &self,
        key: Key,
        history: &[Chord],
        limit: usize,
    ) -> Vec<ChordSuggestion> {
        let current = history.last();
        let previous = history.len().checked_sub(2).and_then(|i| history.get(i));
        let mut out: Vec<ChordSuggestion> = Self::candidates(key)
            .into_iter()
            .filter(|c| Some(&c.chord) != current)
            .map(|c| {
                let (score, reason) = self.score(key, current, previous, &c);
                ChordSuggestion {
                    chord: c.chord,
                    roman: c.roman.normalized(key.mode),
                    score,
                    reason,
                }
            })
            .collect();
        out.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.roman.format(key.mode).cmp(&b.roman.format(key.mode)))
        });
        let max = out.first().map_or(1.0, |s| s.score);
        for s in &mut out {
            s.score /= max;
        }
        out.truncate(limit);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::progression::ProgressionKey;
    use crate::domain::scale::Mode;

    fn key(s: &str) -> Key {
        Key::parse(s).unwrap()
    }

    fn chord(s: &str) -> Chord {
        Chord::parse(s).unwrap()
    }

    #[test]
    fn after_g7_in_c_the_tonic_comes_first() {
        let s = RuleBasedSuggester::default();
        let out = s.suggest_next_chords(key("C"), &[chord("Dm7"), chord("G7")], 5);
        assert_eq!(out[0].chord, chord("C"));
        assert_eq!(out[0].reason, SuggestionReason::AuthenticCadence);
        assert!((out[0].score - 1.0).abs() < f32::EPSILON);
        assert!(out.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(out.iter().any(|c| c.chord == chord("Am")));
    }

    #[test]
    fn after_e7_in_a_minor_resolves_to_am() {
        let s = RuleBasedSuggester::default();
        let out = s.suggest_next_chords(key("Am"), &[chord("E7")], 3);
        assert_eq!(out[0].chord.root, PitchClass::parse("A").unwrap());
        assert!(out[0].chord.quality.is_minor_like());
    }

    #[test]
    fn secondary_dominant_resolves_to_its_target() {
        let s = RuleBasedSuggester::default();
        let out = s.suggest_next_chords(key("C"), &[chord("C"), chord("A7")], 3);
        assert_eq!(out[0].chord.root, PitchClass::parse("D").unwrap());
        assert_eq!(out[0].reason, SuggestionReason::DominantResolution);
    }

    #[test]
    fn empty_history_starts_on_tonic() {
        let s = RuleBasedSuggester::default();
        let out = s.suggest_next_chords(key("G"), &[], 3);
        assert_eq!(out[0].chord, chord("G"));
    }

    #[test]
    fn borrowed_chords_are_offered_with_lower_score() {
        let s = RuleBasedSuggester::default();
        let out = s.suggest_next_chords(key("C"), &[chord("C")], 40);
        let borrowed: Vec<_> = out
            .iter()
            .filter(|c| c.reason == SuggestionReason::Borrowed)
            .collect();
        assert!(borrowed.iter().any(|c| c.chord == chord("Fm")));
        assert!(borrowed.iter().any(|c| c.chord == chord("Bb")));
        let f = out.iter().find(|c| c.chord == chord("F")).unwrap();
        let fm = out.iter().find(|c| c.chord == chord("Fm")).unwrap();
        assert!(f.score > fm.score);
        assert!(out
            .iter()
            .any(|c| c.reason == SuggestionReason::SecondaryDominant));
    }

    #[test]
    fn next_chord_suggestions_are_transposition_invariant() {
        let s = RuleBasedSuggester::default();
        for mode in [
            Mode::Ionian,
            Mode::Aeolian,
            Mode::Dorian,
            Mode::HarmonicMinor,
        ] {
            let base_key = Key::new(PitchClass::new(0).unwrap(), mode);
            let hist = [chord("Dm7"), chord("G7")];
            let base = s.suggest_next_chords(base_key, &hist, 20);
            for t in 1..12 {
                let k = base_key.transpose(t);
                let th: Vec<Chord> = hist.iter().map(|c| c.transpose(t)).collect();
                let other = s.suggest_next_chords(k, &th, 20);
                assert_eq!(base.len(), other.len());
                for (a, b) in base.iter().zip(&other) {
                    assert_eq!(a.chord.transpose(t), b.chord, "{mode:?} +{t}");
                    assert_eq!(a.roman, b.roman);
                    assert_eq!(a.score, b.score);
                    assert_eq!(a.reason, b.reason);
                }
            }
        }
    }

    #[test]
    fn progression_suggestions_by_key_and_genre() {
        let s = RuleBasedSuggester::default();
        let out = s.suggest_progressions(&ProgressionQuery {
            key: key("D"),
            genre: Some(Genre::Samba),
            limit: 10,
        });
        assert!(!out.is_empty());
        assert!(out
            .iter()
            .all(|p| p.progression.genres.contains(&Genre::Samba) || p.progression.universal));
        assert!(out[0].progression.genres.contains(&Genre::Samba));
        assert!(out.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(out.iter().all(|p| !p.progression.mode.is_minor()));
        assert!(out.len() <= 10);
        let minor = s.suggest_progressions(&ProgressionQuery {
            key: key("Em"),
            genre: None,
            limit: 100,
        });
        assert!(minor.iter().all(|p| p.progression.mode.is_minor()));
        assert!(minor.iter().any(
            |p| matches!(&p.progression.key, ProgressionKey::Builtin(s) if s == "andalusian")
        ));
    }

    #[test]
    fn progression_suggestions_are_transposition_invariant() {
        let s = RuleBasedSuggester::default();
        let q = |k: Key| ProgressionQuery {
            key: k,
            genre: Some(Genre::Pop),
            limit: 50,
        };
        let base = s.suggest_progressions(&q(key("C")));
        for t in 1..12 {
            let other = s.suggest_progressions(&q(key("C").transpose(t)));
            assert_eq!(base.len(), other.len());
            for (a, b) in base.iter().zip(&other) {
                assert_eq!(a.progression.key, b.progression.key);
                assert_eq!(a.score, b.score);
                let moved: Vec<Chord> = a.chords.iter().map(|c| c.transpose(t)).collect();
                assert_eq!(moved, b.chords);
            }
        }
    }

    #[test]
    fn user_progressions_join_ranking() {
        let user = Progression::new_user(
            "Minha do samba",
            Mode::Ionian,
            [Genre::Samba].into(),
            4,
            "I vi ii V7",
        )
        .unwrap();
        let s = RuleBasedSuggester::with_user(std::slice::from_ref(&user));
        let out = s.suggest_progressions(&ProgressionQuery {
            key: key("C"),
            genre: Some(Genre::Samba),
            limit: 100,
        });
        assert_eq!(
            out[0].progression.key, user.key,
            "bônus de progressão do usuário"
        );
    }

    #[test]
    fn port_default_realization_works_through_trait_object() {
        let p: Box<dyn SuggestionProvider> = Box::new(RuleBasedSuggester::default());
        let r = p
            .realize(
                &[TimedChord {
                    chord: chord("C"),
                    beats: 4,
                }],
                &VoicingOptions::default(),
            )
            .unwrap();
        assert_eq!(r.len(), 1);
    }
}
