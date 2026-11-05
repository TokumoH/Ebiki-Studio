//! Arranjo de notas temporizadas em compassos e vozes (serviço de domínio).
//!
//! Usado por "acordes → partitura" e pela importação de MIDI: quantiza, agrupa notas simultâneas em
//! acordes, distribui sobreposições em vozes (até 4), divide nas barras de compasso com ligaduras,
//! decompõe durações em figuras e completa com pausas.

use crate::duration::Duration;
use crate::error::NotationError;
use crate::frac::Frac;
use crate::model::*;
use crate::pitch::{KeySignature, Pitch};
use crate::tab;
use serde::{Deserialize, Serialize};
use shared_kernel::time::{Tick, PPQ};
use std::collections::BTreeMap;

/// Nota com posição e duração absolutas (ticks, PPQ do projeto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimedNote {
    pub start: Tick,
    pub duration: Tick,
    pub key: u8,
    pub velocity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrangeOptions {
    /// Grade de quantização em ticks (0 = sem quantização).
    pub grid: i64,
    /// Máximo de vozes no compasso (dividido entre as pautas).
    pub max_voices: usize,
    /// Com 2 pautas: notas ≥ este valor vão para a pauta superior.
    pub split_point: u8,
    /// Gera marcas de dinâmica a partir da velocity.
    pub dynamics: bool,
}

impl Default for ArrangeOptions {
    fn default() -> Self {
        Self {
            grid: PPQ / 4,
            max_voices: MAX_VOICES,
            split_point: 60,
            dynamics: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrangeReport {
    /// Notas descartadas (vozes esgotadas ou fora da extensão da partitura).
    pub dropped_notes: usize,
}

/// Estrutura de compassos (só atributos, uma voz vazia) cobrindo até `end`, aplicando mudanças de
/// fórmula/armadura na primeira barra em ou após o tick indicado. O 1º compasso herda `default_meter`
/// (fórmula padrão da partitura), salvo mudança no tick 0.
pub fn build_template(
    default_meter: Meter,
    meter_changes: &[(Tick, Meter)],
    key_changes: &[(Tick, KeySignature)],
    end: Tick,
) -> Result<Vec<Measure>, NotationError> {
    let mut out: Vec<Measure> = Vec::new();
    let mut meter = default_meter;
    let mut key = KeySignature::c_major();
    let mut t = 0i64;
    let mut mi = meter_changes.iter().peekable();
    let mut ki = key_changes.iter().peekable();
    loop {
        let mut changed_meter = None;
        while let Some((tick, m)) = mi.peek() {
            if tick.0 <= t {
                Meter::new(m.numerator, m.denominator)?;
                if *m != meter {
                    meter = *m;
                    changed_meter = Some(*m);
                }
                mi.next();
            } else {
                break;
            }
        }
        while let Some((tick, k)) = ki.peek() {
            if tick.0 <= t {
                key = KeySignature::new(k.fifths, k.mode)?;
                ki.next();
            } else {
                break;
            }
        }
        let mut m = Measure::new(key);
        m.meter = changed_meter;
        out.push(m);
        if out.len() > MAX_MEASURES {
            return Err(NotationError::LimitExceeded("compassos"));
        }
        t += meter.ticks();
        if t >= end.0 {
            break;
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
struct Group {
    start: i64,
    end: i64,
    keys: Vec<u8>,
    velocity: u8,
}

/// Distribui `notes` numa parte com a estrutura `template`.
pub fn arrange_part(
    template: &[Measure],
    default_meter: Meter,
    notes: &[TimedNote],
    name: impl Into<String>,
    program: u8,
    staves: Vec<Staff>,
    opts: &ArrangeOptions,
) -> Result<(Part, ArrangeReport), NotationError> {
    if template.is_empty() {
        return Err(NotationError::LastMeasure);
    }
    let n_staves = staves.len().clamp(1, MAX_STAVES);
    let mut report = ArrangeReport::default();
    // layout
    let mut meters = Vec::with_capacity(template.len());
    let mut cur = default_meter;
    for m in template {
        if let Some(x) = m.meter {
            cur = x;
        }
        meters.push(cur);
    }
    let mut starts = vec![0i64];
    for m in &meters {
        let last = *starts.last().expect("não vazio");
        starts.push(last + m.ticks());
    }
    let total_end = *starts.last().expect("não vazio");

    // quantização + agrupamento por pauta
    let mut by_staff: Vec<BTreeMap<(i64, i64), Group>> = vec![BTreeMap::new(); n_staves];
    for n in notes {
        if n.key > 127 {
            report.dropped_notes += 1;
            continue;
        }
        let start = n.start.0.clamp(0, i64::MAX / 4);
        let (mut s, mut e) = (start, start + n.duration.0.clamp(1, i64::MAX / 4));
        let grid = opts.grid.clamp(0, PPQ * 16);
        if grid > 0 {
            s = round_to(s, grid);
            e = round_to(e, grid).max(s + grid);
        }
        e = e.min(total_end);
        if s >= total_end || e <= s {
            report.dropped_notes += 1;
            continue;
        }
        let staff = if n_staves > 1 && n.key < opts.split_point {
            1
        } else {
            0
        };
        let g = by_staff[staff].entry((s, e)).or_insert_with(|| Group {
            start: s,
            end: e,
            keys: Vec::new(),
            velocity: 0,
        });
        if !g.keys.contains(&n.key) {
            if g.keys.len() >= MAX_NOTES_PER_CHORD {
                report.dropped_notes += 1;
                continue;
            }
            g.keys.push(n.key);
        }
        g.velocity = g.velocity.max(n.velocity);
    }

    let voices_per_staff = (opts.max_voices.clamp(1, MAX_VOICES) / n_staves).max(1);
    // (pauta, grupos não sobrepostos) por voz
    let mut voice_streams: Vec<(u8, Vec<Group>)> = Vec::new();
    for (staff, groups) in by_staff.into_iter().enumerate() {
        let mut ordered: Vec<Group> = groups.into_values().collect();
        ordered.sort_by_key(|g| (g.start, std::cmp::Reverse(g.end)));
        let mut lanes: Vec<Vec<Group>> = vec![Vec::new()];
        for mut g in ordered {
            g.keys.sort_unstable();
            let free = lanes
                .iter()
                .position(|l| l.last().is_none_or(|x| x.end <= g.start));
            match free {
                Some(i) => lanes[i].push(g),
                None if lanes.len() < voices_per_staff => lanes.push(vec![g]),
                None => report.dropped_notes += g.keys.len(),
            }
        }
        for l in lanes {
            voice_streams.push((staff as u8, l));
        }
    }

    let mut measures: Vec<Measure> = template
        .iter()
        .map(|m| Measure {
            voices: Vec::new(),
            ..m.clone()
        })
        .collect();
    let mut first_of_staff = [true; MAX_STAVES];
    for (staff, groups) in &voice_streams {
        let is_first = std::mem::replace(&mut first_of_staff[*staff as usize], false);
        let tab_cfg = staves.get(*staff as usize).and_then(|s| s.tab.clone());
        let mut last_dyn: Option<Dynamic> = None;
        let mut gi = 0usize;
        for (mi, m) in measures.iter_mut().enumerate() {
            let (ms, me) = (starts[mi], starts[mi + 1]);
            let mut voice = Voice::new(*staff);
            let mut cursor = ms;
            while gi < groups.len() && groups[gi].end <= ms {
                gi += 1;
            }
            let mut k = gi;
            while k < groups.len() && groups[k].start < me {
                let g = &groups[k];
                let seg_s = g.start.max(ms);
                let seg_e = g.end.min(me);
                push_rests(&mut voice, cursor, seg_s);
                let pieces = Duration::decompose(Frac::from_ticks(seg_e - seg_s)).0;
                let n_pieces = pieces.len();
                for (pi, d) in pieces.into_iter().enumerate() {
                    let tied = pi + 1 < n_pieces || seg_e < g.end;
                    let mut notes: Vec<Note> = g
                        .keys
                        .iter()
                        .map(|&k| Note {
                            tie: tied,
                            ..Note::new(Pitch::from_midi(k, &m.key))
                        })
                        .collect();
                    if let Some(cfg) = &tab_cfg {
                        if let Some(pos) = tab::assign_chord(cfg, &g.keys) {
                            for (n, p) in notes.iter_mut().zip(pos) {
                                n.tab = Some(p);
                            }
                        }
                    }
                    let mut ev = Event::chord(d, notes);
                    if opts.dynamics && is_first && pi == 0 && seg_s == g.start {
                        let dy = Dynamic::from_velocity(g.velocity);
                        if last_dyn != Some(dy) {
                            ev.dynamic = Some(dy);
                            last_dyn = Some(dy);
                        }
                    }
                    voice.events.push(ev);
                }
                cursor = seg_e;
                k += 1;
            }
            if is_first || !voice.events.is_empty() {
                push_rests(&mut voice, cursor, me);
            }
            m.voices.push(voice);
        }
    }
    // pautas sem nenhuma voz recebem pausas
    for (staff, first) in first_of_staff.iter().enumerate().take(n_staves) {
        if *first {
            for (mi, m) in measures.iter_mut().enumerate() {
                let mut v = Voice::new(staff as u8);
                push_rests(&mut v, starts[mi], starts[mi + 1]);
                m.voices.push(v);
            }
        }
    }
    // remove vozes vazias excedentes no fim de cada compasso (mantém a ordem das demais)
    for m in &mut measures {
        while m.voices.len() > 1 && m.voices.last().is_some_and(|v| v.events.is_empty()) {
            m.voices.pop();
        }
    }
    let part = Part {
        name: name.into(),
        program: program.min(127),
        percussion: false,
        staves: staves.into_iter().take(MAX_STAVES).collect(),
        measures,
        spanners: Vec::new(),
    };
    Ok((part, report))
}

fn round_to(v: i64, grid: i64) -> i64 {
    ((v + grid / 2).div_euclid(grid)) * grid
}

fn push_rests(voice: &mut Voice, from: i64, to: i64) {
    if to > from {
        for d in Duration::decompose(Frac::from_ticks(to - from)).0 {
            voice.events.push(Event::rest(d));
        }
    }
}

/// Acorde realizado (ex.: vindo do contexto Harmony, já convertido pela camada de aplicação).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordSlot {
    pub start_tick: Tick,
    pub duration_ticks: Tick,
    pub notes: Vec<u8>,
}

fn slots_to_notes(slots: &[ChordSlot]) -> Vec<TimedNote> {
    slots
        .iter()
        .flat_map(|s| {
            s.notes.iter().map(|&k| TimedNote {
                start: s.start_tick,
                duration: s.duration_ticks,
                key: k,
                velocity: Dynamic::Mf.velocity(),
            })
        })
        .collect()
}

/// Gera uma parte de piano (pauta dupla, divisão no Dó central) a partir de acordes realizados,
/// usando a estrutura de compassos da partitura (ou criando-a, se a partitura ainda não tem partes).
pub fn chords_to_part(
    score: &Score,
    slots: &[ChordSlot],
    name: &str,
) -> Result<(Part, ArrangeReport), NotationError> {
    let template = if score.parts.is_empty() {
        let end = slots
            .iter()
            .map(|s| s.start_tick.0 + s.duration_ticks.0)
            .max()
            .unwrap_or(1);
        build_template(score.default_meter(), &[], &[], Tick(end))?
    } else {
        score.reference_measures().to_vec()
    };
    arrange_part(
        &template,
        score.default_meter(),
        &slots_to_notes(slots),
        name,
        0,
        vec![Staff::standard(Clef::Treble), Staff::standard(Clef::Bass)],
        &ArrangeOptions {
            dynamics: false,
            ..ArrangeOptions::default()
        },
    )
}

/// Cria uma partitura nova só com a parte de piano dos acordes.
pub fn chords_to_score(
    title: &str,
    bpm: f64,
    meter: Meter,
    key: KeySignature,
    slots: &[ChordSlot],
) -> Result<Score, NotationError> {
    let mut score = Score::new(title, bpm, meter)?;
    let end = slots
        .iter()
        .map(|s| s.start_tick.0 + s.duration_ticks.0)
        .max()
        .unwrap_or(1);
    let template = build_template(meter, &[], &[(Tick(0), key)], Tick(end))?;
    let (part, _) = arrange_part(
        &template,
        meter,
        &slots_to_notes(slots),
        "Piano",
        0,
        vec![Staff::standard(Clef::Treble), Staff::standard(Clef::Bass)],
        &ArrangeOptions {
            dynamics: false,
            ..ArrangeOptions::default()
        },
    )?;
    score.push_part(part)?;
    Ok(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::validate_measures;
    use crate::pitch::Mode;

    fn tn(start: i64, dur: i64, key: u8) -> TimedNote {
        TimedNote {
            start: Tick(start),
            duration: Tick(dur),
            key,
            velocity: 80,
        }
    }

    #[test]
    fn template_applies_changes_on_bar_lines() {
        let t = build_template(
            Meter::COMMON,
            &[
                (Tick(3840), Meter::new(3, 4).unwrap()),
                (Tick(5000), Meter::COMMON),
            ],
            &[(Tick(0), KeySignature::new(2, Mode::Major).unwrap())],
            Tick(3840 * 4),
        )
        .unwrap();
        assert_eq!(t[0].meter, None, "herda a fórmula padrão");
        assert_eq!(t[1].meter, Some(Meter::new(3, 4).unwrap()));
        // 5000 cai dentro do compasso 2 (3840..6720): aplicada na barra seguinte
        assert_eq!(t[2].meter, Some(Meter::COMMON));
        assert_eq!(t[3].meter, None);
        assert!(t.iter().all(|m| m.key.fifths == 2));
    }

    #[test]
    fn chords_to_score_builds_valid_piano_part() {
        // C (C3 E4 G4 C5) por 1 compasso, depois G (G2 D4 G4 B4) por 2 tempos + F 6 tempos (atravessa a barra)
        let slots = vec![
            ChordSlot {
                start_tick: Tick(0),
                duration_ticks: Tick(3840),
                notes: vec![48, 64, 67, 72],
            },
            ChordSlot {
                start_tick: Tick(3840),
                duration_ticks: Tick(1920),
                notes: vec![43, 62, 67, 71],
            },
            ChordSlot {
                start_tick: Tick(5760),
                duration_ticks: Tick(5760),
                notes: vec![41, 65, 69, 72],
            },
        ];
        let s =
            chords_to_score("Prog", 90.0, Meter::COMMON, KeySignature::c_major(), &slots).unwrap();
        s.check_invariants().unwrap();
        assert_eq!(s.measure_count(), 3);
        assert!(
            validate_measures(&s).is_empty(),
            "{:?}",
            validate_measures(&s)
        );
        let p = &s.parts[0];
        assert_eq!(p.staves.len(), 2);
        let m0 = &p.measures[0];
        let upper: Vec<&Voice> = m0.voices.iter().filter(|v| v.staff == 0).collect();
        let lower: Vec<&Voice> = m0.voices.iter().filter(|v| v.staff == 1).collect();
        assert_eq!(upper[0].events[0].notes().len(), 3);
        assert_eq!(lower[0].events[0].notes()[0].pitch.midi(), 48);
        // F atravessa a barra 2→3: última figura do compasso 2 ligada
        let m1_upper = p.measures[1].voices.iter().find(|v| v.staff == 0).unwrap();
        assert!(m1_upper.events.last().unwrap().notes()[0].tie);
        let m2_upper = p.measures[2].voices.iter().find(|v| v.staff == 0).unwrap();
        assert_eq!(m2_upper.events.len(), 1, "F ocupa o compasso 3 inteiro");
        assert!(!m2_upper.events[0].notes()[0].tie, "fim da ligadura");
        assert_eq!(m2_upper.events[0].notes()[0].pitch.name(), "F4");
    }

    #[test]
    fn overlapping_notes_go_to_new_voices_and_excess_is_dropped() {
        let t = build_template(Meter::COMMON, &[], &[], Tick(3840)).unwrap();
        let notes = vec![
            tn(0, 3840, 60),
            tn(960, 960, 64),
            tn(960, 1920, 67),
            tn(960, 480, 72),
            tn(960, 240, 76),
        ];
        let opts = ArrangeOptions {
            dynamics: false,
            ..ArrangeOptions::default()
        };
        let (p, rep) = arrange_part(
            &t,
            Meter::COMMON,
            &notes,
            "P",
            0,
            vec![Staff::standard(Clef::Treble)],
            &opts,
        )
        .unwrap();
        assert_eq!(p.measures[0].voices.len(), 4);
        assert_eq!(rep.dropped_notes, 1);
        let mut s = Score::new("x", 120.0, Meter::COMMON).unwrap();
        s.push_part(p).unwrap();
        assert!(validate_measures(&s).is_empty());
    }

    #[test]
    fn quantization_and_dynamics() {
        let t = build_template(Meter::COMMON, &[], &[], Tick(3840)).unwrap();
        let mut a = tn(10, 950, 60);
        a.velocity = 30;
        let b = tn(955, 960, 62);
        let (p, _) = arrange_part(
            &t,
            Meter::COMMON,
            &[a, b],
            "P",
            0,
            vec![Staff::standard(Clef::Treble)],
            &ArrangeOptions::default(),
        )
        .unwrap();
        let ev = &p.measures[0].voices[0].events;
        assert_eq!(ev[0].duration, Duration::quarter());
        assert_eq!(ev[0].dynamic, Some(Dynamic::Pp));
        assert_eq!(ev[1].dynamic, Some(Dynamic::Mf));
        assert_eq!(ev[1].notes()[0].pitch.midi(), 62);
    }

    #[test]
    fn tab_staff_gets_positions() {
        let t = build_template(Meter::COMMON, &[], &[], Tick(3840)).unwrap();
        let (p, _) = arrange_part(
            &t,
            Meter::COMMON,
            &[tn(0, 3840, 40), tn(0, 3840, 47), tn(0, 3840, 52)],
            "Violão",
            24,
            vec![Staff::standard_and_tab(
                Clef::Treble8vb,
                TabConfig::standard_guitar(),
            )],
            &ArrangeOptions::default(),
        )
        .unwrap();
        let notes = p.measures[0].voices[0].events[0].notes();
        assert!(notes.iter().all(|n| n.tab.is_some()));
        let cfg = TabConfig::standard_guitar();
        for n in notes {
            assert_eq!(
                tab::pitch_of(&cfg, n.tab.unwrap()).unwrap() as i32,
                n.pitch.midi()
            );
        }
    }
}
