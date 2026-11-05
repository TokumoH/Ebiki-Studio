//! Guitar Pro 7+ (`.gp`): zip com `Content/score.gpif` (XML "GPIF"). Importa o subconjunto central:
//! faixas, pautas (afinação, capotraste), compassos mestres (fórmula, armadura, repetições),
//! vozes, batidas (figura, pontos, quiálteras, dinâmica, apojatura) e notas (corda/traste, ligadura,
//! acentos, técnicas: hammer/pull, slide, bend, vibrato, palm mute, harmônico, let ring) e andamento.
//!
//! Convenções GPIF (conferidas no importador do alphaTab): `Pitches` da afinação vêm da corda mais
//! grave para a mais aguda e `String` 0 = corda mais grave (no modelo: corda 1 = mais aguda);
//! bend em centésimos de tom (100 = 1 tom = 200 cents); acentos em bits (1 staccato, 4 marcato,
//! 8 acento, 16 tenuto). GP3–GP6 (binários) são convertidos para GP7 na UI pelo alphaTab (ADR-0020).

use crate::archive::Archive;
use crate::error::{check_size, Imported, IoError};
use crate::musicxml::normalize_parts;
use crate::xml::{parse, Element};
use ctx_notation::duration::{Duration, NoteValue, Tuplet};
use ctx_notation::model::*;
use ctx_notation::pitch::{KeySignature, Mode, Pitch, Step};
use std::collections::HashMap;

pub const GPIF_ENTRY: &str = "Content/score.gpif";

/// Importa um arquivo `.gp` (zip).
pub fn import_gp(bytes: &[u8]) -> Result<Imported, IoError> {
    let mut zip = Archive::open(bytes)?;
    let gpif = zip.read(GPIF_ENTRY).map_err(|_| {
        IoError::Unsupported("arquivo .gp sem Content/score.gpif (GP6/GPX?)".into())
    })?;
    import_gpif(&gpif)
}

fn ids(text: &str) -> Vec<i64> {
    text.split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn by_id<'a>(parent: Option<&'a Element>, child: &str) -> HashMap<String, &'a Element> {
    parent
        .map(|p| {
            p.children_named(child)
                .filter_map(|c| c.attr("id").map(|id| (id.to_owned(), c)))
                .collect()
        })
        .unwrap_or_default()
}

fn property<'a>(props: Option<&'a Element>, name: &str) -> Option<&'a Element> {
    props?
        .children_named("Property")
        .find(|p| p.attr("name") == Some(name))
}

fn note_value(s: &str) -> Option<NoteValue> {
    Some(match s {
        "Whole" | "DoubleWhole" | "Long" => NoteValue::Whole,
        "Half" => NoteValue::Half,
        "Quarter" => NoteValue::Quarter,
        "Eighth" => NoteValue::Eighth,
        "16th" => NoteValue::Sixteenth,
        "32nd" => NoteValue::ThirtySecond,
        "64th" => NoteValue::SixtyFourth,
        "128th" => NoteValue::OneTwentyEighth,
        _ => return None,
    })
}

fn dynamic(s: &str) -> Option<Dynamic> {
    Dynamic::from_symbol(&s.to_ascii_lowercase())
}

struct TrackInfo {
    name: String,
    program: u8,
    percussion: bool,
    staves: Vec<Staff>,
    /// Afinação da grave para a aguda (convenção GPIF), por pauta.
    tuning_low_high: Vec<Vec<u8>>,
}

fn read_track(t: &Element) -> TrackInfo {
    let name: String = t
        .child_text("Name")
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect();
    let program = t
        .path("Sounds/Sound/MIDI/Program")
        .or_else(|| t.path("GeneralMidi/Program"))
        .and_then(|e| e.text.trim().parse::<u8>().ok())
        .unwrap_or(0)
        .min(127);
    let channel = t
        .path("MidiConnection/PrimaryChannel")
        .or_else(|| t.path("GeneralMidi/PrimaryChannel"))
        .and_then(|e| e.text.trim().parse::<u8>().ok());
    let drum_kit = t
        .path("InstrumentSet/Type")
        .is_some_and(|e| e.text.trim() == "drumKit");
    let percussion = channel == Some(9) || drum_kit;
    let staff_props: Vec<Option<&Element>> = match t.child("Staves") {
        Some(st) => st
            .children_named("Staff")
            .map(|s| s.child("Properties"))
            .collect(),
        None => vec![t.child("Properties")],
    };
    let mut staves = Vec::new();
    let mut tunings = Vec::new();
    for props in staff_props.into_iter().take(MAX_STAVES) {
        let tuning: Vec<u8> = property(props, "Tuning")
            .and_then(|p| p.child_text("Pitches"))
            .map(|t| {
                t.split_whitespace()
                    .filter_map(|x| x.parse::<u8>().ok())
                    .filter(|x| *x <= 127)
                    .collect()
            })
            .unwrap_or_default();
        let capo = property(props, "CapoFret")
            .and_then(|p| p.child_num::<u8>("Fret"))
            .unwrap_or(0);
        let tab =
            (!tuning.is_empty() && tuning.len() <= MAX_STRINGS && !percussion).then(|| TabConfig {
                tuning: tuning.iter().rev().copied().take(MAX_STRINGS).collect(),
                capo,
                frets: 24,
            });
        let tab = tab.filter(|t| t.validate().is_ok());
        let staff = match (&tab, percussion) {
            (_, true) => Staff::standard(Clef::Percussion),
            (Some(cfg), _) => {
                let clef = if cfg.tuning.last().is_some_and(|&low| low < 36) {
                    Clef::Bass
                } else {
                    Clef::Treble8vb
                };
                Staff::standard_and_tab(clef, cfg.clone())
            }
            (None, _) => Staff::standard(Clef::Treble),
        };
        tunings.push(if tab.is_some() { tuning } else { Vec::new() });
        staves.push(staff);
    }
    if staves.is_empty() {
        staves.push(Staff::standard(Clef::Treble));
        tunings.push(Vec::new());
    }
    TrackInfo {
        name,
        program,
        percussion,
        staves,
        tuning_low_high: tunings,
    }
}

/// Importa o XML GPIF (conteúdo de `Content/score.gpif`).
pub fn import_gpif(bytes: &[u8]) -> Result<Imported, IoError> {
    check_size(bytes)?;
    let root = parse(bytes)?;
    if root.name != "GPIF" {
        return Err(IoError::malformed("raiz GPIF ausente"));
    }
    let mut warnings: Vec<String> = Vec::new();
    let title = root
        .path("Score/Title")
        .map(|e| e.text.trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Sem título".into());
    let composer = root
        .path("Score/Music")
        .or_else(|| root.path("Score/Artist"))
        .map(|e| e.text.trim().chars().take(200).collect::<String>())
        .unwrap_or_default();

    let tracks: Vec<TrackInfo> = root
        .child("Tracks")
        .map(|t| t.children_named("Track").map(read_track).collect())
        .unwrap_or_default();
    if tracks.is_empty() {
        return Err(IoError::malformed("GPIF sem faixas"));
    }
    let bars = by_id(root.child("Bars"), "Bar");
    let voices = by_id(root.child("Voices"), "Voice");
    let beats = by_id(root.child("Beats"), "Beat");
    let notes = by_id(root.child("Notes"), "Note");
    let rhythms = by_id(root.child("Rhythms"), "Rhythm");

    let master_bars: Vec<&Element> = root
        .child("MasterBars")
        .map(|m| m.children_named("MasterBar").collect())
        .unwrap_or_default();
    if master_bars.is_empty() {
        return Err(IoError::malformed("GPIF sem compassos"));
    }
    if master_bars.len() > MAX_MEASURES {
        return Err(IoError::TooLarge("compassos"));
    }

    let mut next_id = 1u64;
    let mut parts: Vec<Part> = tracks
        .iter()
        .take(MAX_PARTS)
        .map(|t| Part {
            name: if t.name.trim().is_empty() {
                "Faixa".into()
            } else {
                t.name.clone()
            },
            program: t.program,
            percussion: t.percussion,
            staves: t.staves.clone(),
            measures: Vec::new(),
            spanners: Vec::new(),
        })
        .collect();
    let mut last_dyn: HashMap<(usize, usize), Dynamic> = HashMap::new();
    let mut meter = Meter::COMMON;
    let mut first_meter = None;
    for (mi, mb) in master_bars.iter().enumerate() {
        let key = mb
            .child("Key")
            .and_then(|k| {
                let n: i8 = k.child_num("AccidentalCount")?;
                let mode = if k.child_text("Mode") == Some("Minor") {
                    Mode::Minor
                } else {
                    Mode::Major
                };
                KeySignature::new(n, mode).ok()
            })
            .unwrap_or_default();
        let this_meter = mb.child_text("Time").and_then(|t| {
            let (a, b) = t.split_once('/')?;
            Meter::new(a.trim().parse().ok()?, b.trim().parse().ok()?).ok()
        });
        let mut measure_meter = None;
        if let Some(mt) = this_meter {
            if mi == 0 {
                first_meter = Some(mt);
            } else if mt != meter {
                measure_meter = Some(mt);
            }
            meter = mt;
        }
        let rep = mb.child("Repeat");
        let flag = |name: &str| rep.and_then(|r| r.attr(name)) == Some("true");
        let repeat_start = flag("start");
        let repeat_end = flag("end").then(|| {
            rep.and_then(|r| r.attr("count"))
                .and_then(|c| c.parse::<u8>().ok())
                .unwrap_or(2)
                .clamp(2, 16)
        });
        let bar_ids = mb.child_text("Bars").map(ids).unwrap_or_default();
        let mut cursor = 0usize;
        for (ti, track) in tracks.iter().enumerate().take(parts.len()) {
            let mut m = Measure::new(key);
            m.meter = measure_meter;
            m.repeat_start = repeat_start;
            m.repeat_end = repeat_end;
            m.voices.clear();
            for (si, staff) in track.staves.iter().enumerate() {
                let bar_id = bar_ids.get(cursor + si).copied().unwrap_or(-1);
                let Some(bar) = bars.get(&bar_id.to_string()) else {
                    continue;
                };
                let tuning = &track.tuning_low_high[si];
                let n_strings = tuning.len();
                let capo = staff.tab.as_ref().map_or(0, |t| t.capo);
                let tab_cfg = staff.tab.as_ref();
                let voice_ids = bar.child_text("Voices").map(ids).unwrap_or_default();
                for (slot, vid) in voice_ids.iter().enumerate() {
                    let Some(vel) = voices.get(&vid.to_string()) else {
                        continue;
                    };
                    if m.voices.len() >= MAX_VOICES {
                        warnings.push(format!(
                            "compasso {}: mais de 4 vozes, excedentes descartadas",
                            mi + 1
                        ));
                        break;
                    }
                    let mut voice = Voice::new(si as u8);
                    let stream = (ti, si * 4 + slot);
                    for bid in vel.child_text("Beats").map(ids).unwrap_or_default() {
                        let Some(beat) = beats.get(&bid.to_string()) else {
                            continue;
                        };
                        if voice.events.len() >= MAX_EVENTS_PER_VOICE {
                            warnings.push("eventos demais na voz: descartados".into());
                            break;
                        }
                        let rhythm = beat
                            .child("Rhythm")
                            .and_then(|r| r.attr("ref"))
                            .and_then(|r| rhythms.get(r));
                        let duration = rhythm
                            .and_then(|r| {
                                let v = note_value(r.child_text("NoteValue")?)?;
                                let dots = r
                                    .child("AugmentationDot")
                                    .and_then(|d| d.attr("count"))
                                    .and_then(|c| c.parse::<u8>().ok())
                                    .unwrap_or(0)
                                    .min(3);
                                let mut d = Duration::new(v).dotted(dots);
                                if let Some(t) = r.child("PrimaryTuplet") {
                                    let num = t.attr("num").and_then(|x| x.parse().ok());
                                    let den = t.attr("den").and_then(|x| x.parse().ok());
                                    if let (Some(a), Some(b)) = (num, den) {
                                        d.tuplet = Tuplet::new(a, b).ok();
                                    }
                                }
                                Some(d)
                            })
                            .unwrap_or_else(Duration::quarter);
                        let mut chord: Vec<Note> = Vec::new();
                        let mut articulations: Vec<Articulation> = Vec::new();
                        for nid in beat.child_text("Notes").map(ids).unwrap_or_default() {
                            let Some(nel) = notes.get(&nid.to_string()) else {
                                continue;
                            };
                            match read_note(nel, tuning, n_strings, capo, &key).map(|(mut n, a)| {
                                // posição fora do braço: mantém a altura, descarta a tablatura
                                if let (Some(pos), Some(cfg)) = (n.tab, tab_cfg) {
                                    if ctx_notation::tab::check_position(cfg, pos).is_err() {
                                        n.tab = None;
                                    }
                                }
                                (n, a)
                            }) {
                                Some((note, arts)) if chord.len() < MAX_NOTES_PER_CHORD => {
                                    chord.push(note);
                                    for a in arts {
                                        if !articulations.contains(&a) {
                                            articulations.push(a);
                                        }
                                    }
                                }
                                Some(_) => {}
                                None => warnings.push("nota sem altura descartada".into()),
                            }
                        }
                        let mut ev = if chord.is_empty() {
                            Event::rest(duration)
                        } else {
                            chord.sort_by_key(|n| n.pitch.midi());
                            Event::chord(duration, chord)
                        };
                        ev.id = EventId(next_id);
                        next_id += 1;
                        articulations.sort();
                        ev.articulations = articulations;
                        if !ev.is_rest() {
                            ev.grace = match beat.child_text("GraceNotes") {
                                Some("OnBeat") => Some(GraceKind::Appoggiatura),
                                Some("BeforeBeat") => Some(GraceKind::Acciaccatura),
                                _ => None,
                            };
                        }
                        if let Some(d) = beat.child_text("Dynamic").and_then(dynamic) {
                            if last_dyn.get(&stream) != Some(&d) {
                                ev.dynamic = Some(d);
                                last_dyn.insert(stream, d);
                            }
                        }
                        voice.events.push(ev);
                    }
                    resolve_hopo(&mut voice);
                    m.voices.push(voice);
                }
            }
            cursor += track.staves.len();
            if m.voices.is_empty() {
                let mut v = Voice::new(0);
                for d in Duration::decompose(meter.capacity()).0 {
                    let mut e = Event::rest(d);
                    e.id = EventId(next_id);
                    next_id += 1;
                    v.events.push(e);
                }
                m.voices.push(v);
            }
            parts[ti].measures.push(m);
        }
    }
    normalize_parts(&mut parts, &mut warnings);

    let mut tempos: Vec<(usize, f64)> = Vec::new();
    if let Some(autos) = root.path("MasterTrack/Automations") {
        for a in autos.children_named("Automation") {
            if a.child_text("Type") != Some("Tempo") {
                continue;
            }
            let bar: usize = a.child_num("Bar").unwrap_or(0);
            let bpm = a
                .child_text("Value")
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite());
            if let Some(b) = bpm {
                tempos.push((bar, b.clamp(20.0, 999.0)));
            }
        }
    }
    tempos.sort_by_key(|t| t.0);
    let bpm0 = tempos.iter().find(|t| t.0 == 0).map_or(120.0, |t| t.1);
    let mut score = Score::new(title, bpm0, first_meter.unwrap_or(Meter::COMMON))?;
    score.composer = composer;
    score.parts = parts;
    score.next_event_id = next_id;
    let starts = score.measure_starts();
    for (bar, bpm) in tempos {
        if bar > 0 {
            if let Some(t) = starts.get(bar) {
                let _ = score.tempo.set_tempo(*t, bpm);
            }
        }
    }
    warnings.dedup();
    warnings.truncate(200);
    score.check_invariants()?;
    Ok(Imported { score, warnings })
}

fn read_note(
    n: &Element,
    tuning_low_high: &[u8],
    n_strings: usize,
    capo: u8,
    key: &KeySignature,
) -> Option<(Note, Vec<Articulation>)> {
    let props = n.child("Properties");
    let enabled = |name: &str| property(props, name).is_some_and(|p| p.child("Enable").is_some());
    let gp_string: Option<usize> = property(props, "String").and_then(|p| p.child_num("String"));
    let fret: Option<u8> = property(props, "Fret").and_then(|p| p.child_num("Fret"));
    let mut tab = None;
    let midi: Option<i32> = match (gp_string, fret) {
        (Some(s), Some(f)) if s < n_strings && n_strings > 0 => {
            tab = Some(TabPosition {
                string: (n_strings - s) as u8,
                fret: f,
            });
            Some(tuning_low_high[s] as i32 + capo as i32 + f as i32)
        }
        _ => property(props, "Midi")
            .and_then(|p| p.child_num::<i32>("Number"))
            .or_else(|| {
                let p = property(props, "ConcertPitch")
                    .or_else(|| property(props, "TransposedPitch"))?
                    .child("Pitch")?;
                let step = Step::from_letter(p.child_text("Step")?.chars().next()?)?;
                let alter = match p.child_text("Accidental").unwrap_or("") {
                    "#" => 1,
                    "##" | "x" => 2,
                    "b" => -1,
                    "bb" => -2,
                    _ => 0,
                };
                let oct: i8 = p.child_num("Octave")?;
                Some(Pitch::new(step, alter, oct).ok()?.midi())
            }),
    };
    let midi = midi.filter(|m| (0..=127).contains(m))? as u8;
    let mut techniques = Vec::new();
    if enabled("HopoOrigin") {
        techniques.push(Technique::HammerOn);
    }
    if property(props, "Slide")
        .and_then(|p| p.child_num::<u32>("Flags"))
        .is_some_and(|f| f != 0)
    {
        techniques.push(Technique::Slide);
    }
    if enabled("Bended") || property(props, "BendDestinationValue").is_some() {
        let value = property(props, "BendDestinationValue")
            .or_else(|| property(props, "BendMiddleValue"))
            .and_then(|p| p.child_text("Float"))
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(100.0);
        techniques.push(Technique::Bend {
            cents: (value * 2.0).round().clamp(0.0, 1200.0) as u16,
        });
    }
    if n.child("Vibrato").is_some() {
        techniques.push(Technique::Vibrato);
    }
    if enabled("PalmMuted") {
        techniques.push(Technique::PalmMute);
    }
    if let Some(h) = property(props, "HarmonicType").and_then(|p| p.child_text("HType")) {
        techniques.push(Technique::Harmonic {
            harmonic: if h.eq_ignore_ascii_case("natural") {
                HarmonicKind::Natural
            } else {
                HarmonicKind::Artificial
            },
        });
    }
    if n.child("LetRing").is_some() {
        techniques.push(Technique::LetRing);
    }
    let tie = n
        .child("Tie")
        .is_some_and(|t| t.attr("origin") == Some("true"));
    let flags: u32 = n.child_num("Accent").unwrap_or(0);
    let mut arts = Vec::new();
    if flags & 0x01 != 0 {
        arts.push(Articulation::Staccato);
    }
    if flags & 0x04 != 0 {
        arts.push(Articulation::Marcato);
    }
    if flags & 0x08 != 0 {
        arts.push(Articulation::Accent);
    }
    if flags & 0x10 != 0 {
        arts.push(Articulation::Tenuto);
    }
    Some((
        Note {
            pitch: Pitch::from_midi(midi, key),
            tie,
            tab,
            techniques,
        },
        arts,
    ))
}

/// GPIF marca só a origem do ligado: vira pull-off quando a próxima nota na mesma corda é mais grave.
fn resolve_hopo(voice: &mut Voice) {
    for i in 0..voice.events.len().saturating_sub(1) {
        let next: Vec<(Option<TabPosition>, i32)> = voice.events[i + 1]
            .notes()
            .iter()
            .map(|n| (n.tab, n.pitch.midi()))
            .collect();
        if let Some(notes) = voice.events[i].notes_mut() {
            for n in notes {
                if !n.techniques.contains(&Technique::HammerOn) {
                    continue;
                }
                let target = next
                    .iter()
                    .find(|(t, _)| t.is_some() && t.map(|x| x.string) == n.tab.map(|x| x.string))
                    .or(next.first());
                if let Some((_, m)) = target {
                    if *m < n.pitch.midi() {
                        for t in &mut n.techniques {
                            if *t == Technique::HammerOn {
                                *t = Technique::PullOff;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_gpif() {
        assert!(import_gpif(b"<score-partwise/>").is_err());
        assert!(import_gpif(b"<GPIF></GPIF>").is_err());
        assert!(import_gp(b"PK\x03\x04").is_err());
    }
}
