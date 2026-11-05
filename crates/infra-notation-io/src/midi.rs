//! Standard MIDI File ⇄ `Score` (crate `midly`).
//!
//! Exportação: formato 1, 960 PPQ; trilha 0 = condutor (tempo, fórmula, armadura), uma trilha por
//! parte (nome, programa, notas de `playback::render`, com repetições expandidas por padrão).
//! Importação: formato 0/1; uma parte por (trilha, canal); resolução convertida para 960 PPQ;
//! quantização configurável; compassos/vozes montados por `ctx_notation::arrange`.

use crate::error::{check_size, Imported, IoError, MAX_MIDI_NOTES};
use ctx_notation::arrange::{arrange_part, build_template, ArrangeOptions, TimedNote};
use ctx_notation::model::*;
use ctx_notation::pitch::{KeySignature, Mode};
use ctx_notation::playback::{self, PlaybackOptions};
use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use shared_kernel::time::{Tick, PPQ};
use std::collections::{BTreeMap, HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiExportOptions {
    pub expand_repeats: bool,
}

impl Default for MidiExportOptions {
    fn default() -> Self {
        Self {
            expand_repeats: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiImportOptions {
    /// Grade de quantização em ticks a 960 PPQ (0 = sem quantização). Padrão: semicolcheia.
    pub grid_ticks: i64,
    pub max_voices: usize,
    /// Título (senão: nome da trilha condutora ou "MIDI importado").
    pub title: Option<String>,
}

impl Default for MidiImportOptions {
    fn default() -> Self {
        Self {
            grid_ticks: PPQ / 4,
            max_voices: MAX_VOICES,
            title: None,
        }
    }
}

// ───────────────────────────── exportação ─────────────────────────────

fn to_delta<'a>(mut events: Vec<(i64, u8, TrackEventKind<'a>)>) -> Vec<TrackEvent<'a>> {
    events.sort_by_key(|e| (e.0, e.1));
    let mut last = 0i64;
    let mut out: Vec<TrackEvent<'a>> = events
        .into_iter()
        .map(|(t, _, kind)| {
            let delta = (t - last).clamp(0, 0x0FFF_FFFF) as u32;
            last = t.max(last);
            TrackEvent {
                delta: u28::new(delta),
                kind,
            }
        })
        .collect();
    out.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    out
}

fn tempo_us(bpm: f64) -> u24 {
    u24::new((60_000_000.0 / bpm.clamp(20.0, 999.0)).round() as u32)
}

/// Exporta a partitura como SMF formato 1.
pub fn export_midi(score: &Score, opts: &MidiExportOptions) -> Result<Vec<u8>, IoError> {
    let order = playback::performance_order(score, opts.expand_repeats);
    let meters = score.meters();
    let starts = score.measure_starts();
    let refs = score.reference_measures();
    let title = score.title.clone();
    let names: Vec<String> = score.parts.iter().map(|p| p.name.clone()).collect();

    let mut conductor: Vec<(i64, u8, TrackEventKind)> = vec![(
        0,
        0,
        TrackEventKind::Meta(MetaMessage::TrackName(title.as_bytes())),
    )];
    let mut last_meter: Option<Meter> = None;
    let mut last_key: Option<KeySignature> = None;
    let mut last_bpm: Option<f64> = None;
    for play in &order {
        let mi = play.measure;
        let p0 = play.start.0;
        if last_meter != Some(meters[mi]) {
            let mt = meters[mi];
            conductor.push((
                p0,
                1,
                TrackEventKind::Meta(MetaMessage::TimeSignature(
                    mt.numerator,
                    mt.denominator.trailing_zeros() as u8,
                    24,
                    8,
                )),
            ));
            last_meter = Some(mt);
        }
        if let Some(m) = refs.get(mi) {
            if last_key != Some(m.key) {
                conductor.push((
                    p0,
                    2,
                    TrackEventKind::Meta(MetaMessage::KeySignature(
                        m.key.fifths,
                        m.key.mode == Mode::Minor,
                    )),
                ));
                last_key = Some(m.key);
            }
        }
        let (s, e) = (starts[mi], starts[mi + 1]);
        let at_start = score.tempo.bpm_at(s);
        if last_bpm != Some(at_start) {
            conductor.push((
                p0,
                3,
                TrackEventKind::Meta(MetaMessage::Tempo(tempo_us(at_start))),
            ));
            last_bpm = Some(at_start);
        }
        for t in score.tempo.tempos() {
            if t.tick > s && t.tick < e && last_bpm != Some(t.bpm) {
                conductor.push((
                    p0 + (t.tick.0 - s.0),
                    3,
                    TrackEventKind::Meta(MetaMessage::Tempo(tempo_us(t.bpm))),
                ));
                last_bpm = Some(t.bpm);
            }
        }
    }
    let mut tracks = vec![to_delta(conductor)];

    let notes = playback::render(
        score,
        &PlaybackOptions {
            expand_repeats: opts.expand_repeats,
        },
    );
    let channels = playback::channels(score);
    for (pi, p) in score.parts.iter().enumerate() {
        let ch = u4::new(channels[pi]);
        let mut evs: Vec<(i64, u8, TrackEventKind)> = vec![
            (
                0,
                0,
                TrackEventKind::Meta(MetaMessage::TrackName(names[pi].as_bytes())),
            ),
            (
                0,
                1,
                TrackEventKind::Midi {
                    channel: ch,
                    message: MidiMessage::ProgramChange {
                        program: u7::new(p.program.min(127)),
                    },
                },
            ),
        ];
        for n in notes.iter().filter(|n| n.part == pi) {
            evs.push((
                n.start_tick.0,
                3,
                TrackEventKind::Midi {
                    channel: ch,
                    message: MidiMessage::NoteOn {
                        key: u7::new(n.key.min(127)),
                        vel: u7::new(n.velocity.clamp(1, 127)),
                    },
                },
            ));
            evs.push((
                n.start_tick.0 + n.duration_ticks.0,
                2,
                TrackEventKind::Midi {
                    channel: ch,
                    message: MidiMessage::NoteOff {
                        key: u7::new(n.key.min(127)),
                        vel: u7::new(64),
                    },
                },
            ));
        }
        tracks.push(to_delta(evs));
    }
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(PPQ as u16)),
    ));
    smf.tracks = tracks;
    let mut out = Vec::new();
    smf.write_std(&mut out).map_err(IoError::malformed)?;
    Ok(out)
}

// ───────────────────────────── importação ─────────────────────────────

#[derive(Default)]
struct Group {
    name: Option<String>,
    program: Option<u8>,
    notes: Vec<TimedNote>,
}

fn staves_for(program: u8, percussion: bool, notes: &[TimedNote]) -> Vec<Staff> {
    if percussion {
        return vec![Staff::standard(Clef::Percussion)];
    }
    let min = notes.iter().map(|n| n.key).min().unwrap_or(60);
    let max = notes.iter().map(|n| n.key).max().unwrap_or(60);
    let avg = notes.iter().map(|n| n.key as u32).sum::<u32>() / notes.len().max(1) as u32;
    match program {
        24..=31 => vec![Staff::standard_and_tab(
            Clef::Treble8vb,
            TabConfig::standard_guitar(),
        )],
        32..=39 => vec![Staff::standard_and_tab(
            Clef::Bass,
            TabConfig::standard_bass(),
        )],
        0..=7 => vec![Staff::standard(Clef::Treble), Staff::standard(Clef::Bass)],
        _ if min < 55 && max >= 67 => {
            vec![Staff::standard(Clef::Treble), Staff::standard(Clef::Bass)]
        }
        _ if avg < 55 => vec![Staff::standard(Clef::Bass)],
        _ => vec![Staff::standard(Clef::Treble)],
    }
}

/// Importa um Standard MIDI File.
pub fn import_midi(bytes: &[u8], opts: &MidiImportOptions) -> Result<Imported, IoError> {
    check_size(bytes)?;
    let smf = Smf::parse(bytes).map_err(IoError::malformed)?;
    let ppq = match smf.header.timing {
        Timing::Metrical(t) if t.as_int() > 0 => t.as_int() as i128,
        Timing::Metrical(_) => return Err(IoError::malformed("resolução zero")),
        Timing::Timecode(..) => {
            return Err(IoError::Unsupported("MIDI com tempo SMPTE".into()));
        }
    };
    let mut warnings = Vec::new();
    if smf.header.format == Format::Sequential {
        warnings.push("MIDI formato 2: trilhas sobrepostas como simultâneas".into());
    }
    let scale = |t: u64| -> i64 {
        ((t as i128 * PPQ as i128 + ppq / 2) / ppq).min(i64::MAX as i128 / 4) as i64
    };
    let mut tempos: BTreeMap<i64, f64> = BTreeMap::new();
    let mut meters: BTreeMap<i64, Meter> = BTreeMap::new();
    let mut keys: BTreeMap<i64, KeySignature> = BTreeMap::new();
    let mut groups: BTreeMap<(usize, u8), Group> = BTreeMap::new();
    let mut track_names: Vec<Option<String>> = Vec::new();
    let mut total_notes = 0usize;
    for (ti, track) in smf.tracks.iter().enumerate() {
        let mut abs: u64 = 0;
        let mut name = None;
        let mut open: HashMap<(u8, u8), VecDeque<(i64, u8)>> = HashMap::new();
        let mut programs: HashMap<u8, u8> = HashMap::new();
        for ev in track {
            abs = abs.saturating_add(ev.delta.as_int() as u64);
            let t = scale(abs);
            match ev.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(us)) if us.as_int() > 0 => {
                    // arredonda a 3 casas: microssegundos inteiros não representam BPMs exatos
                    let bpm = ((60_000_000.0 / us.as_int() as f64) * 1000.0).round() / 1000.0;
                    let bpm = bpm.clamp(20.0, 999.0);
                    tempos.insert(t, bpm);
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, dpow, _, _)) => {
                    match (dpow <= 5).then(|| Meter::new(n, 1u8 << dpow)) {
                        Some(Ok(m)) => {
                            meters.insert(t, m);
                        }
                        _ => warnings.push(format!(
                            "fórmula de compasso inválida ignorada: {n}/2^{dpow}"
                        )),
                    }
                }
                TrackEventKind::Meta(MetaMessage::KeySignature(sf, minor)) => {
                    if let Ok(k) =
                        KeySignature::new(sf, if minor { Mode::Minor } else { Mode::Major })
                    {
                        keys.insert(t, k);
                    }
                }
                TrackEventKind::Meta(MetaMessage::TrackName(raw)) => {
                    let s: String = String::from_utf8_lossy(raw)
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(64)
                        .collect();
                    if !s.trim().is_empty() {
                        name = Some(s.trim().to_owned());
                    }
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    let gkey = (
                        if smf.header.format == Format::SingleTrack {
                            0
                        } else {
                            ti
                        },
                        ch,
                    );
                    match message {
                        MidiMessage::ProgramChange { program } => {
                            programs.entry(ch).or_insert(program.as_int());
                        }
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            open.entry((ch, key.as_int()))
                                .or_default()
                                .push_back((t, vel.as_int()));
                        }
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            if let Some((start, vel)) = open
                                .get_mut(&(ch, key.as_int()))
                                .and_then(|q| q.pop_front())
                            {
                                total_notes += 1;
                                if total_notes > MAX_MIDI_NOTES {
                                    return Err(IoError::TooLarge("notas MIDI"));
                                }
                                let g = groups.entry(gkey).or_default();
                                g.program = g.program.or(programs.get(&ch).copied());
                                g.notes.push(TimedNote {
                                    start: Tick(start),
                                    duration: Tick((t - start).max(1)),
                                    key: key.as_int(),
                                    velocity: vel,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        let end = scale(abs);
        for ((ch, key), q) in open {
            for (start, vel) in q {
                let gkey = (
                    if smf.header.format == Format::SingleTrack {
                        0
                    } else {
                        ti
                    },
                    ch,
                );
                let g = groups.entry(gkey).or_default();
                g.notes.push(TimedNote {
                    start: Tick(start),
                    duration: Tick((end - start).max(1)),
                    key,
                    velocity: vel,
                });
                warnings.push("nota sem fim encerrada no fim da trilha".into());
            }
        }
        for ((gt, _), g) in groups.iter_mut() {
            if *gt == ti && g.name.is_none() {
                g.name = name.clone();
            }
        }
        track_names.push(name);
    }
    warnings.truncate(200);

    let meter0 = meters.get(&0).copied().unwrap_or(Meter::COMMON);
    let bpm0 = tempos.get(&0).copied().unwrap_or(120.0);
    let title = opts
        .title
        .clone()
        .or_else(|| {
            let has_notes = groups.keys().any(|(t, _)| *t == 0);
            (!has_notes)
                .then(|| track_names.first().cloned().flatten())
                .flatten()
        })
        .unwrap_or_else(|| "MIDI importado".into());
    let mut score = Score::new(title, bpm0, meter0)?;
    for (&t, &bpm) in &tempos {
        if t > 0 {
            score
                .tempo
                .set_tempo(Tick(t), bpm)
                .map_err(|_| IoError::malformed("andamento inválido"))?;
        }
    }
    let end = groups
        .values()
        .flat_map(|g| g.notes.iter().map(|n| n.start.0 + n.duration.0))
        .max()
        .unwrap_or(1);
    let meter_changes: Vec<(Tick, Meter)> = meters.iter().map(|(t, m)| (Tick(*t), *m)).collect();
    let key_changes: Vec<(Tick, KeySignature)> = keys.iter().map(|(t, k)| (Tick(*t), *k)).collect();
    let template = build_template(meter0, &meter_changes, &key_changes, Tick(end.max(1)))?;
    let arrange_opts = ArrangeOptions {
        grid: opts.grid_ticks.max(0),
        max_voices: opts.max_voices.clamp(1, MAX_VOICES),
        ..ArrangeOptions::default()
    };
    let multi_channel_tracks: Vec<usize> = {
        let mut count: HashMap<usize, usize> = HashMap::new();
        for (t, _) in groups.keys() {
            *count.entry(*t).or_default() += 1;
        }
        count
            .into_iter()
            .filter(|(_, c)| *c > 1)
            .map(|(t, _)| t)
            .collect()
    };
    for ((ti, ch), g) in groups {
        if g.notes.is_empty() {
            continue;
        }
        if score.parts.len() >= MAX_PARTS {
            warnings.push("partes demais: excedentes descartadas".into());
            break;
        }
        let percussion = ch == 9;
        let program = g.program.unwrap_or(0).min(127);
        let base = g.name.unwrap_or_else(|| format!("Canal {}", ch + 1));
        let name: String = if multi_channel_tracks.contains(&ti) {
            format!("{base} (canal {})", ch + 1)
        } else {
            base
        }
        .chars()
        .take(64)
        .collect();
        let staves = staves_for(program, percussion, &g.notes);
        let (mut part, report) = arrange_part(
            &template,
            meter0,
            &g.notes,
            name,
            program,
            staves,
            &arrange_opts,
        )?;
        part.percussion = percussion;
        if report.dropped_notes > 0 {
            warnings.push(format!(
                "{}: {} nota(s) descartada(s) (vozes esgotadas)",
                part.name, report.dropped_notes
            ));
        }
        score.push_part(part)?;
    }
    if score.parts.is_empty() {
        let blank = Part {
            name: "Vazio".into(),
            program: 0,
            percussion: false,
            staves: vec![Staff::standard(Clef::Treble)],
            measures: {
                let mut cur = meter0;
                template
                    .iter()
                    .map(|m| {
                        cur = m.meter.unwrap_or(cur);
                        m.empty_like(cur.capacity(), 1)
                    })
                    .collect()
            },
            spanners: vec![],
        };
        score.push_part(blank)?;
        warnings.push("MIDI sem notas".into());
    }
    Ok(Imported { score, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage_and_smpte() {
        assert!(import_midi(b"MThd", &MidiImportOptions::default()).is_err());
        assert!(import_midi(b"", &MidiImportOptions::default()).is_err());
        // cabeçalho com divisão SMPTE (bit 15 ligado)
        let mut smpte = b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\xE7\x28".to_vec();
        smpte.extend_from_slice(b"MTrk\x00\x00\x00\x04\x00\xFF\x2F\x00");
        assert!(matches!(
            import_midi(&smpte, &MidiImportOptions::default()),
            Err(IoError::Unsupported(_))
        ));
    }
}
