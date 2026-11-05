//! Afinação de um projeto ([`TuningBook`]): um mapa de correção por fonte de áudio + ajustes do
//! afinador. Comandos adaptam os comandos de [`PitchCorrectionMap`]/[`TunerSettings`] para o livro.

use crate::domain::*;
use serde::{Deserialize, Serialize};
use shared_kernel::command::{Command, CommandError};
use shared_kernel::id::AudioSourceId;
use std::collections::BTreeMap;

/// Máximo de notas detectadas por fonte (proteção contra análises patológicas).
pub const MAX_NOTES_PER_SOURCE: usize = 20_000;

/// Mapas de correção por fonte e ajustes do afinador.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuningBook {
    maps: BTreeMap<AudioSourceId, PitchCorrectionMap>,
    pub tuner: TunerSettings,
}

impl PitchCorrectionMap {
    /// Reconstrói um mapa com identidade conhecida (persistência), validando tudo.
    pub fn from_parts(
        id: PitchCorrectionMapId,
        source: AudioSourceId,
        notes: Vec<DetectedNote>,
        edits: BTreeMap<DetectedNoteId, NoteEdit>,
        scale: ScaleCorrection,
    ) -> Result<Self, TuningError> {
        let mut m = Self::new(source);
        m.id = id;
        m.replace_notes(notes)?;
        for (note, edit) in edits {
            m.set_edit(note, edit)?;
        }
        m.set_scale(scale)?;
        Ok(m)
    }
}

impl TuningBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstrói (persistência): cada mapa é guardado pela sua fonte.
    pub fn from_parts(maps: Vec<PitchCorrectionMap>, tuner: TunerSettings) -> Self {
        Self {
            maps: maps.into_iter().map(|m| (m.source, m)).collect(),
            tuner,
        }
    }

    pub fn maps(&self) -> &BTreeMap<AudioSourceId, PitchCorrectionMap> {
        &self.maps
    }

    pub fn map(&self, source: AudioSourceId) -> Option<&PitchCorrectionMap> {
        self.maps.get(&source)
    }

    fn map_mut(&mut self, source: AudioSourceId) -> Result<&mut PitchCorrectionMap, CommandError> {
        self.maps.get_mut(&source).ok_or_else(|| {
            CommandError::new(
                "tuning.not_analyzed",
                "Esta fonte de áudio ainda não foi analisada.",
            )
        })
    }

    /// Define (ou remove, com `None`) o mapa de uma fonte. Retorna o anterior.
    pub fn put(
        &mut self,
        source: AudioSourceId,
        map: Option<PitchCorrectionMap>,
    ) -> Result<Option<PitchCorrectionMap>, TuningError> {
        Ok(match map {
            Some(m) => {
                m.validate()?;
                if m.source != source {
                    return Err(TuningError::InvalidAmount("fonte do mapa"));
                }
                if m.notes().len() > MAX_NOTES_PER_SOURCE {
                    return Err(TuningError::InvalidAmount("número de notas"));
                }
                self.maps.insert(source, m)
            }
            None => self.maps.remove(&source),
        })
    }
}

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Cria, substitui ou remove o mapa de correção de uma fonte.
pub struct PutCorrectionMap {
    source: AudioSourceId,
    map: Option<PitchCorrectionMap>,
    previous: Option<Option<PitchCorrectionMap>>,
    label: &'static str,
}

impl PutCorrectionMap {
    pub fn new(source: AudioSourceId, map: Option<PitchCorrectionMap>) -> Self {
        let label = if map.is_some() {
            "Analisar afinação"
        } else {
            "Limpar análise de afinação"
        };
        Self {
            source,
            map,
            previous: None,
            label,
        }
    }
}

impl Command<TuningBook> for PutCorrectionMap {
    fn apply(&mut self, b: &mut TuningBook) -> Result<(), CommandError> {
        self.previous = Some(b.put(self.source, self.map.clone())?);
        Ok(())
    }
    fn revert(&mut self, b: &mut TuningBook) -> Result<(), CommandError> {
        let prev = self.previous.take().ok_or_else(missing)?;
        b.put(self.source, prev)?;
        Ok(())
    }
    fn label(&self) -> &str {
        self.label
    }
}

/// Aplica um comando de mapa (ex.: [`crate::SetNoteEdit`]) ao mapa da fonte `source`.
pub struct OnCorrectionMap {
    pub source: AudioSourceId,
    inner: Box<dyn Command<PitchCorrectionMap>>,
}

impl OnCorrectionMap {
    pub fn new(source: AudioSourceId, inner: impl Command<PitchCorrectionMap> + 'static) -> Self {
        Self {
            source,
            inner: Box::new(inner),
        }
    }
}

impl Command<TuningBook> for OnCorrectionMap {
    fn apply(&mut self, b: &mut TuningBook) -> Result<(), CommandError> {
        self.inner.apply(b.map_mut(self.source)?)
    }
    fn revert(&mut self, b: &mut TuningBook) -> Result<(), CommandError> {
        self.inner.revert(b.map_mut(self.source)?)
    }
    fn label(&self) -> &str {
        self.inner.label()
    }
}

/// Aplica um comando dos ajustes do afinador (ex.: [`crate::SetTunerReference`]).
pub struct OnTuner {
    inner: Box<dyn Command<TunerSettings>>,
}

impl OnTuner {
    pub fn new(inner: impl Command<TunerSettings> + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl Command<TuningBook> for OnTuner {
    fn apply(&mut self, b: &mut TuningBook) -> Result<(), CommandError> {
        self.inner.apply(&mut b.tuner)
    }
    fn revert(&mut self, b: &mut TuningBook) -> Result<(), CommandError> {
        self.inner.revert(&mut b.tuner)
    }
    fn label(&self) -> &str {
        self.inner.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{SetNoteEdit, SetTunerReference};
    use shared_kernel::command::History;
    use shared_kernel::music::MidiNote;

    fn analyzed(source: AudioSourceId) -> PitchCorrectionMap {
        let mut m = PitchCorrectionMap::new(source);
        m.replace_notes(vec![
            DetectedNote::new(0, 1000, 60.3).unwrap(),
            DetectedNote::new(1000, 2000, 64.0).unwrap(),
        ])
        .unwrap();
        m
    }

    #[test]
    fn put_edit_and_tuner_roundtrip_through_history() {
        let src = AudioSourceId::new();
        let mut b = TuningBook::new();
        let mut h = History::new();
        let map = analyzed(src);
        let note = map.notes()[0].id;
        h.execute(
            &mut b,
            Box::new(PutCorrectionMap::new(src, Some(map))),
            None,
        )
        .unwrap();
        h.execute(
            &mut b,
            Box::new(OnCorrectionMap::new(
                src,
                SetNoteEdit::new(note, NoteEdit::to_target(MidiNote(60))),
            )),
            None,
        )
        .unwrap();
        h.execute(
            &mut b,
            Box::new(OnTuner::new(SetTunerReference::new(432.0))),
            None,
        )
        .unwrap();
        let done = b.clone();
        assert!((b.tuner.a4_hz() - 432.0).abs() < 1e-9);
        assert!(b.map(src).unwrap().edit(note).is_some());
        for _ in 0..3 {
            h.undo(&mut b).unwrap();
        }
        assert_eq!(b, TuningBook::new());
        for _ in 0..3 {
            h.redo(&mut b).unwrap();
        }
        assert_eq!(b, done);
    }

    #[test]
    fn editing_a_source_without_analysis_fails_cleanly() {
        let mut b = TuningBook::new();
        let mut cmd = OnCorrectionMap::new(
            AudioSourceId::new(),
            SetNoteEdit::new(DetectedNoteId::new(), NoteEdit::default()),
        );
        assert_eq!(cmd.apply(&mut b).unwrap_err().code, "tuning.not_analyzed");
    }

    #[test]
    fn from_parts_keeps_identity_and_validates() {
        let src = AudioSourceId::new();
        let m = analyzed(src);
        let mut edits = BTreeMap::new();
        edits.insert(m.notes()[1].id, NoteEdit::to_target(MidiNote(65)));
        let r = PitchCorrectionMap::from_parts(
            m.id,
            src,
            m.notes().to_vec(),
            edits.clone(),
            ScaleCorrection::major(shared_kernel::music::PitchClass::new(0).unwrap()),
        )
        .unwrap();
        assert_eq!(r.id, m.id);
        assert_eq!(r.edits(), &edits);
        edits.insert(DetectedNoteId::new(), NoteEdit::to_target(MidiNote(1)));
        assert!(PitchCorrectionMap::from_parts(
            m.id,
            src,
            m.notes().to_vec(),
            edits,
            ScaleCorrection::default()
        )
        .is_err());
    }
}
