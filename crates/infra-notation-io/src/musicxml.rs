//! MusicXML 4.0 (partwise) — importação e exportação, `.musicxml` (texto) e `.mxl` (zip).
//!
//! Mapeamento resumido (ver spec da Fase 7):
//! notas/pausas/acordes (`<chord/>`), figuras (`<type>`, `<dot/>`, `<time-modification>`), ligaduras
//! (`<tie>`/`<tied>`), armadura/fórmula (`<key>`, `<time>`), articulações, fermata, dinâmicas e
//! crescendo/diminuendo (`<direction>`), ligaduras de expressão (`<slur>`), tablatura
//! (`<technical><string><fret>` + técnicas), afinação (`<staff-details><staff-tuning>`, `<capo>`),
//! apojaturas (`<grace>`), letra (`<lyric>`), repetições (`<barline><repeat>`) e andamento
//! (`<sound tempo>`). Divisões fixas: 960 por semínima (PPQ do projeto).

use crate::archive::{write_zip, Archive};
use crate::error::{check_size, Imported, IoError};
use crate::xml::{parse, Element, XmlWriter};
use ctx_notation::duration::{Duration, NoteValue, Tuplet};
use ctx_notation::frac::Frac;
use ctx_notation::model::*;
use ctx_notation::pitch::{KeySignature, Mode, Pitch, Step};
use ctx_notation::playback;
use shared_kernel::time::{Tick, PPQ};
use std::collections::{HashMap, HashSet};

const DIVISIONS: i64 = PPQ;

fn type_name(v: NoteValue) -> &'static str {
    match v {
        NoteValue::Whole => "whole",
        NoteValue::Half => "half",
        NoteValue::Quarter => "quarter",
        NoteValue::Eighth => "eighth",
        NoteValue::Sixteenth => "16th",
        NoteValue::ThirtySecond => "32nd",
        NoteValue::SixtyFourth => "64th",
        NoteValue::OneTwentyEighth => "128th",
    }
}

fn type_from_name(s: &str) -> Option<NoteValue> {
    NoteValue::ALL.into_iter().find(|v| type_name(*v) == s)
}

fn clef_xml(clef: Clef) -> (&'static str, u8, i8) {
    match clef {
        Clef::Treble => ("G", 2, 0),
        Clef::Treble8vb => ("G", 2, -1),
        Clef::Bass => ("F", 4, 0),
        Clef::Alto => ("C", 3, 0),
        Clef::Tenor => ("C", 4, 0),
        Clef::Percussion => ("percussion", 2, 0),
    }
}

fn articulation_tag(a: Articulation) -> &'static str {
    match a {
        Articulation::Staccato => "staccato",
        Articulation::Accent => "accent",
        Articulation::Tenuto => "tenuto",
        Articulation::Marcato => "strong-accent",
        Articulation::Fermata => "fermata",
    }
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

// ───────────────────────────── exportação ─────────────────────────────

/// Exporta como MusicXML (texto UTF-8).
pub fn export_musicxml(score: &Score) -> String {
    let mut w = XmlWriter::new();
    w.raw(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#);
    w.raw(r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#);
    w.open("score-partwise", &[("version", "4.0".into())]);
    w.open("work", &[]);
    w.leaf("work-title", &score.title);
    w.close("work");
    w.open("identification", &[]);
    if !score.composer.is_empty() {
        w.leaf_with("creator", &[("type", "composer".into())], &score.composer);
    }
    w.open("encoding", &[]);
    w.leaf("software", "Ebiki Studio");
    w.close("encoding");
    w.close("identification");

    let channels = playback::channels(score);
    w.open("part-list", &[]);
    for (pi, p) in score.parts.iter().enumerate() {
        let id = format!("P{}", pi + 1);
        let inst = format!("{id}-I1");
        w.open("score-part", &[("id", id.clone())]);
        w.leaf("part-name", &p.name);
        w.open("score-instrument", &[("id", inst.clone())]);
        w.leaf("instrument-name", &p.name);
        w.close("score-instrument");
        w.open("midi-instrument", &[("id", inst)]);
        w.leaf("midi-channel", channels[pi] as u32 + 1);
        w.leaf("midi-program", p.program as u32 + 1);
        w.close("midi-instrument");
        w.close("score-part");
    }
    w.close("part-list");

    let meters = score.meters();
    let starts = score.measure_starts();
    for (pi, p) in score.parts.iter().enumerate() {
        w.open("part", &[("id", format!("P{}", pi + 1))]);
        export_part(&mut w, score, pi, p, &meters, &starts);
        w.close("part");
    }
    w.close("score-partwise");
    w.finish()
}

fn export_part(
    w: &mut XmlWriter,
    score: &Score,
    pi: usize,
    p: &Part,
    meters: &[Meter],
    starts: &[Tick],
) {
    let mut open_ties: HashMap<usize, HashSet<i32>> = HashMap::new();
    let number = |i: usize| ((i % 16) + 1).to_string();
    let mut prev_key: Option<KeySignature> = None;
    for (mi, m) in p.measures.iter().enumerate() {
        w.open("measure", &[("number", (mi + 1).to_string())]);
        if m.repeat_start {
            w.open("barline", &[("location", "left".into())]);
            w.leaf("bar-style", "heavy-light");
            w.empty("repeat", &[("direction", "forward".into())]);
            w.close("barline");
        }
        let key_changed = prev_key != Some(m.key);
        if mi == 0 || key_changed || m.meter.is_some() {
            w.open("attributes", &[]);
            if mi == 0 {
                w.leaf("divisions", DIVISIONS);
            }
            if key_changed {
                w.open("key", &[]);
                w.leaf("fifths", m.key.fifths);
                w.leaf(
                    "mode",
                    if m.key.mode == Mode::Minor {
                        "minor"
                    } else {
                        "major"
                    },
                );
                w.close("key");
            }
            if mi == 0 || m.meter.is_some() {
                let mt = meters[mi];
                w.open("time", &[]);
                w.leaf("beats", mt.numerator);
                w.leaf("beat-type", mt.denominator);
                w.close("time");
            }
            if mi == 0 {
                if p.staves.len() > 1 {
                    w.leaf("staves", p.staves.len());
                }
                for (si, st) in p.staves.iter().enumerate() {
                    let attrs = [("number", (si + 1).to_string())];
                    w.open("clef", &attrs);
                    if !st.standard && st.tab.is_some() {
                        w.leaf("sign", "TAB");
                        w.leaf("line", 5);
                    } else {
                        let (sign, line, oct) = clef_xml(st.clef);
                        w.leaf("sign", sign);
                        w.leaf("line", line);
                        if oct != 0 {
                            w.leaf("clef-octave-change", oct);
                        }
                    }
                    w.close("clef");
                }
                for (si, st) in p.staves.iter().enumerate() {
                    if let Some(tab) = &st.tab {
                        w.open("staff-details", &[("number", (si + 1).to_string())]);
                        w.leaf("staff-lines", tab.tuning.len());
                        let n = tab.tuning.len();
                        for (i, &midi) in tab.tuning.iter().enumerate().rev() {
                            let pt = Pitch::from_midi(midi, &KeySignature::c_major());
                            w.open("staff-tuning", &[("line", (n - i).to_string())]);
                            w.leaf("tuning-step", pt.step.letter());
                            if pt.alter != 0 {
                                w.leaf("tuning-alter", pt.alter);
                            }
                            w.leaf("tuning-octave", pt.octave);
                            w.close("staff-tuning");
                        }
                        if tab.capo > 0 {
                            w.leaf("capo", tab.capo);
                        }
                        w.close("staff-details");
                    }
                }
            }
            w.close("attributes");
        }
        prev_key = Some(m.key);
        if pi == 0 {
            for t in score.tempo.tempos() {
                if t.tick >= starts[mi] && t.tick < starts[mi + 1] {
                    w.open("direction", &[("placement", "above".into())]);
                    w.open("direction-type", &[]);
                    w.open("metronome", &[]);
                    w.leaf("beat-unit", "quarter");
                    w.leaf("per-minute", fmt_num(t.bpm));
                    w.close("metronome");
                    w.close("direction-type");
                    w.empty("sound", &[("tempo", fmt_num(t.bpm))]);
                    w.close("direction");
                }
            }
        }
        let mut written_prev = 0i64;
        for (vi, v) in m.voices.iter().enumerate() {
            if vi > 0 && written_prev > 0 {
                w.open("backup", &[]);
                w.leaf("duration", written_prev);
                w.close("backup");
            }
            let voice_no = (vi + 1).to_string();
            let staff_no = (v.staff as usize + 1).to_string();
            let multi_staff = p.staves.len() > 1;
            let mut pos = Frac::ZERO;
            let ties = open_ties.entry(vi).or_default();
            for e in &v.events {
                if let Some(d) = e.dynamic {
                    w.open("direction", &[("placement", "below".into())]);
                    w.open("direction-type", &[]);
                    w.open("dynamics", &[]);
                    w.empty(d.symbol(), &[]);
                    w.close("dynamics");
                    w.close("direction-type");
                    w.leaf("voice", &voice_no);
                    if multi_staff {
                        w.leaf("staff", &staff_no);
                    }
                    w.close("direction");
                }
                for (si, sp) in p.spanners.iter().enumerate() {
                    if sp.start == e.id && sp.kind != SpannerKind::Slur {
                        let ty = if sp.kind == SpannerKind::Crescendo {
                            "crescendo"
                        } else {
                            "diminuendo"
                        };
                        write_wedge(w, ty, &number(si));
                    }
                }
                let dur = if e.grace.is_some() {
                    0
                } else {
                    let d = (pos + e.duration.fraction()).to_ticks() - pos.to_ticks();
                    pos += e.duration.fraction();
                    d
                };
                let slur_starts: Vec<String> = p
                    .spanners
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.kind == SpannerKind::Slur && s.start == e.id)
                    .map(|(i, _)| number(i))
                    .collect();
                let slur_stops: Vec<String> = p
                    .spanners
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.kind == SpannerKind::Slur && s.end == e.id)
                    .map(|(i, _)| number(i))
                    .collect();
                let ctx = NoteCtx {
                    voice: &voice_no,
                    staff: multi_staff.then_some(staff_no.as_str()),
                    dur,
                    percussion: p.percussion,
                };
                match &e.kind {
                    EventKind::Rest => write_note(w, e, None, 0, &ctx, false, &[], &[]),
                    EventKind::Chord(notes) => {
                        for (ni, n) in notes.iter().enumerate() {
                            let midi = n.pitch.midi();
                            let stop = ties.remove(&midi);
                            if n.tie {
                                ties.insert(midi);
                            }
                            let (ss, se): (&[String], &[String]) = if ni == 0 {
                                (&slur_starts, &slur_stops)
                            } else {
                                (&[], &[])
                            };
                            write_note(w, e, Some(n), ni, &ctx, stop, ss, se);
                        }
                    }
                }
                for (si, sp) in p.spanners.iter().enumerate() {
                    if sp.end == e.id && sp.kind != SpannerKind::Slur {
                        write_wedge(w, "stop", &number(si));
                    }
                }
            }
            written_prev = pos.to_ticks();
        }
        if let Some(times) = m.repeat_end {
            w.open("barline", &[("location", "right".into())]);
            w.leaf("bar-style", "light-heavy");
            w.empty(
                "repeat",
                &[
                    ("direction", "backward".into()),
                    ("times", times.to_string()),
                ],
            );
            w.close("barline");
        }
        w.close("measure");
    }
}

fn write_wedge(w: &mut XmlWriter, ty: &str, number: &str) {
    w.open("direction", &[("placement", "below".into())]);
    w.open("direction-type", &[]);
    w.empty("wedge", &[("type", ty.into()), ("number", number.into())]);
    w.close("direction-type");
    w.close("direction");
}

struct NoteCtx<'a> {
    voice: &'a str,
    staff: Option<&'a str>,
    dur: i64,
    percussion: bool,
}

#[allow(clippy::too_many_arguments)]
fn write_note(
    w: &mut XmlWriter,
    e: &Event,
    note: Option<&Note>,
    index_in_chord: usize,
    ctx: &NoteCtx<'_>,
    tie_stop: bool,
    slur_starts: &[String],
    slur_stops: &[String],
) {
    w.open("note", &[]);
    match e.grace {
        Some(GraceKind::Acciaccatura) => w.empty("grace", &[("slash", "yes".into())]),
        Some(GraceKind::Appoggiatura) => w.empty("grace", &[]),
        None => {}
    }
    if index_in_chord > 0 {
        w.empty("chord", &[]);
    }
    match note {
        None => w.empty("rest", &[]),
        Some(n) if ctx.percussion => {
            w.open("unpitched", &[]);
            w.leaf("display-step", n.pitch.step.letter());
            w.leaf("display-octave", n.pitch.octave);
            w.close("unpitched");
        }
        Some(n) => {
            w.open("pitch", &[]);
            w.leaf("step", n.pitch.step.letter());
            if n.pitch.alter != 0 {
                w.leaf("alter", n.pitch.alter);
            }
            w.leaf("octave", n.pitch.octave);
            w.close("pitch");
        }
    }
    if e.grace.is_none() {
        w.leaf("duration", ctx.dur);
    }
    let tie_start = note.is_some_and(|n| n.tie);
    if tie_stop {
        w.empty("tie", &[("type", "stop".into())]);
    }
    if tie_start {
        w.empty("tie", &[("type", "start".into())]);
    }
    w.leaf("voice", ctx.voice);
    w.leaf("type", type_name(e.duration.value));
    for _ in 0..e.duration.dots {
        w.empty("dot", &[]);
    }
    if let Some(t) = e.duration.tuplet {
        w.open("time-modification", &[]);
        w.leaf("actual-notes", t.actual);
        w.leaf("normal-notes", t.normal);
        w.close("time-modification");
    }
    if let Some(s) = ctx.staff {
        w.leaf("staff", s);
    }
    let first = index_in_chord == 0;
    let arts: Vec<Articulation> = if first {
        e.articulations
            .iter()
            .copied()
            .filter(|a| *a != Articulation::Fermata)
            .collect()
    } else {
        Vec::new()
    };
    let fermata = first && e.articulations.contains(&Articulation::Fermata);
    let techs: &[Technique] = note.map_or(&[], |n| &n.techniques);
    let tab = note.and_then(|n| n.tab);
    let has_technical = tab.is_some()
        || techs
            .iter()
            .any(|t| !matches!(t, Technique::Slide | Technique::Vibrato));
    let has_notations = tie_stop
        || tie_start
        || !slur_starts.is_empty()
        || !slur_stops.is_empty()
        || !arts.is_empty()
        || fermata
        || has_technical
        || !techs.is_empty();
    if has_notations {
        w.open("notations", &[]);
        if tie_stop {
            w.empty("tied", &[("type", "stop".into())]);
        }
        if tie_start {
            w.empty("tied", &[("type", "start".into())]);
        }
        for n in slur_stops {
            w.empty("slur", &[("type", "stop".into()), ("number", n.clone())]);
        }
        for n in slur_starts {
            w.empty("slur", &[("type", "start".into()), ("number", n.clone())]);
        }
        if techs.contains(&Technique::Slide) {
            w.empty("slide", &[("type", "start".into()), ("number", "1".into())]);
        }
        if techs.contains(&Technique::Vibrato) {
            w.open("ornaments", &[]);
            w.empty("wavy-line", &[("type", "start".into())]);
            w.close("ornaments");
        }
        if has_technical {
            w.open("technical", &[]);
            for t in techs {
                match t {
                    Technique::HammerOn => {
                        w.leaf_with("hammer-on", &[("type", "start".into())], "H")
                    }
                    Technique::PullOff => w.leaf_with("pull-off", &[("type", "start".into())], "P"),
                    Technique::Bend { cents } => {
                        w.open("bend", &[]);
                        w.leaf("bend-alter", fmt_num(*cents as f64 / 100.0));
                        w.close("bend");
                    }
                    Technique::Harmonic { harmonic } => {
                        w.open("harmonic", &[]);
                        w.empty(
                            if *harmonic == HarmonicKind::Natural {
                                "natural"
                            } else {
                                "artificial"
                            },
                            &[],
                        );
                        w.close("harmonic");
                    }
                    Technique::PalmMute => w.leaf("other-technical", "palm mute"),
                    Technique::LetRing => w.leaf("other-technical", "let ring"),
                    Technique::Slide | Technique::Vibrato => {}
                }
            }
            if let Some(pos) = tab {
                w.leaf("string", pos.string);
                w.leaf("fret", pos.fret);
            }
            w.close("technical");
        }
        if !arts.is_empty() {
            w.open("articulations", &[]);
            for a in &arts {
                w.empty(articulation_tag(*a), &[]);
            }
            w.close("articulations");
        }
        if fermata {
            w.empty("fermata", &[]);
        }
        w.close("notations");
    }
    if let (true, Some(l), Some(_)) = (first, &e.lyric, note) {
        w.open("lyric", &[("number", "1".into())]);
        w.leaf(
            "syllabic",
            match l.syllabic {
                Syllabic::Single => "single",
                Syllabic::Begin => "begin",
                Syllabic::Middle => "middle",
                Syllabic::End => "end",
            },
        );
        w.leaf("text", &l.text);
        w.close("lyric");
    }
    w.close("note");
}

/// Exporta como `.mxl` (MusicXML compactado).
pub fn export_mxl(score: &Score) -> Result<Vec<u8>, IoError> {
    let xml = export_musicxml(score);
    let container = r#"<?xml version="1.0" encoding="UTF-8"?>
<container>
  <rootfiles>
    <rootfile full-path="score.musicxml" media-type="application/vnd.recordare.musicxml+xml"/>
  </rootfiles>
</container>
"#;
    write_zip(&[
        ("mimetype", b"application/vnd.recordare.musicxml", true),
        ("META-INF/container.xml", container.as_bytes(), false),
        ("score.musicxml", xml.as_bytes(), false),
    ])
}

// ───────────────────────────── importação ─────────────────────────────

/// Importa `.musicxml` ou `.mxl` (detecta zip pelo cabeçalho `PK`).
pub fn import_auto(bytes: &[u8]) -> Result<Imported, IoError> {
    if bytes.starts_with(b"PK") {
        import_mxl(bytes)
    } else {
        import_musicxml(bytes)
    }
}

/// Importa `.mxl` (lê `META-INF/container.xml` para achar o arquivo raiz).
pub fn import_mxl(bytes: &[u8]) -> Result<Imported, IoError> {
    let mut zip = Archive::open(bytes)?;
    let root_path = match zip.read("META-INF/container.xml") {
        Ok(c) => {
            let el = parse(&c)?;
            el.path("rootfiles/rootfile")
                .and_then(|r| r.attr("full-path"))
                .map(str::to_owned)
        }
        Err(_) => None,
    };
    let path = match root_path {
        Some(p) => p,
        None => zip
            .names()
            .into_iter()
            .find(|n| {
                !n.starts_with("META-INF") && (n.ends_with(".musicxml") || n.ends_with(".xml"))
            })
            .ok_or_else(|| IoError::malformed("mxl sem partitura"))?,
    };
    let xml = zip.read(&path)?;
    import_musicxml(&xml)
}

#[derive(Default)]
struct StaffInfo {
    clef: Option<Clef>,
    tab_clef: bool,
    lines: Option<usize>,
    tuning: Vec<(u32, u8)>,
    capo: u8,
}

struct PartReader<'a> {
    warnings: &'a mut Vec<String>,
    next_id: &'a mut u64,
    divisions: i64,
    key: KeySignature,
    meter: Meter,
    first_meter: Option<Meter>,
    staves: usize,
    staff_info: Vec<StaffInfo>,
    voice_map: HashMap<String, usize>,
    pending_dynamic: Option<Dynamic>,
    pending_wedges: Vec<(SpannerKind, String)>,
    open_wedges: HashMap<String, (SpannerKind, EventId)>,
    open_slurs: HashMap<String, EventId>,
    last_event: Option<EventId>,
    spanners: Vec<Spanner>,
    tempos: Vec<(usize, f64)>,
}

fn parse_meter(time: &Element) -> Option<Meter> {
    let beats: i64 = time
        .child_text("beats")?
        .split('+')
        .map(|b| b.trim().parse::<i64>().ok())
        .sum::<Option<i64>>()?;
    let bt: u8 = time.child_num("beat-type")?;
    Meter::new(u8::try_from(beats).ok()?, bt).ok()
}

fn parse_clef(c: &Element) -> (Option<Clef>, bool) {
    let sign = c.child_text("sign").unwrap_or("G");
    let line: u8 = c.child_num("line").unwrap_or(0);
    let oct: i8 = c.child_num("clef-octave-change").unwrap_or(0);
    match sign {
        "G" if oct < 0 => (Some(Clef::Treble8vb), false),
        "G" => (Some(Clef::Treble), false),
        "F" => (Some(Clef::Bass), false),
        "C" if line == 4 => (Some(Clef::Tenor), false),
        "C" => (Some(Clef::Alto), false),
        "percussion" => (Some(Clef::Percussion), false),
        "TAB" => (None, true),
        _ => (None, false),
    }
}

fn step_of(s: &str) -> Option<Step> {
    s.chars().next().and_then(Step::from_letter)
}

fn parse_alter(el: &Element, name: &str) -> i8 {
    el.child_text(name)
        .and_then(|t| t.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .map_or(0, |v| v.round().clamp(-2.0, 2.0) as i8)
}

impl PartReader<'_> {
    fn warn(&mut self, msg: impl Into<String>) {
        if self.warnings.len() < 200 {
            self.warnings.push(msg.into());
        }
    }

    fn read_attributes(&mut self, a: &Element, mi: usize, m: &mut Measure) {
        if let Some(d) = a.child_num::<i64>("divisions") {
            if (1..=1_000_000).contains(&d) {
                self.divisions = d;
            } else {
                self.warn("divisões inválidas ignoradas");
            }
        }
        if let Some(k) = a.child("key") {
            let fifths: i8 = k.child_num("fifths").unwrap_or(0);
            let mode = if k.child_text("mode") == Some("minor") {
                Mode::Minor
            } else {
                Mode::Major
            };
            match KeySignature::new(fifths, mode) {
                Ok(key) => {
                    self.key = key;
                    m.key = key;
                }
                Err(_) => self.warn("armadura inválida ignorada"),
            }
        }
        if let Some(t) = a.child("time") {
            match parse_meter(t) {
                Some(mt) if mi == 0 => {
                    self.first_meter = Some(mt);
                    self.meter = mt;
                }
                Some(mt) => {
                    if mt != self.meter {
                        m.meter = Some(mt);
                        self.meter = mt;
                    }
                }
                None => self.warn("fórmula de compasso não suportada ignorada"),
            }
        }
        if let Some(s) = a.child_num::<usize>("staves") {
            if s > MAX_STAVES {
                self.warn("mais de 2 pautas: excedentes unidas à 2ª");
            }
            self.staves = s.clamp(1, MAX_STAVES);
        }
        for c in a.children_named("clef") {
            let idx = staff_index(c.attr("number"), MAX_STAVES);
            let (clef, tab) = parse_clef(c);
            let info = &mut self.staff_info[idx];
            if clef.is_some() {
                info.clef = clef;
            }
            info.tab_clef = tab;
        }
        for d in a.children_named("staff-details") {
            let idx = staff_index(d.attr("number"), MAX_STAVES);
            let info = &mut self.staff_info[idx];
            info.lines = d.child_num("staff-lines");
            info.tuning.clear();
            for t in d.children_named("staff-tuning") {
                let line: u32 = t.attr("line").and_then(|l| l.parse().ok()).unwrap_or(0);
                let step = t.child_text("tuning-step").and_then(step_of);
                let oct: i8 = t.child_num("tuning-octave").unwrap_or(-10);
                if let Some(Ok(p)) =
                    step.map(|s| Pitch::new(s, parse_alter(t, "tuning-alter"), oct))
                {
                    info.tuning.push((line, p.midi() as u8));
                }
            }
            info.capo = d.child_num("capo").unwrap_or(0);
        }
    }

    fn read_direction(&mut self, d: &Element, mi: usize) {
        for dt in d.children_named("direction-type") {
            if let Some(dy) = dt.child("dynamics") {
                if let Some(x) = dy
                    .children
                    .iter()
                    .find_map(|c| Dynamic::from_symbol(&c.name))
                {
                    self.pending_dynamic = Some(x);
                }
            }
            if let Some(wg) = dt.child("wedge") {
                let num = wg.attr("number").unwrap_or("1").to_owned();
                match wg.attr("type") {
                    Some("crescendo") => self.pending_wedges.push((SpannerKind::Crescendo, num)),
                    Some("diminuendo") => self.pending_wedges.push((SpannerKind::Diminuendo, num)),
                    Some("stop") => {
                        if let (Some((kind, start)), Some(end)) =
                            (self.open_wedges.remove(&num), self.last_event)
                        {
                            self.spanners.push(Spanner { kind, start, end });
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(s) = d.child("sound") {
            self.read_sound(s, mi);
        }
    }

    fn read_sound(&mut self, s: &Element, mi: usize) {
        if let Some(t) = s.attr("tempo").and_then(|t| t.parse::<f64>().ok()) {
            self.tempos.push((mi, t));
        }
    }

    fn duration_of(&mut self, n: &Element) -> Option<Duration> {
        let tuplet = n.child("time-modification").and_then(|tm| {
            let a: u8 = tm.child_num("actual-notes")?;
            let b: u8 = tm.child_num("normal-notes")?;
            Tuplet::new(a, b).ok()
        });
        if let Some(v) = n.child_text("type").and_then(type_from_name) {
            let dots = n.children_named("dot").count().min(3) as u8;
            let mut d = Duration::new(v).dotted(dots);
            d.tuplet = tuplet;
            return Some(d);
        }
        let raw: i64 = n.child_num::<i64>("duration")?.clamp(0, 1 << 40);
        let f = Frac::new(raw, self.divisions * 4);
        if let Some(d) = Duration::from_fraction_exact(f) {
            return Some(d);
        }
        if let Some(d) = Duration::from_fraction_exact(f.mul(3, 2)) {
            return Some(d.with_tuplet(Tuplet::TRIPLET));
        }
        let (pieces, _) = Duration::decompose(f);
        self.warn("duração sem figura aproximada");
        pieces.first().copied()
    }

    fn read_note(&mut self, n: &Element, m: &mut Measure) {
        let Some(duration) = self.duration_of(n) else {
            self.warn("nota sem duração ignorada");
            return;
        };
        let voice_key = n.child_text("voice").unwrap_or("1").to_owned();
        let vidx = match self.voice_map.get(&voice_key) {
            Some(v) => *v,
            None if self.voice_map.len() < MAX_VOICES => {
                let v = self.voice_map.len();
                self.voice_map.insert(voice_key, v);
                v
            }
            None => {
                self.warn("mais de 4 vozes: nota descartada");
                return;
            }
        };
        let staff = n
            .child_num::<usize>("staff")
            .map_or(0, |s| s.saturating_sub(1).min(self.staves - 1)) as u8;
        while m.voices.len() <= vidx {
            m.voices.push(Voice::new(staff));
        }
        if m.voices[vidx].events.is_empty() {
            m.voices[vidx].staff = staff;
        }
        let is_chord = n.child("chord").is_some();
        let is_rest = n.child("rest").is_some();
        let pitch = if let Some(p) = n.child("pitch") {
            let step = p.child_text("step").and_then(step_of);
            let oct: i8 = p.child_num("octave").unwrap_or(4);
            step.and_then(|s| Pitch::new(s, parse_alter(p, "alter"), oct).ok())
        } else if let Some(u) = n.child("unpitched") {
            let step = u.child_text("display-step").and_then(step_of);
            let oct: i8 = u.child_num("display-octave").unwrap_or(4);
            step.and_then(|s| Pitch::new(s, 0, oct).ok())
        } else {
            None
        };
        if !is_rest && pitch.is_none() {
            self.warn("altura inválida: nota descartada");
            return;
        }
        let notations: Vec<&Element> = n.children_named("notations").collect();
        let find = |name: &str| -> Vec<&Element> {
            notations
                .iter()
                .flat_map(|x| x.children_named(name))
                .collect()
        };
        let tie = n
            .children_named("tie")
            .any(|t| t.attr("type") == Some("start"))
            || find("tied").iter().any(|t| t.attr("type") == Some("start"));
        let mut techniques = Vec::new();
        let mut tab = None;
        let mut articulations = Vec::new();
        for tech in find("technical") {
            let s: Option<u8> = tech.child_num("string");
            let f: Option<u8> = tech.child_num("fret");
            if let (Some(string), Some(fret)) = (s, f) {
                tab = Some(TabPosition { string, fret });
            }
            for c in &tech.children {
                match c.name.as_str() {
                    "hammer-on" if c.attr("type") != Some("stop") => {
                        techniques.push(Technique::HammerOn)
                    }
                    "pull-off" if c.attr("type") != Some("stop") => {
                        techniques.push(Technique::PullOff)
                    }
                    "bend" => {
                        let alter = c
                            .child_text("bend-alter")
                            .and_then(|t| t.parse::<f64>().ok())
                            .filter(|v| v.is_finite())
                            .unwrap_or(1.0);
                        techniques.push(Technique::Bend {
                            cents: (alter.abs() * 100.0).round().clamp(0.0, 1200.0) as u16,
                        });
                    }
                    "harmonic" => techniques.push(Technique::Harmonic {
                        harmonic: if c.child("artificial").is_some() {
                            HarmonicKind::Artificial
                        } else {
                            HarmonicKind::Natural
                        },
                    }),
                    "other-technical" => match c.text.trim().to_lowercase().as_str() {
                        "palm mute" | "p.m." | "pm" => techniques.push(Technique::PalmMute),
                        "let ring" => techniques.push(Technique::LetRing),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        if find("slide")
            .iter()
            .chain(find("glissando").iter())
            .any(|s| s.attr("type") == Some("start"))
        {
            techniques.push(Technique::Slide);
        }
        for o in find("ornaments") {
            if o.children.iter().any(|c| {
                (c.name == "wavy-line" && c.attr("type") == Some("start")) || c.name == "vibrato"
            }) {
                techniques.push(Technique::Vibrato);
            }
        }
        for a in find("articulations") {
            for c in &a.children {
                let art = match c.name.as_str() {
                    "staccato" => Some(Articulation::Staccato),
                    "accent" => Some(Articulation::Accent),
                    "tenuto" => Some(Articulation::Tenuto),
                    "strong-accent" => Some(Articulation::Marcato),
                    _ => None,
                };
                articulations.extend(art);
            }
        }
        if !find("fermata").is_empty() {
            articulations.push(Articulation::Fermata);
        }
        let slurs: Vec<(String, String)> = find("slur")
            .iter()
            .map(|s| {
                (
                    s.attr("type").unwrap_or("").to_owned(),
                    s.attr("number").unwrap_or("1").to_owned(),
                )
            })
            .collect();

        let voice = &mut m.voices[vidx];
        let note = pitch.map(|p| Note {
            pitch: p,
            tie,
            tab,
            techniques,
        });
        // nota de acorde: anexa ao último evento da voz
        if is_chord {
            if let (Some(last), Some(nn)) = (voice.events.last_mut(), note.clone()) {
                if let Some(notes) = last.notes_mut() {
                    if notes.len() < MAX_NOTES_PER_CHORD {
                        notes.push(nn);
                    }
                    for a in articulations {
                        if !last.articulations.contains(&a) {
                            last.articulations.push(a);
                        }
                    }
                    return;
                }
            }
        }
        if voice.events.len() >= MAX_EVENTS_PER_VOICE {
            self.warn("eventos demais na voz: descartado");
            return;
        }
        let id = EventId(*self.next_id);
        *self.next_id += 1;
        let mut ev = match note {
            Some(nn) => Event::chord(duration, vec![nn]),
            None => Event::rest(duration),
        };
        ev.id = id;
        articulations.sort();
        articulations.dedup();
        ev.articulations = articulations;
        if !ev.is_rest() {
            ev.grace = n.child("grace").map(|g| {
                if g.attr("slash") == Some("yes") {
                    GraceKind::Acciaccatura
                } else {
                    GraceKind::Appoggiatura
                }
            });
        }
        ev.dynamic = self.pending_dynamic.take();
        if let Some(l) = n.child("lyric") {
            if let Some(text) = l.child_text("text") {
                let syllabic = match l.child_text("syllabic") {
                    Some("begin") => Syllabic::Begin,
                    Some("middle") => Syllabic::Middle,
                    Some("end") => Syllabic::End,
                    _ => Syllabic::Single,
                };
                ev.lyric = Some(Lyric {
                    text: text.chars().take(200).collect(),
                    syllabic,
                });
            }
        }
        voice.events.push(ev);
        for (kind, num) in self.pending_wedges.drain(..) {
            self.open_wedges.insert(num, (kind, id));
        }
        for (ty, num) in slurs {
            match ty.as_str() {
                "start" => {
                    self.open_slurs.insert(num, id);
                }
                "stop" => {
                    if let Some(start) = self.open_slurs.remove(&num) {
                        self.spanners.push(Spanner {
                            kind: SpannerKind::Slur,
                            start,
                            end: id,
                        });
                    }
                }
                _ => {}
            }
        }
        self.last_event = Some(id);
    }

    fn forward(&mut self, f: &Element, m: &mut Measure) {
        let Some(raw) = f.child_num::<i64>("duration") else {
            return;
        };
        let Some(vk) = f.child_text("voice") else {
            return;
        };
        let Some(&vidx) = self.voice_map.get(vk) else {
            return;
        };
        while m.voices.len() <= vidx {
            m.voices.push(Voice::new(0));
        }
        for d in Duration::decompose(Frac::new(raw.clamp(0, 1 << 40), self.divisions * 4)).0 {
            let mut e = Event::rest(d);
            e.id = EventId(*self.next_id);
            *self.next_id += 1;
            m.voices[vidx].events.push(e);
        }
    }

    fn staves(&self) -> Vec<Staff> {
        (0..self.staves)
            .map(|i| {
                let info = &self.staff_info[i];
                let tab = if !info.tuning.is_empty() {
                    let mut t = info.tuning.clone();
                    t.sort_by_key(|x| std::cmp::Reverse(x.0));
                    Some(TabConfig {
                        tuning: t.into_iter().map(|(_, m)| m).take(MAX_STRINGS).collect(),
                        capo: info.capo,
                        frets: 24,
                    })
                } else if info.tab_clef {
                    Some(if info.lines == Some(4) {
                        TabConfig::standard_bass()
                    } else {
                        TabConfig::standard_guitar()
                    })
                } else {
                    None
                };
                let tab = tab.filter(|t| t.validate().is_ok());
                Staff {
                    clef: info.clef.unwrap_or(if info.tab_clef {
                        Clef::Treble8vb
                    } else {
                        Clef::Treble
                    }),
                    standard: !(info.tab_clef && tab.is_some()),
                    tab,
                }
            })
            .collect()
    }
}

fn staff_index(number: Option<&str>, max: usize) -> usize {
    number
        .and_then(|n| n.parse::<usize>().ok())
        .map_or(0, |n| n.saturating_sub(1).min(max - 1))
}

/// Importa MusicXML partwise (texto).
pub fn import_musicxml(bytes: &[u8]) -> Result<Imported, IoError> {
    check_size(bytes)?;
    let root = parse(bytes)?;
    match root.name.as_str() {
        "score-partwise" => {}
        "score-timewise" => {
            return Err(IoError::Unsupported("MusicXML timewise".into()));
        }
        other => return Err(IoError::malformed(format!("raiz inesperada: {other}"))),
    }
    let mut warnings = Vec::new();
    let title = root
        .path("work/work-title")
        .or_else(|| root.child("movement-title"))
        .map(|e| e.text.trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Sem título".into());
    let composer = root
        .child("identification")
        .and_then(|i| {
            i.children_named("creator")
                .find(|c| c.attr("type") == Some("composer"))
        })
        .map(|c| c.text.trim().to_owned())
        .unwrap_or_default();

    let mut part_info: HashMap<String, (String, u8, bool)> = HashMap::new();
    if let Some(pl) = root.child("part-list") {
        for sp in pl.children_named("score-part") {
            let id = sp.attr("id").unwrap_or("").to_owned();
            let name = sp.child_text("part-name").unwrap_or("").to_owned();
            let mi = sp.child("midi-instrument");
            let program = mi
                .and_then(|m| m.child_num::<u16>("midi-program"))
                .map_or(0, |p| p.clamp(1, 128) as u8 - 1);
            let perc = mi.and_then(|m| m.child_num::<u8>("midi-channel")) == Some(10);
            part_info.insert(id, (name, program, perc));
        }
    }

    let mut next_id = 1u64;
    let mut parts: Vec<Part> = Vec::new();
    let mut first_meter: Option<Meter> = None;
    let mut tempos: Vec<(usize, f64)> = Vec::new();
    for (pi, pel) in root.children_named("part").enumerate() {
        if parts.len() >= MAX_PARTS {
            warnings.push("partes demais: excedentes descartadas".into());
            break;
        }
        let (name, program, percussion) = pel
            .attr("id")
            .and_then(|id| part_info.get(id).cloned())
            .unwrap_or_else(|| (String::new(), 0, false));
        let mut r = PartReader {
            warnings: &mut warnings,
            next_id: &mut next_id,
            divisions: 1,
            key: KeySignature::c_major(),
            meter: Meter::COMMON,
            first_meter: None,
            staves: 1,
            staff_info: vec![StaffInfo::default(), StaffInfo::default()],
            voice_map: HashMap::new(),
            pending_dynamic: None,
            pending_wedges: Vec::new(),
            open_wedges: HashMap::new(),
            open_slurs: HashMap::new(),
            last_event: None,
            spanners: Vec::new(),
            tempos: Vec::new(),
        };
        let mut measures = Vec::new();
        for (mi, mel) in pel.children_named("measure").enumerate() {
            if mi >= MAX_MEASURES {
                r.warn("compassos demais: excedentes descartados");
                break;
            }
            let mut m = Measure::new(r.key);
            m.voices.clear();
            for c in &mel.children {
                match c.name.as_str() {
                    "attributes" => r.read_attributes(c, mi, &mut m),
                    "note" => r.read_note(c, &mut m),
                    "direction" => r.read_direction(c, mi),
                    "sound" => r.read_sound(c, mi),
                    "forward" => r.forward(c, &mut m),
                    "barline" => {
                        if let Some(rep) = c.child("repeat") {
                            match rep.attr("direction") {
                                Some("forward") => m.repeat_start = true,
                                Some("backward") => {
                                    let times = rep
                                        .attr("times")
                                        .and_then(|t| t.parse::<u8>().ok())
                                        .unwrap_or(2)
                                        .clamp(2, 16);
                                    m.repeat_end = Some(times);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            if m.voices.is_empty() {
                m.voices.push(Voice::new(0));
            }
            measures.push(m);
        }
        if measures.is_empty() {
            r.warn("parte sem compassos ignorada");
            continue;
        }
        let staves = r.staves();
        // tablatura em pauta sem afinação é descartada
        for m in &mut measures {
            for v in &mut m.voices {
                let tab = staves.get(v.staff as usize).and_then(|s| s.tab.clone());
                for e in &mut v.events {
                    if let Some(notes) = e.notes_mut() {
                        for n in notes {
                            if let Some(pos) = n.tab {
                                let ok = tab.as_ref().is_some_and(|t| {
                                    ctx_notation::tab::check_position(t, pos).is_ok()
                                });
                                if !ok {
                                    n.tab = None;
                                }
                            }
                        }
                    }
                }
            }
        }
        if pi == 0 {
            first_meter = r.first_meter;
            tempos = std::mem::take(&mut r.tempos);
        }
        let spanners = std::mem::take(&mut r.spanners);
        let name = if name.trim().is_empty() {
            format!("Parte {}", pi + 1)
        } else {
            name.chars().filter(|c| !c.is_control()).take(64).collect()
        };
        parts.push(Part {
            name,
            program,
            percussion,
            staves,
            measures,
            spanners,
        });
    }
    if parts.is_empty() {
        return Err(IoError::malformed("partitura sem partes"));
    }
    normalize_parts(&mut parts, &mut warnings);

    let meter = first_meter.unwrap_or(Meter::COMMON);
    let initial_bpm = tempos
        .iter()
        .find(|(mi, _)| *mi == 0)
        .map_or(120.0, |t| t.1)
        .clamp(20.0, 999.0);
    let mut score = Score::new(title, initial_bpm, meter)?;
    score.composer = composer;
    score.parts = parts;
    score.next_event_id = next_id;
    let starts = score.measure_starts();
    for (mi, bpm) in tempos {
        if mi == 0 {
            continue;
        }
        if let Some(t) = starts.get(mi) {
            if score.tempo.set_tempo(*t, bpm).is_err() {
                warnings.push(format!("andamento inválido ignorado: {bpm}"));
            }
        }
    }
    score.check_invariants()?;
    Ok(Imported { score, warnings })
}

/// Iguala número de compassos e atributos (armadura, fórmula, repetições) em todas as partes.
pub(crate) fn normalize_parts(parts: &mut [Part], warnings: &mut Vec<String>) {
    let n = parts.iter().map(|p| p.measures.len()).max().unwrap_or(0);
    let reference: Vec<Measure> = {
        let longest = parts
            .iter()
            .max_by_key(|p| p.measures.len())
            .expect("não vazio");
        longest.measures.clone()
    };
    let mut fixed = false;
    for p in parts.iter_mut() {
        while p.measures.len() < n {
            let r = &reference[p.measures.len()];
            let mut m = r.clone();
            m.voices = vec![Voice::new(0)];
            p.measures.push(m);
            fixed = true;
        }
        for (m, r) in p.measures.iter_mut().zip(&reference) {
            if !m.same_attributes(r) {
                m.key = r.key;
                m.meter = r.meter;
                m.repeat_start = r.repeat_start;
                m.repeat_end = r.repeat_end;
                fixed = true;
            }
        }
    }
    if fixed {
        warnings.push("estrutura de compassos diferente entre partes foi igualada".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names_roundtrip() {
        for v in NoteValue::ALL {
            assert_eq!(type_from_name(type_name(v)), Some(v));
        }
    }

    #[test]
    fn rejects_timewise_and_wrong_root() {
        assert!(matches!(
            import_musicxml(b"<score-timewise/>"),
            Err(IoError::Unsupported(_))
        ));
        assert!(import_musicxml(b"<html/>").is_err());
        assert!(import_musicxml(b"<score-partwise/>").is_err());
    }
}
