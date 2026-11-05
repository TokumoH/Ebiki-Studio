//! Auxílios de escrita: validação de compasso, preenchimento de pausas e notas fora da escala.

use crate::duration::Duration;
use crate::error::NotationError;
use crate::frac::Frac;
use crate::model::{Event, EventLoc, Score, Voice, VoiceLoc};
use crate::pitch::Pitch;
use serde::{Deserialize, Serialize};
use shared_kernel::music::PitchClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum FillIssue {
    /// Falta tempo (em semibreves).
    Underfilled { missing: Frac },
    /// Sobra tempo (em semibreves).
    Overfilled { excess: Frac },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasureIssue {
    pub loc: VoiceLoc,
    pub issue: FillIssue,
}

/// Verifica a duração de cada voz de cada compasso contra a fórmula vigente. Vozes vazias que não
/// sejam a primeira do compasso são ignoradas (vozes opcionais).
pub fn validate_measures(score: &Score) -> Vec<MeasureIssue> {
    let meters = score.meters();
    let mut out = Vec::new();
    for (pi, p) in score.parts.iter().enumerate() {
        for (mi, m) in p.measures.iter().enumerate() {
            let cap = meters.get(mi).map_or(Frac::ZERO, |x| x.capacity());
            for (vi, v) in m.voices.iter().enumerate() {
                if vi > 0 && v.events.is_empty() {
                    continue;
                }
                let filled = v.filled();
                let issue = match filled.cmp(&cap) {
                    std::cmp::Ordering::Less => FillIssue::Underfilled {
                        missing: cap - filled,
                    },
                    std::cmp::Ordering::Greater => FillIssue::Overfilled {
                        excess: filled - cap,
                    },
                    std::cmp::Ordering::Equal => continue,
                };
                out.push(MeasureIssue {
                    loc: VoiceLoc::new(pi, mi, vi),
                    issue,
                });
            }
        }
    }
    out
}

/// Pausas que completam a voz até `capacity` (vazio se cheia ou excedida).
pub fn rests_to_fill(voice: &Voice, capacity: Frac) -> Vec<Duration> {
    let filled = voice.filled();
    if filled >= capacity {
        return Vec::new();
    }
    Duration::decompose(capacity - filled).0
}

/// Completa a voz com pausas (ids novos). Retorna quantas pausas foram inseridas.
pub fn fill_rests(score: &mut Score, loc: VoiceLoc) -> Result<usize, NotationError> {
    let cap = score.meter_at(loc.measure).capacity();
    let rests = rests_to_fill(score.voice(loc)?, cap);
    let n = rests.len();
    let mut events = Vec::with_capacity(n);
    for d in rests {
        let mut e = Event::rest(d);
        e.id = score.alloc_event_id();
        events.push(e);
    }
    score.voice_mut(loc)?.events.extend(events);
    Ok(n)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutOfScaleNote {
    pub event: EventLoc,
    pub note_index: usize,
    pub pitch: Pitch,
}

/// Notas cuja classe de altura não está em `scale` (destaque visual de notas fora da escala).
pub fn out_of_scale_notes(score: &Score, scale: &[PitchClass]) -> Vec<OutOfScaleNote> {
    scan(score, |_, pc| !scale.contains(&pc))
}

/// Notas fora da escala diatônica da armadura de cada compasso.
pub fn out_of_key_notes(score: &Score) -> Vec<OutOfScaleNote> {
    scan(score, |key, pc| !key.diatonic_pitch_classes().contains(&pc))
}

fn scan(
    score: &Score,
    outside: impl Fn(&crate::pitch::KeySignature, PitchClass) -> bool,
) -> Vec<OutOfScaleNote> {
    let mut out = Vec::new();
    for (pi, p) in score.parts.iter().enumerate() {
        if p.percussion {
            continue;
        }
        for (mi, m) in p.measures.iter().enumerate() {
            for (vi, v) in m.voices.iter().enumerate() {
                for (ei, e) in v.events.iter().enumerate() {
                    for (ni, n) in e.notes().iter().enumerate() {
                        if outside(&m.key, n.pitch.pitch_class()) {
                            out.push(OutOfScaleNote {
                                event: EventLoc {
                                    voice: VoiceLoc::new(pi, mi, vi),
                                    index: ei,
                                },
                                note_index: ni,
                                pitch: n.pitch,
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::NoteValue;
    use crate::model::*;
    use crate::pitch::{KeySignature, Step};

    fn score(events: Vec<Event>) -> Score {
        let mut s = Score::new("T", 120.0, Meter::COMMON).unwrap();
        let mut m = Measure::new(KeySignature::c_major());
        m.voices[0].events = events;
        s.push_part(Part {
            name: "P".into(),
            program: 0,
            percussion: false,
            staves: vec![Staff::standard(Clef::Treble)],
            measures: vec![m],
            spanners: vec![],
        })
        .unwrap();
        s
    }

    fn p(step: Step, alter: i8) -> Pitch {
        Pitch::new(step, alter, 4).unwrap()
    }

    #[test]
    fn detects_under_and_overfilled() {
        let s = score(vec![Event::note(
            Duration::new(NoteValue::Half),
            p(Step::C, 0),
        )]);
        let issues = validate_measures(&s);
        assert_eq!(
            issues,
            vec![MeasureIssue {
                loc: VoiceLoc::new(0, 0, 0),
                issue: FillIssue::Underfilled {
                    missing: Frac::new(1, 2)
                }
            }]
        );
        let s = score(vec![
            Event::note(Duration::new(NoteValue::Whole), p(Step::C, 0)),
            Event::rest(Duration::new(NoteValue::Eighth)),
        ]);
        assert!(matches!(
            validate_measures(&s)[0].issue,
            FillIssue::Overfilled { excess } if excess == Frac::new(1, 8)
        ));
    }

    #[test]
    fn grace_notes_and_triplets_count_correctly() {
        let trip = Duration::new(NoteValue::Quarter).with_tuplet(crate::duration::Tuplet::TRIPLET);
        let mut grace = Event::note(Duration::new(NoteValue::Eighth), p(Step::D, 0));
        grace.grace = Some(GraceKind::Acciaccatura);
        let s = score(vec![
            grace,
            Event::note(trip, p(Step::C, 0)),
            Event::note(trip, p(Step::C, 0)),
            Event::note(trip, p(Step::C, 0)),
            Event::rest(Duration::new(NoteValue::Half)),
        ]);
        assert!(validate_measures(&s).is_empty());
    }

    #[test]
    fn fill_rests_completes_measure() {
        let mut s = score(vec![Event::note(
            Duration::quarter().dotted(1),
            p(Step::C, 0),
        )]);
        let n = fill_rests(&mut s, VoiceLoc::new(0, 0, 0)).unwrap();
        assert!(n >= 1);
        assert!(validate_measures(&s).is_empty());
        s.check_invariants().unwrap();
        assert_eq!(fill_rests(&mut s, VoiceLoc::new(0, 0, 0)).unwrap(), 0);
    }

    #[test]
    fn out_of_scale_detection() {
        let s = score(vec![
            Event::chord(
                Duration::quarter(),
                vec![Note::new(p(Step::C, 0)), Note::new(p(Step::E, -1))],
            ),
            Event::note(Duration::quarter(), p(Step::F, 1)),
            Event::rest(Duration::new(NoteValue::Half)),
        ]);
        let c_major: Vec<PitchClass> = KeySignature::c_major().diatonic_pitch_classes();
        let found = out_of_scale_notes(&s, &c_major);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].note_index, 1);
        assert_eq!(found[1].event.index, 1);
        assert_eq!(out_of_key_notes(&s), found);
        // Escala de Dó menor (C D Eb F G Ab Bb): só F# fica fora
        let c_minor: Vec<PitchClass> = [0u8, 2, 3, 5, 7, 8, 10]
            .iter()
            .map(|&x| PitchClass::new(x).unwrap())
            .collect();
        assert_eq!(out_of_scale_notes(&s, &c_minor).len(), 1);
    }
}
