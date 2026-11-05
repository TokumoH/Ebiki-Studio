//! Comandos desfazíveis sobre o agregado [`Score`].
//!
//! - [`EditVoice`]: edições dentro de uma voz (inserir/remover/substituir evento, duração, altura,
//!   articulações, dinâmica, ligadura, tablatura, técnicas, letra, apojatura, completar com pausas).
//!   Aplicada sobre uma cópia do compasso e validada antes de entrar (atômica); o desfazer restaura o
//!   compasso, os spanners e o contador de ids.
//! - [`EditScore`]: edições estruturais (compassos, armadura, fórmula, repetição, partes, spanners,
//!   transposição, título, andamento), cada uma com inverso exato.

use crate::analysis::rests_to_fill;
use crate::error::NotationError;
use crate::model::*;
use crate::pitch::{KeySignature, Pitch};
use crate::tab;
use crate::transpose::{transpose, TransposeScope};
use shared_kernel::command::{Command, CommandError};
use shared_kernel::tempo::TempoMap;
use shared_kernel::time::Tick;

type R = Result<(), CommandError>;

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Edição dentro de uma voz. `index` é a posição do evento na voz; `note` a posição no acorde.
/// Serializável (`{"kind": "insert", "index": 0, "event": {...}}`) para o IPC.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum VoiceEdit {
    /// Insere em `index` (≤ len). Em uma voz inexistente (índice = nº de vozes) cria a voz na pauta 0.
    Insert {
        index: usize,
        event: Event,
    },
    Delete {
        index: usize,
    },
    /// Substitui o conteúdo mantendo o id (spanners continuam válidos).
    Replace {
        index: usize,
        event: Event,
    },
    SetDuration {
        index: usize,
        duration: crate::duration::Duration,
    },
    SetPitch {
        index: usize,
        note: usize,
        pitch: Pitch,
    },
    AddNote {
        index: usize,
        note: Note,
    },
    RemoveNote {
        index: usize,
        note: usize,
    },
    AddArticulation {
        index: usize,
        articulation: Articulation,
    },
    RemoveArticulation {
        index: usize,
        articulation: Articulation,
    },
    SetDynamic {
        index: usize,
        dynamic: Option<Dynamic>,
    },
    SetTie {
        index: usize,
        note: usize,
        tie: bool,
    },
    SetTab {
        index: usize,
        note: usize,
        tab: Option<TabPosition>,
    },
    AddTechnique {
        index: usize,
        note: usize,
        technique: Technique,
    },
    RemoveTechnique {
        index: usize,
        note: usize,
        technique: Technique,
    },
    SetLyric {
        index: usize,
        lyric: Option<Lyric>,
    },
    SetGrace {
        index: usize,
        grace: Option<GraceKind>,
    },
    /// Completa a voz com pausas até a capacidade do compasso.
    FillRests,
}

struct VoiceSnapshot {
    measure: Measure,
    spanners: Vec<Spanner>,
    next_event_id: u64,
}

pub struct EditVoice {
    pub loc: VoiceLoc,
    pub edit: VoiceEdit,
    previous: Option<VoiceSnapshot>,
}

impl EditVoice {
    pub fn new(loc: VoiceLoc, edit: VoiceEdit) -> Self {
        Self {
            loc,
            edit,
            previous: None,
        }
    }
}

fn ev_mut(v: &mut Voice, i: usize) -> Result<&mut Event, NotationError> {
    v.events.get_mut(i).ok_or(NotationError::EventNotFound(i))
}

fn note_mut(v: &mut Voice, i: usize, n: usize) -> Result<&mut Note, NotationError> {
    ev_mut(v, i)?
        .notes_mut()
        .ok_or(NotationError::NotAChord)?
        .get_mut(n)
        .ok_or(NotationError::NoteNotFound(n))
}

fn apply_voice_edit(
    score: &mut Score,
    loc: VoiceLoc,
    edit: &VoiceEdit,
) -> Result<(), NotationError> {
    let capacity = score.meter_at(loc.measure).capacity();
    let part = score.part(loc.part)?;
    let mut measure = part
        .measures
        .get(loc.measure)
        .ok_or(NotationError::MeasureNotFound(loc.measure))?
        .clone();
    let creating = loc.voice == measure.voices.len() && matches!(edit, VoiceEdit::Insert { .. });
    if creating {
        if measure.voices.len() >= MAX_VOICES {
            return Err(NotationError::TooManyVoices);
        }
        measure.voices.push(Voice::new(0));
    }
    let staff = measure
        .voices
        .get(loc.voice)
        .ok_or(NotationError::VoiceNotFound(loc.voice))?
        .staff;
    let tab_cfg = part.tab_for_staff(staff).cloned();
    let key = measure.key;
    let mut removed_id: Option<EventId> = None;
    let mut new_ids = 0u64;
    {
        let v = &mut measure.voices[loc.voice];
        match edit {
            VoiceEdit::Insert { index, event } => {
                if *index > v.events.len() {
                    return Err(NotationError::EventNotFound(*index));
                }
                let mut e = event.clone();
                e.id = EventId(score.next_event_id);
                new_ids = 1;
                v.events.insert(*index, e);
            }
            VoiceEdit::Delete { index } => {
                if *index >= v.events.len() {
                    return Err(NotationError::EventNotFound(*index));
                }
                removed_id = Some(v.events.remove(*index).id);
            }
            VoiceEdit::Replace { index, event } => {
                let slot = ev_mut(v, *index)?;
                let id = slot.id;
                *slot = Event {
                    id,
                    ..event.clone()
                };
            }
            VoiceEdit::SetDuration { index, duration } => {
                ev_mut(v, *index)?.duration = *duration;
            }
            VoiceEdit::SetPitch { index, note, pitch } => {
                let n = note_mut(v, *index, *note)?;
                n.pitch = *pitch;
                if let (Some(cfg), Some(old)) = (&tab_cfg, n.tab) {
                    let midi = pitch
                        .midi_note()
                        .ok_or(NotationError::PitchOutOfRange(pitch.midi()))?
                        .0;
                    if tab::pitch_of(cfg, old) != Ok(midi) {
                        n.tab = tab::best_position(cfg, midi, Some(old.fret));
                    }
                }
            }
            VoiceEdit::AddNote { index, note } => {
                let notes = ev_mut(v, *index)?
                    .notes_mut()
                    .ok_or(NotationError::NotAChord)?;
                notes.push(note.clone());
                notes.sort_by_key(|n| n.pitch.midi());
            }
            VoiceEdit::RemoveNote { index, note } => {
                let notes = ev_mut(v, *index)?
                    .notes_mut()
                    .ok_or(NotationError::NotAChord)?;
                if *note >= notes.len() {
                    return Err(NotationError::NoteNotFound(*note));
                }
                notes.remove(*note);
                if notes.is_empty() {
                    ev_mut(v, *index)?.kind = EventKind::Rest;
                }
            }
            VoiceEdit::AddArticulation {
                index,
                articulation,
            } => {
                let e = ev_mut(v, *index)?;
                if !e.articulations.contains(articulation) {
                    e.articulations.push(*articulation);
                    e.articulations.sort();
                }
            }
            VoiceEdit::RemoveArticulation {
                index,
                articulation,
            } => {
                ev_mut(v, *index)?
                    .articulations
                    .retain(|a| a != articulation);
            }
            VoiceEdit::SetDynamic { index, dynamic } => {
                ev_mut(v, *index)?.dynamic = *dynamic;
            }
            VoiceEdit::SetTie { index, note, tie } => {
                note_mut(v, *index, *note)?.tie = *tie;
            }
            VoiceEdit::SetTab {
                index,
                note,
                tab: pos,
            } => {
                let n = note_mut(v, *index, *note)?;
                if let Some(p) = pos {
                    let cfg = tab_cfg
                        .as_ref()
                        .ok_or(NotationError::InvalidTab("pauta sem tablatura"))?;
                    let midi = tab::pitch_of(cfg, *p)?;
                    // a altura acompanha a posição escolhida
                    n.pitch = Pitch::from_midi(midi, &key);
                }
                n.tab = *pos;
            }
            VoiceEdit::AddTechnique {
                index,
                note,
                technique,
            } => {
                let n = note_mut(v, *index, *note)?;
                n.techniques.retain(|t| !t.same_kind(technique));
                n.techniques.push(*technique);
            }
            VoiceEdit::RemoveTechnique {
                index,
                note,
                technique,
            } => {
                note_mut(v, *index, *note)?
                    .techniques
                    .retain(|t| !t.same_kind(technique));
            }
            VoiceEdit::SetLyric { index, lyric } => {
                ev_mut(v, *index)?.lyric = lyric.clone();
            }
            VoiceEdit::SetGrace { index, grace } => {
                ev_mut(v, *index)?.grace = *grace;
            }
            VoiceEdit::FillRests => {
                for d in rests_to_fill(v, capacity) {
                    let mut e = Event::rest(d);
                    e.id = EventId(score.next_event_id + new_ids);
                    new_ids += 1;
                    v.events.push(e);
                }
            }
        }
    }
    check_measure_content(part, &measure)?;
    score.next_event_id += new_ids;
    let p = score.part_mut(loc.part)?;
    p.measures[loc.measure] = measure;
    if let Some(id) = removed_id {
        p.spanners.retain(|s| s.start != id && s.end != id);
    }
    Ok(())
}

impl Command<Score> for EditVoice {
    fn apply(&mut self, s: &mut Score) -> R {
        let snap = VoiceSnapshot {
            measure: s
                .part(self.loc.part)?
                .measures
                .get(self.loc.measure)
                .ok_or(NotationError::MeasureNotFound(self.loc.measure))?
                .clone(),
            spanners: s.part(self.loc.part)?.spanners.clone(),
            next_event_id: s.next_event_id,
        };
        apply_voice_edit(s, self.loc, &self.edit)?;
        self.previous = Some(snap);
        Ok(())
    }

    fn revert(&mut self, s: &mut Score) -> R {
        let snap = self.previous.take().ok_or_else(missing)?;
        let p = s.part_mut(self.loc.part)?;
        p.measures[self.loc.measure] = snap.measure;
        p.spanners = snap.spanners;
        s.next_event_id = snap.next_event_id;
        Ok(())
    }

    fn label(&self) -> &str {
        match self.edit {
            VoiceEdit::Insert { .. } => "Inserir evento",
            VoiceEdit::Delete { .. } => "Remover evento",
            VoiceEdit::Replace { .. } => "Substituir evento",
            VoiceEdit::SetDuration { .. } => "Alterar duração",
            VoiceEdit::SetPitch { .. } => "Alterar altura",
            VoiceEdit::AddNote { .. } => "Adicionar nota ao acorde",
            VoiceEdit::RemoveNote { .. } => "Remover nota do acorde",
            VoiceEdit::AddArticulation { .. } => "Adicionar articulação",
            VoiceEdit::RemoveArticulation { .. } => "Remover articulação",
            VoiceEdit::SetDynamic {
                dynamic: Some(_), ..
            } => "Definir dinâmica",
            VoiceEdit::SetDynamic { dynamic: None, .. } => "Remover dinâmica",
            VoiceEdit::SetTie { .. } => "Ligadura de valor",
            VoiceEdit::SetTab { .. } => "Posição na tablatura",
            VoiceEdit::AddTechnique { .. } => "Adicionar técnica",
            VoiceEdit::RemoveTechnique { .. } => "Remover técnica",
            VoiceEdit::SetLyric { .. } => "Letra",
            VoiceEdit::SetGrace { .. } => "Apojatura",
            VoiceEdit::FillRests => "Completar com pausas",
        }
    }
}

/// Edição estrutural da partitura. Serializável (`{"kind": "setKey", ...}`) para o IPC.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum ScoreEdit {
    /// Insere um compasso vazio (pausas) em todas as partes.
    AddMeasure {
        index: usize,
    },
    RemoveMeasure {
        index: usize,
    },
    /// Define a armadura a partir de `measure` até a próxima mudança de armadura.
    SetKey {
        measure: usize,
        key: KeySignature,
    },
    SetMeter {
        measure: usize,
        meter: Option<Meter>,
    },
    SetRepeat {
        measure: usize,
        start: bool,
        end: Option<u8>,
    },
    AddPart {
        index: usize,
        part: Box<Part>,
    },
    RemovePart {
        index: usize,
    },
    AddSpanner {
        part: usize,
        spanner: Spanner,
    },
    RemoveSpanner {
        part: usize,
        index: usize,
    },
    Transpose {
        scope: TransposeScope,
        semitones: i32,
    },
    SetHeader {
        title: String,
        composer: String,
    },
    SetTempo {
        tick: Tick,
        bpm: f64,
    },
}

enum Undo {
    NextId(u64),
    Measures {
        index: usize,
        removed: Vec<(Measure, Vec<Spanner>)>,
        next_meter: Option<Option<Meter>>,
    },
    Keys(Vec<(usize, KeySignature)>),
    Meter(Option<Meter>),
    Repeat(bool, Option<u8>),
    PartAdded(u64),
    PartRemoved(Box<Part>),
    SpannerAdded,
    SpannerRemoved(Spanner),
    Parts(Vec<Part>),
    Header(String, String),
    Tempo(TempoMap),
}

pub struct EditScore {
    pub edit: ScoreEdit,
    undo: Option<Undo>,
}

impl EditScore {
    pub fn new(edit: ScoreEdit) -> Self {
        Self { edit, undo: None }
    }
}

fn for_all_measures(
    s: &mut Score,
    index: usize,
    f: impl Fn(&mut Measure),
) -> Result<(), NotationError> {
    if index >= s.measure_count() {
        return Err(NotationError::MeasureNotFound(index));
    }
    for p in &mut s.parts {
        f(&mut p.measures[index]);
    }
    Ok(())
}

fn apply_score_edit(s: &mut Score, edit: &ScoreEdit) -> Result<Undo, NotationError> {
    match edit {
        ScoreEdit::AddMeasure { index } => {
            let count = s.measure_count();
            if s.parts.is_empty() {
                return Err(NotationError::PartNotFound(0));
            }
            if *index > count {
                return Err(NotationError::MeasureNotFound(*index));
            }
            if count >= MAX_MEASURES {
                return Err(NotationError::LimitExceeded("compassos"));
            }
            let prev_id = s.next_event_id;
            let src = index.saturating_sub(1).min(count - 1);
            let capacity = s.meter_at(src).capacity();
            let reference = s.parts[0].measures[src].clone();
            let mut template = Measure::new(reference.key);
            if *index == 0 {
                template.meter = reference.meter;
            }
            for pi in 0..s.parts.len() {
                let staves = s.parts[pi].staves.len();
                let mut m = template.empty_like(capacity, staves);
                for v in &mut m.voices {
                    for e in &mut v.events {
                        e.id = s.alloc_event_id();
                    }
                }
                s.parts[pi].measures.insert(*index, m);
            }
            Ok(Undo::NextId(prev_id))
        }
        ScoreEdit::RemoveMeasure { index } => {
            let count = s.measure_count();
            if *index >= count {
                return Err(NotationError::MeasureNotFound(*index));
            }
            if count <= 1 {
                return Err(NotationError::LastMeasure);
            }
            let moved = s.parts[0].measures[*index].meter;
            let next_meter = (moved.is_some() && index + 1 < count)
                .then(|| s.parts[0].measures[index + 1].meter);
            let mut removed = Vec::new();
            for p in &mut s.parts {
                let m = p.measures.remove(*index);
                let ids: std::collections::HashSet<EventId> = m
                    .voices
                    .iter()
                    .flat_map(|v| v.events.iter().map(|e| e.id))
                    .collect();
                let spanners = p.spanners.clone();
                p.spanners
                    .retain(|sp| !ids.contains(&sp.start) && !ids.contains(&sp.end));
                if next_meter.is_some() && p.measures[*index].meter.is_none() {
                    p.measures[*index].meter = moved;
                }
                removed.push((m, spanners));
            }
            Ok(Undo::Measures {
                index: *index,
                removed,
                next_meter,
            })
        }
        ScoreEdit::SetKey { measure, key } => {
            KeySignature::new(key.fifths, key.mode)?;
            if *measure >= s.measure_count() {
                return Err(NotationError::MeasureNotFound(*measure));
            }
            let old = s.parts[0].measures[*measure].key;
            let mut prev = Vec::new();
            let mut i = *measure;
            while i < s.measure_count() && s.parts[0].measures[i].key == old {
                prev.push((i, old));
                for p in &mut s.parts {
                    p.measures[i].key = *key;
                }
                i += 1;
            }
            Ok(Undo::Keys(prev))
        }
        ScoreEdit::SetMeter { measure, meter } => {
            if let Some(m) = meter {
                Meter::new(m.numerator, m.denominator)?;
            }
            let prev = s
                .reference_measures()
                .get(*measure)
                .ok_or(NotationError::MeasureNotFound(*measure))?
                .meter;
            for_all_measures(s, *measure, |m| m.meter = *meter)?;
            Ok(Undo::Meter(prev))
        }
        ScoreEdit::SetRepeat {
            measure,
            start,
            end,
        } => {
            if end.is_some_and(|t| !(2..=16).contains(&t)) {
                return Err(NotationError::Inconsistent(
                    "repetição deve tocar 2–16 vezes",
                ));
            }
            let m0 = s
                .reference_measures()
                .get(*measure)
                .ok_or(NotationError::MeasureNotFound(*measure))?;
            let prev = (m0.repeat_start, m0.repeat_end);
            for_all_measures(s, *measure, |m| {
                m.repeat_start = *start;
                m.repeat_end = *end;
            })?;
            Ok(Undo::Repeat(prev.0, prev.1))
        }
        ScoreEdit::AddPart { index, part } => {
            if *index > s.parts.len() {
                return Err(NotationError::PartNotFound(*index));
            }
            s.check_part_fits(part)?;
            let prev = s.next_event_id;
            let mut p = (**part).clone();
            s.assign_ids(&mut p);
            s.parts.insert(*index, p);
            Ok(Undo::PartAdded(prev))
        }
        ScoreEdit::RemovePart { index } => {
            s.part(*index)?;
            Ok(Undo::PartRemoved(Box::new(s.parts.remove(*index))))
        }
        ScoreEdit::AddSpanner { part, spanner } => {
            let p = s.part(*part)?;
            let has = |id: EventId| p.events().any(|e| e.id == id);
            if !has(spanner.start) {
                return Err(NotationError::EventIdNotFound(spanner.start.0));
            }
            if !has(spanner.end) {
                return Err(NotationError::EventIdNotFound(spanner.end.0));
            }
            s.part_mut(*part)?.spanners.push(*spanner);
            Ok(Undo::SpannerAdded)
        }
        ScoreEdit::RemoveSpanner { part, index } => {
            let p = s.part_mut(*part)?;
            if *index >= p.spanners.len() {
                return Err(NotationError::Inconsistent("spanner inexistente"));
            }
            Ok(Undo::SpannerRemoved(p.spanners.remove(*index)))
        }
        ScoreEdit::Transpose { scope, semitones } => {
            let prev = s.parts.clone();
            transpose(s, scope, *semitones)?;
            Ok(Undo::Parts(prev))
        }
        ScoreEdit::SetHeader { title, composer } => {
            if title.chars().count() > 200 || composer.chars().count() > 200 {
                return Err(NotationError::InvalidName);
            }
            let prev = Undo::Header(
                std::mem::replace(&mut s.title, title.clone()),
                std::mem::replace(&mut s.composer, composer.clone()),
            );
            Ok(prev)
        }
        ScoreEdit::SetTempo { tick, bpm } => {
            let prev = s.tempo.clone();
            s.tempo
                .set_tempo(*tick, *bpm)
                .map_err(|_| NotationError::InvalidTempo)?;
            Ok(Undo::Tempo(prev))
        }
    }
}

impl Command<Score> for EditScore {
    fn apply(&mut self, s: &mut Score) -> R {
        self.undo = Some(apply_score_edit(s, &self.edit)?);
        Ok(())
    }

    fn revert(&mut self, s: &mut Score) -> R {
        match (self.undo.take().ok_or_else(missing)?, &self.edit) {
            (Undo::NextId(prev), ScoreEdit::AddMeasure { index }) => {
                for p in &mut s.parts {
                    p.measures.remove(*index);
                }
                s.next_event_id = prev;
            }
            (
                Undo::Measures {
                    index,
                    removed,
                    next_meter,
                },
                _,
            ) => {
                for (p, (m, spanners)) in s.parts.iter_mut().zip(removed) {
                    if let Some(prev) = next_meter {
                        p.measures[index].meter = prev;
                    }
                    p.measures.insert(index, m);
                    p.spanners = spanners;
                }
            }
            (Undo::Keys(prev), _) => {
                for (i, k) in prev {
                    for p in &mut s.parts {
                        p.measures[i].key = k;
                    }
                }
            }
            (Undo::Meter(prev), ScoreEdit::SetMeter { measure, .. }) => {
                for_all_measures(s, *measure, |m| m.meter = prev)?;
            }
            (Undo::Repeat(a, b), ScoreEdit::SetRepeat { measure, .. }) => {
                for_all_measures(s, *measure, |m| {
                    m.repeat_start = a;
                    m.repeat_end = b;
                })?;
            }
            (Undo::PartAdded(prev), ScoreEdit::AddPart { index, .. }) => {
                s.parts.remove(*index);
                s.next_event_id = prev;
            }
            (Undo::PartRemoved(p), ScoreEdit::RemovePart { index }) => {
                s.parts.insert(*index, *p);
            }
            (Undo::SpannerAdded, ScoreEdit::AddSpanner { part, .. }) => {
                s.part_mut(*part)?.spanners.pop();
            }
            (Undo::SpannerRemoved(sp), ScoreEdit::RemoveSpanner { part, index }) => {
                s.part_mut(*part)?.spanners.insert(*index, sp);
            }
            (Undo::Parts(parts), _) => s.parts = parts,
            (Undo::Header(t, c), _) => {
                s.title = t;
                s.composer = c;
            }
            (Undo::Tempo(t), _) => s.tempo = t,
            _ => return Err(missing()),
        }
        Ok(())
    }

    fn label(&self) -> &str {
        match self.edit {
            ScoreEdit::AddMeasure { .. } => "Adicionar compasso",
            ScoreEdit::RemoveMeasure { .. } => "Remover compasso",
            ScoreEdit::SetKey { .. } => "Alterar armadura",
            ScoreEdit::SetMeter { .. } => "Alterar fórmula de compasso",
            ScoreEdit::SetRepeat { .. } => "Repetição",
            ScoreEdit::AddPart { .. } => "Adicionar parte",
            ScoreEdit::RemovePart { .. } => "Remover parte",
            ScoreEdit::AddSpanner { .. } => "Adicionar ligadura/dinâmica",
            ScoreEdit::RemoveSpanner { .. } => "Remover ligadura/dinâmica",
            ScoreEdit::Transpose { .. } => "Transpor",
            ScoreEdit::SetHeader { .. } => "Título e compositor",
            ScoreEdit::SetTempo { .. } => "Andamento",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::validate_measures;
    use crate::duration::{Duration, NoteValue};
    use crate::pitch::{Mode, Step};
    use shared_kernel::command::History;

    fn p(step: Step, alter: i8, oct: i8) -> Pitch {
        Pitch::new(step, alter, oct).unwrap()
    }

    fn base() -> Score {
        let mut s = Score::new("Base", 100.0, Meter::COMMON).unwrap();
        let mut m = Measure::new(KeySignature::c_major());
        m.voices[0].events = vec![
            Event::note(Duration::quarter(), p(Step::C, 0, 4)),
            Event::note(Duration::quarter(), p(Step::E, 0, 4)),
            Event::chord(
                Duration::new(NoteValue::Half),
                vec![Note::new(p(Step::G, 0, 3)), Note::new(p(Step::B, 0, 3))],
            ),
        ];
        let m2 = Measure::new(KeySignature::c_major()).empty_like(Meter::COMMON.capacity(), 1);
        s.push_part(Part {
            name: "Violão".into(),
            program: 24,
            percussion: false,
            staves: vec![Staff::standard_and_tab(
                Clef::Treble8vb,
                TabConfig::standard_guitar(),
            )],
            measures: vec![m, m2],
            spanners: vec![],
        })
        .unwrap();
        s
    }

    /// apply → revert == antes; apply de novo == depois; invariantes válidas.
    fn roundtrip(mut s: Score, mut cmd: Box<dyn Command<Score>>) -> Score {
        let before = s.clone();
        cmd.apply(&mut s)
            .unwrap_or_else(|e| panic!("{}: {e}", cmd.label()));
        s.check_invariants().unwrap();
        let after = s.clone();
        cmd.revert(&mut s).unwrap();
        assert_eq!(s, before, "desfazer restaura: {}", cmd.label());
        cmd.apply(&mut s).unwrap();
        assert_eq!(s, after, "refazer reproduz: {}", cmd.label());
        s
    }

    const L0: VoiceLoc = VoiceLoc::new(0, 0, 0);

    fn ve(edit: VoiceEdit) -> Box<dyn Command<Score>> {
        Box::new(EditVoice::new(L0, edit))
    }

    #[test]
    fn insert_delete_replace_roundtrip() {
        let s = roundtrip(
            base(),
            ve(VoiceEdit::Insert {
                index: 1,
                event: Event::rest(Duration::quarter()),
            }),
        );
        assert_eq!(s.parts[0].measures[0].voices[0].events.len(), 4);
        assert!(s.parts[0].measures[0].voices[0].events[1].id.0 > 0);
        let s = roundtrip(base(), ve(VoiceEdit::Delete { index: 0 }));
        assert_eq!(s.parts[0].measures[0].voices[0].events.len(), 2);
        let s = roundtrip(
            base(),
            ve(VoiceEdit::Replace {
                index: 0,
                event: Event::note(Duration::quarter(), p(Step::D, 0, 4)),
            }),
        );
        let e = &s.parts[0].measures[0].voices[0].events[0];
        assert_eq!(e.notes()[0].pitch.name(), "D4");
        assert_eq!(
            e.id,
            base().parts[0].measures[0].voices[0].events[0].id,
            "id preservado"
        );
    }

    #[test]
    fn insert_into_new_voice_and_limit() {
        let s = roundtrip(
            base(),
            Box::new(EditVoice::new(
                VoiceLoc::new(0, 0, 1),
                VoiceEdit::Insert {
                    index: 0,
                    event: Event::rest(Duration::new(NoteValue::Whole)),
                },
            )),
        );
        assert_eq!(s.parts[0].measures[0].voices.len(), 2);
        let mut s = base();
        let mut bad = EditVoice::new(
            VoiceLoc::new(0, 0, 3),
            VoiceEdit::Insert {
                index: 0,
                event: Event::rest(Duration::quarter()),
            },
        );
        assert!(bad.apply(&mut s).is_err());
    }

    #[test]
    fn duration_pitch_articulation_dynamic_tie() {
        roundtrip(
            base(),
            ve(VoiceEdit::SetDuration {
                index: 0,
                duration: Duration::new(NoteValue::Eighth),
            }),
        );
        let s = roundtrip(
            base(),
            ve(VoiceEdit::SetPitch {
                index: 2,
                note: 1,
                pitch: p(Step::B, -1, 3),
            }),
        );
        assert_eq!(
            s.parts[0].measures[0].voices[0].events[2].notes()[1]
                .pitch
                .name(),
            "Bb3"
        );
        let s = roundtrip(
            base(),
            ve(VoiceEdit::AddArticulation {
                index: 0,
                articulation: Articulation::Accent,
            }),
        );
        assert_eq!(
            s.parts[0].measures[0].voices[0].events[0].articulations,
            vec![Articulation::Accent]
        );
        let mut s2 = s.clone();
        let mut rm = EditVoice::new(
            L0,
            VoiceEdit::RemoveArticulation {
                index: 0,
                articulation: Articulation::Accent,
            },
        );
        rm.apply(&mut s2).unwrap();
        assert!(s2.parts[0].measures[0].voices[0].events[0]
            .articulations
            .is_empty());
        rm.revert(&mut s2).unwrap();
        assert_eq!(s2, s);
        roundtrip(
            base(),
            ve(VoiceEdit::SetDynamic {
                index: 0,
                dynamic: Some(Dynamic::Ff),
            }),
        );
        let s = roundtrip(
            base(),
            ve(VoiceEdit::SetTie {
                index: 0,
                note: 0,
                tie: true,
            }),
        );
        assert!(s.parts[0].measures[0].voices[0].events[0].notes()[0].tie);
        // pausa não tem notas
        let mut s = base();
        let mut c = EditVoice::new(
            VoiceLoc::new(0, 1, 0),
            VoiceEdit::SetTie {
                index: 0,
                note: 0,
                tie: true,
            },
        );
        assert_eq!(c.apply(&mut s).unwrap_err().code, "notation.not_a_chord");
    }

    #[test]
    fn tab_and_techniques() {
        let s = roundtrip(
            base(),
            ve(VoiceEdit::SetTab {
                index: 0,
                note: 0,
                tab: Some(TabPosition { string: 2, fret: 1 }),
            }),
        );
        let n = &s.parts[0].measures[0].voices[0].events[0].notes()[0];
        assert_eq!(n.pitch.name(), "C4");
        let s = roundtrip(
            s,
            ve(VoiceEdit::AddTechnique {
                index: 0,
                note: 0,
                technique: Technique::Bend { cents: 200 },
            }),
        );
        let s = roundtrip(
            s,
            ve(VoiceEdit::AddTechnique {
                index: 0,
                note: 0,
                technique: Technique::Bend { cents: 100 },
            }),
        );
        assert_eq!(
            s.parts[0].measures[0].voices[0].events[0].notes()[0].techniques,
            vec![Technique::Bend { cents: 100 }],
            "mesma técnica substitui"
        );
        let s = roundtrip(
            s,
            ve(VoiceEdit::RemoveTechnique {
                index: 0,
                note: 0,
                technique: Technique::Bend { cents: 0 },
            }),
        );
        assert!(s.parts[0].measures[0].voices[0].events[0].notes()[0]
            .techniques
            .is_empty());
        // corda inexistente é rejeitada e nada muda
        let mut s2 = base();
        let before = s2.clone();
        let mut bad = EditVoice::new(
            L0,
            VoiceEdit::SetTab {
                index: 0,
                note: 0,
                tab: Some(TabPosition { string: 9, fret: 1 }),
            },
        );
        assert!(bad.apply(&mut s2).is_err());
        assert_eq!(s2, before);
        // alterar altura move a posição na tablatura
        let s = roundtrip(
            s,
            ve(VoiceEdit::SetPitch {
                index: 0,
                note: 0,
                pitch: p(Step::D, 0, 4),
            }),
        );
        let n = &s.parts[0].measures[0].voices[0].events[0].notes()[0];
        assert_eq!(
            tab::pitch_of(&TabConfig::standard_guitar(), n.tab.unwrap()).unwrap(),
            62
        );
    }

    #[test]
    fn notes_lyrics_grace_and_fill() {
        let s = roundtrip(
            base(),
            ve(VoiceEdit::AddNote {
                index: 0,
                note: Note::new(p(Step::G, 0, 4)),
            }),
        );
        assert_eq!(s.parts[0].measures[0].voices[0].events[0].notes().len(), 2);
        let s = roundtrip(base(), ve(VoiceEdit::RemoveNote { index: 0, note: 0 }));
        assert!(s.parts[0].measures[0].voices[0].events[0].is_rest());
        roundtrip(
            base(),
            ve(VoiceEdit::SetLyric {
                index: 0,
                lyric: Some(Lyric {
                    text: "lá".into(),
                    syllabic: Syllabic::Single,
                }),
            }),
        );
        roundtrip(
            base(),
            ve(VoiceEdit::SetGrace {
                index: 0,
                grace: Some(GraceKind::Acciaccatura),
            }),
        );
        let mut s = base();
        EditVoice::new(L0, VoiceEdit::Delete { index: 2 })
            .apply(&mut s)
            .unwrap();
        assert!(!validate_measures(&s).is_empty());
        let s = roundtrip(s, ve(VoiceEdit::FillRests));
        assert!(validate_measures(&s).is_empty());
    }

    #[test]
    fn delete_removes_spanners_and_undo_restores() {
        let mut s = base();
        let ids: Vec<EventId> = s.parts[0].events().map(|e| e.id).collect();
        let mut add = EditScore::new(ScoreEdit::AddSpanner {
            part: 0,
            spanner: Spanner {
                kind: SpannerKind::Slur,
                start: ids[0],
                end: ids[1],
            },
        });
        add.apply(&mut s).unwrap();
        let s = roundtrip(s, ve(VoiceEdit::Delete { index: 0 }));
        assert!(s.parts[0].spanners.is_empty());
        let mut s = base();
        let mut bad = EditScore::new(ScoreEdit::AddSpanner {
            part: 0,
            spanner: Spanner {
                kind: SpannerKind::Crescendo,
                start: EventId(999),
                end: ids[1],
            },
        });
        assert!(bad.apply(&mut s).is_err());
        let mut add = EditScore::new(ScoreEdit::AddSpanner {
            part: 0,
            spanner: Spanner {
                kind: SpannerKind::Crescendo,
                start: ids[0],
                end: ids[2],
            },
        });
        add.apply(&mut s).unwrap();
        roundtrip(
            s,
            Box::new(EditScore::new(ScoreEdit::RemoveSpanner {
                part: 0,
                index: 0,
            })),
        );
    }

    #[test]
    fn measures_roundtrip() {
        let s = roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::AddMeasure { index: 1 })),
        );
        assert_eq!(s.measure_count(), 3);
        assert!(validate_measures(&s).is_empty());
        roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::AddMeasure { index: 0 })),
        );
        roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::AddMeasure { index: 2 })),
        );
        let s = roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::RemoveMeasure { index: 0 })),
        );
        assert_eq!(s.measure_count(), 1);
        let mut one = s.clone();
        assert_eq!(
            EditScore::new(ScoreEdit::RemoveMeasure { index: 0 })
                .apply(&mut one)
                .unwrap_err()
                .code,
            "notation.last_measure"
        );
        // remover compasso com mudança de fórmula leva a fórmula ao seguinte
        let mut s = base();
        EditScore::new(ScoreEdit::AddMeasure { index: 2 })
            .apply(&mut s)
            .unwrap();
        EditScore::new(ScoreEdit::SetMeter {
            measure: 1,
            meter: Some(Meter::new(3, 4).unwrap()),
        })
        .apply(&mut s)
        .unwrap();
        let s = roundtrip(
            s,
            Box::new(EditScore::new(ScoreEdit::RemoveMeasure { index: 1 })),
        );
        assert_eq!(s.meter_at(1), Meter::new(3, 4).unwrap());
    }

    #[test]
    fn key_meter_repeat_roundtrip() {
        let s = roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::SetKey {
                measure: 0,
                key: KeySignature::new(-1, Mode::Minor).unwrap(),
            })),
        );
        assert!(s.parts[0].measures.iter().all(|m| m.key.fifths == -1));
        let s = roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::SetMeter {
                measure: 1,
                meter: Some(Meter::new(6, 8).unwrap()),
            })),
        );
        assert_eq!(s.meter_at(1), Meter::new(6, 8).unwrap());
        assert!(
            !validate_measures(&s).is_empty(),
            "conteúdo antigo não cabe em 6/8"
        );
        roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::SetRepeat {
                measure: 1,
                start: false,
                end: Some(2),
            })),
        );
        let mut s = base();
        assert!(EditScore::new(ScoreEdit::SetRepeat {
            measure: 1,
            start: false,
            end: Some(1)
        })
        .apply(&mut s)
        .is_err());
        assert!(EditScore::new(ScoreEdit::SetKey {
            measure: 0,
            key: KeySignature {
                fifths: 9,
                mode: Mode::Major
            }
        })
        .apply(&mut s)
        .is_err());
    }

    #[test]
    fn parts_roundtrip() {
        let s0 = base();
        let blank = s0.blank_part("Baixo", 33, vec![Staff::standard(Clef::Bass)]);
        let s = roundtrip(
            s0.clone(),
            Box::new(EditScore::new(ScoreEdit::AddPart {
                index: 0,
                part: Box::new(blank),
            })),
        );
        assert_eq!(s.parts[0].name, "Baixo");
        let s = roundtrip(
            s,
            Box::new(EditScore::new(ScoreEdit::RemovePart { index: 1 })),
        );
        assert_eq!(s.parts.len(), 1);
        let mut bad = s0.parts[0].clone();
        bad.measures.pop();
        let mut s1 = s0.clone();
        assert!(EditScore::new(ScoreEdit::AddPart {
            index: 0,
            part: Box::new(bad)
        })
        .apply(&mut s1)
        .is_err());
        assert_eq!(s1, s0);
    }

    #[test]
    fn transpose_header_tempo_roundtrip() {
        let s = roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::Transpose {
                scope: TransposeScope::Score,
                semitones: 2,
            })),
        );
        assert_eq!(s.parts[0].measures[0].key.fifths, 2);
        roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::Transpose {
                scope: TransposeScope::Part(0),
                semitones: -3,
            })),
        );
        let ids = vec![base().parts[0].measures[0].voices[0].events[0].id];
        roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::Transpose {
                scope: TransposeScope::Events(ids),
                semitones: 1,
            })),
        );
        roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::SetHeader {
                title: "Nova".into(),
                composer: "Eu".into(),
            })),
        );
        let s = roundtrip(
            base(),
            Box::new(EditScore::new(ScoreEdit::SetTempo {
                tick: Tick(3840),
                bpm: 80.0,
            })),
        );
        assert_eq!(s.tempo.bpm_at(Tick(4000)), 80.0);
    }

    #[test]
    fn works_with_history() {
        let mut s = base();
        let before = s.clone();
        let mut h = History::new();
        h.execute(&mut s, ve(VoiceEdit::Delete { index: 0 }), None)
            .unwrap();
        h.execute(
            &mut s,
            Box::new(EditScore::new(ScoreEdit::AddMeasure { index: 2 })),
            None,
        )
        .unwrap();
        h.execute(
            &mut s,
            ve(VoiceEdit::Insert {
                index: 0,
                event: Event::rest(Duration::quarter()),
            }),
            None,
        )
        .unwrap();
        let after = s.clone();
        while h.undo(&mut s).unwrap().is_some() {}
        assert_eq!(s, before);
        while h.redo(&mut s).unwrap().is_some() {}
        assert_eq!(s, after);
    }
}
