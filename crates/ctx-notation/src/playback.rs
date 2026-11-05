//! Partitura → eventos MIDI temporizados para reprodução (soundfont GM) e exportação.
//!
//! Regras: ligaduras de valor (tie) fundem notas; quiálteras usam frações exatas; dinâmicas definem a
//! velocity (com rampas de crescendo/diminuendo); acento +12 e marcato +20; staccato toca metade;
//! apojaturas (acciaccatura antes do tempo, appoggiatura no tempo dividindo a duração);
//! repetições simples (início/fim, sem aninhamento) expandidas.

use crate::frac::Frac;
use crate::model::*;
use serde::{Deserialize, Serialize};
use shared_kernel::time::{Tick, PPQ};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackEvent {
    pub part: usize,
    pub start_tick: Tick,
    pub duration_ticks: Tick,
    pub channel: u8,
    pub program: u8,
    pub key: u8,
    pub velocity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackOptions {
    pub expand_repeats: bool,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            expand_repeats: true,
        }
    }
}

/// Uma execução de compasso na linha do tempo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasurePlay {
    pub measure: usize,
    pub start: Tick,
}

const MAX_PLAYS: usize = 200_000;

/// Ordem de execução dos compassos (expande repetições simples).
pub fn performance_order(score: &Score, expand_repeats: bool) -> Vec<MeasurePlay> {
    let measures = score.reference_measures();
    let meters = score.meters();
    let mut out = Vec::new();
    let mut tick = 0i64;
    let mut i = 0usize;
    let mut section_start = 0usize;
    let mut counts: HashMap<usize, u8> = HashMap::new();
    while i < measures.len() && out.len() < MAX_PLAYS {
        let m = &measures[i];
        if m.repeat_start {
            section_start = i;
        }
        out.push(MeasurePlay {
            measure: i,
            start: Tick(tick),
        });
        tick += meters[i].ticks();
        if let (true, Some(times)) = (expand_repeats, m.repeat_end) {
            let c = counts.entry(i).or_insert(1);
            if *c < times.clamp(1, 16) {
                *c += 1;
                i = section_start;
                continue;
            }
            counts.remove(&i);
            section_start = i + 1;
        }
        i += 1;
    }
    out
}

/// Canal MIDI de cada parte: percussão no canal 10 (índice 9); demais em sequência pulando o 9.
pub fn channels(score: &Score) -> Vec<u8> {
    let melodic: Vec<u8> = (0u8..16).filter(|&c| c != 9).collect();
    let mut next = 0usize;
    score
        .parts
        .iter()
        .map(|p| {
            if p.percussion {
                9
            } else {
                let c = melodic[next % melodic.len()];
                next += 1;
                c
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct Raw {
    start: i64,
    end: i64,
    key: u8,
    accent: u8,
    staccato: bool,
    /// Ligadura de valor aberta (aguarda continuação).
    tied: bool,
}

struct Ramp {
    start: i64,
    end: i64,
    up: bool,
}

/// Renderiza a partitura em eventos de nota ordenados por (início, parte, nota).
pub fn render(score: &Score, opts: &PlaybackOptions) -> Vec<PlaybackEvent> {
    let order = performance_order(score, opts.expand_repeats);
    let chans = channels(score);
    let mut out = Vec::new();
    for (pi, part) in score.parts.iter().enumerate() {
        let mut dyn_marks: Vec<(i64, u8)> = Vec::new();
        let mut occurrences: HashMap<EventId, Vec<(i64, i64)>> = HashMap::new();
        let mut streams: HashMap<usize, Vec<Raw>> = HashMap::new();
        for play in &order {
            let Some(measure) = part.measures.get(play.measure) else {
                continue;
            };
            let mstart = play.start.0;
            for (vi, voice) in measure.voices.iter().enumerate() {
                let stream = streams.entry(vi).or_default();
                let mut open: HashMap<u8, usize> = HashMap::new();
                // ligaduras abertas vindas do compasso anterior (fim == início deste evento)
                for (idx, r) in stream.iter().enumerate() {
                    if r.tied && r.end == mstart {
                        open.insert(r.key, idx);
                    }
                }
                let mut offset = Frac::ZERO;
                let mut graces: Vec<&Event> = Vec::new();
                for ev in &voice.events {
                    if ev.grace.is_some() {
                        graces.push(ev);
                        continue;
                    }
                    let mut start = mstart + offset.to_ticks();
                    let end = mstart + (offset + ev.duration.fraction()).to_ticks();
                    offset += ev.duration.fraction();
                    occurrences.entry(ev.id).or_default().push((start, end));
                    if let Some(d) = ev.dynamic {
                        dyn_marks.push((start, d.velocity()));
                    }
                    // apojaturas
                    let n_graces = graces.len() as i64;
                    for (gi, g) in graces.drain(..).enumerate() {
                        let (gs, ge) = match g.grace {
                            Some(GraceKind::Appoggiatura) => {
                                let half = (end - start) / 2 / n_graces.max(1);
                                let s = start + half * gi as i64;
                                (s, s + half)
                            }
                            _ => {
                                let len = PPQ / 16;
                                let s = (start - len * (n_graces - gi as i64)).max(0);
                                (s, s + len)
                            }
                        };
                        for n in g.notes() {
                            if let Some(k) = n.pitch.midi_note() {
                                stream.push(Raw {
                                    start: gs,
                                    end: ge.max(gs + 1),
                                    key: k.0,
                                    accent: 0,
                                    staccato: false,
                                    tied: false,
                                });
                            }
                        }
                        if matches!(g.grace, Some(GraceKind::Appoggiatura)) {
                            start = ge;
                        }
                    }
                    let accent = if ev.articulations.contains(&Articulation::Marcato) {
                        20
                    } else if ev.articulations.contains(&Articulation::Accent) {
                        12
                    } else {
                        0
                    };
                    let staccato = ev.articulations.contains(&Articulation::Staccato);
                    for n in ev.notes() {
                        let Some(k) = n.pitch.midi_note() else {
                            continue;
                        };
                        let key = k.0;
                        let continued = open.remove(&key).filter(|&idx| stream[idx].end == start);
                        let idx = match continued {
                            Some(idx) => {
                                stream[idx].end = end;
                                stream[idx].tied = n.tie;
                                idx
                            }
                            None => {
                                stream.push(Raw {
                                    start,
                                    end: end.max(start + 1),
                                    key,
                                    accent,
                                    staccato,
                                    tied: n.tie,
                                });
                                stream.len() - 1
                            }
                        };
                        if n.tie {
                            open.insert(key, idx);
                        }
                    }
                }
            }
        }
        dyn_marks.sort_by_key(|d| d.0);
        let ramps: Vec<Ramp> = part
            .spanners
            .iter()
            .filter(|s| s.kind != SpannerKind::Slur)
            .flat_map(|s| {
                let starts = occurrences.get(&s.start).cloned().unwrap_or_default();
                let ends = occurrences.get(&s.end).cloned().unwrap_or_default();
                starts
                    .into_iter()
                    .filter_map(|(st, _)| {
                        ends.iter().find(|(es, _)| *es >= st).map(|&(_, ee)| Ramp {
                            start: st,
                            end: ee,
                            up: s.kind == SpannerKind::Crescendo,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let base_at = |t: i64| -> i32 {
            dyn_marks
                .iter()
                .rev()
                .find(|(dt, _)| *dt <= t)
                .map_or(Dynamic::Mf.velocity(), |d| d.1) as i32
        };
        let vel_at = |t: i64| -> i32 {
            let base = base_at(t);
            for r in &ramps {
                if r.start <= t && t < r.end && r.end > r.start {
                    let v0 = base_at(r.start);
                    let target = dyn_marks
                        .iter()
                        .find(|(dt, v)| *dt >= r.end && ((*v as i32 > v0) == r.up))
                        .map(|d| d.1 as i32)
                        .unwrap_or(if r.up { v0 + 24 } else { v0 - 24 });
                    return v0 + (target - v0) * (t - r.start) as i32 / (r.end - r.start) as i32;
                }
            }
            base
        };
        for raw in streams.into_values().flatten() {
            let mut dur = raw.end - raw.start;
            if raw.staccato {
                dur = (dur / 2).max(1);
            }
            let velocity = (vel_at(raw.start) + raw.accent as i32).clamp(1, 127) as u8;
            out.push(PlaybackEvent {
                part: pi,
                start_tick: Tick(raw.start),
                duration_ticks: Tick(dur),
                channel: chans[pi],
                program: part.program,
                key: raw.key,
                velocity,
            });
        }
    }
    out.sort_by_key(|e| (e.start_tick, e.part, e.key, e.duration_ticks));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::{Duration, NoteValue, Tuplet};
    use crate::pitch::{KeySignature, Pitch, Step};

    fn p(step: Step) -> Pitch {
        Pitch::new(step, 0, 4).unwrap()
    }

    fn one_part(measures: Vec<Measure>) -> Score {
        let mut s = Score::new("T", 120.0, Meter::COMMON).unwrap();
        s.push_part(Part {
            name: "P".into(),
            program: 5,
            percussion: false,
            staves: vec![Staff::standard(Clef::Treble)],
            measures,
            spanners: vec![],
        })
        .unwrap();
        s
    }

    fn measure(events: Vec<Event>) -> Measure {
        let mut m = Measure::new(KeySignature::c_major());
        m.voices[0].events = events;
        m
    }

    #[test]
    fn ties_merge_across_bar_lines() {
        let mut a = Event::note(Duration::new(NoteValue::Half), p(Step::C));
        let mut b = Event::note(Duration::new(NoteValue::Half), p(Step::E));
        b.notes_mut().unwrap()[0].tie = true;
        a.dynamic = Some(Dynamic::F);
        let c = Event::note(Duration::new(NoteValue::Quarter), p(Step::E));
        let s = one_part(vec![
            measure(vec![a, b]),
            measure(vec![
                c,
                Event::rest(Duration::new(NoteValue::Half).dotted(1)),
            ]),
        ]);
        let ev = render(&s, &PlaybackOptions::default());
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].key, 60);
        assert_eq!(ev[0].velocity, Dynamic::F.velocity());
        assert_eq!(ev[0].program, 5);
        assert_eq!(ev[0].channel, 0);
        assert_eq!(ev[1].key, 64);
        assert_eq!(ev[1].start_tick, Tick(1920));
        assert_eq!(ev[1].duration_ticks, Tick(1920 + 960));
    }

    #[test]
    fn tuplets_are_exact() {
        let t = Duration::new(NoteValue::Eighth).with_tuplet(Tuplet::TRIPLET);
        let evs: Vec<Event> = (0..12).map(|_| Event::note(t, p(Step::G))).collect();
        let s = one_part(vec![measure(evs)]);
        let ev = render(&s, &PlaybackOptions::default());
        assert_eq!(ev.len(), 12);
        assert_eq!(ev[1].start_tick, Tick(320));
        assert_eq!(ev[3].start_tick, Tick(960));
        assert_eq!(ev[11].start_tick.0 + ev[11].duration_ticks.0, 3840);
    }

    #[test]
    fn repeats_are_expanded() {
        let mut m1 = measure(vec![Event::note(
            Duration::new(NoteValue::Whole),
            p(Step::C),
        )]);
        m1.repeat_start = true;
        let mut m2 = m1.clone();
        m2.repeat_start = false;
        m2.repeat_end = Some(2);
        m2.voices[0].events[0] = Event::note(Duration::new(NoteValue::Whole), p(Step::D));
        let m3 = measure(vec![Event::note(
            Duration::new(NoteValue::Whole),
            p(Step::E),
        )]);
        let s = one_part(vec![m1, m2, m3]);
        let order: Vec<usize> = performance_order(&s, true)
            .iter()
            .map(|m| m.measure)
            .collect();
        assert_eq!(order, vec![0, 1, 0, 1, 2]);
        let keys: Vec<u8> = render(&s, &PlaybackOptions::default())
            .iter()
            .map(|e| e.key)
            .collect();
        assert_eq!(keys, vec![60, 62, 60, 62, 64]);
        assert_eq!(performance_order(&s, false).len(), 3);
    }

    #[test]
    fn articulations_and_hairpins_shape_velocity() {
        let q = Duration::quarter();
        let mut e1 = Event::note(q, p(Step::C));
        e1.dynamic = Some(Dynamic::P);
        let e2 = Event::note(q, p(Step::D));
        let mut e3 = Event::note(q, p(Step::E));
        e3.articulations = vec![Articulation::Staccato, Articulation::Accent];
        let mut e4 = Event::note(q, p(Step::F));
        e4.dynamic = Some(Dynamic::F);
        let mut s = one_part(vec![measure(vec![e1, e2, e3, e4])]);
        let ids: Vec<EventId> = s.parts[0].events().map(|e| e.id).collect();
        s.parts[0].spanners.push(Spanner {
            kind: SpannerKind::Crescendo,
            start: ids[0],
            end: ids[2],
        });
        let ev = render(&s, &PlaybackOptions::default());
        assert_eq!(ev[0].velocity, Dynamic::P.velocity());
        assert!(ev[1].velocity > ev[0].velocity, "crescendo sobe");
        assert!(ev[2].velocity > ev[1].velocity);
        assert_eq!(ev[2].duration_ticks, Tick(480), "staccato");
        assert_eq!(ev[3].velocity, Dynamic::F.velocity());
    }

    #[test]
    fn grace_notes_and_percussion_channel() {
        let mut g = Event::note(Duration::new(NoteValue::Eighth), p(Step::B));
        g.grace = Some(GraceKind::Acciaccatura);
        let main = Event::note(Duration::new(NoteValue::Whole), p(Step::C));
        let mut s = one_part(vec![measure(vec![g, main])]);
        s.parts[0].percussion = true;
        let ev = render(&s, &PlaybackOptions::default());
        assert_eq!(ev.len(), 2);
        let grace = ev.iter().find(|e| e.key == 71).unwrap();
        assert_eq!(grace.start_tick, Tick(0));
        assert_eq!(grace.duration_ticks, Tick(PPQ / 16));
        assert!(ev.iter().all(|e| e.channel == 9));
    }

    #[test]
    fn channels_skip_drums() {
        let mut s = Score::new("T", 120.0, Meter::COMMON).unwrap();
        let base = one_part(vec![measure(vec![Event::rest(Duration::new(
            NoteValue::Whole,
        ))])])
        .parts[0]
            .clone();
        for _ in 0..11 {
            s.push_part(base.clone()).unwrap();
        }
        let ch = channels(&s);
        assert_eq!(&ch[..10], &[0, 1, 2, 3, 4, 5, 6, 7, 8, 10]);
    }
}
