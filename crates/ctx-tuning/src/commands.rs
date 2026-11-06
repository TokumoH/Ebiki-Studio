//! Comandos desfazíveis do contexto Tuning.

use crate::domain::*;
use shared_kernel::command::{Command, CommandError};
use std::collections::BTreeMap;

type R = Result<(), CommandError>;

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Substitui as notas detectadas (resultado de nova análise).
pub struct ReplaceDetectedNotes {
    notes: Vec<DetectedNote>,
    previous: Option<(Vec<DetectedNote>, BTreeMap<DetectedNoteId, NoteEdit>)>,
}

impl ReplaceDetectedNotes {
    pub fn new(notes: Vec<DetectedNote>) -> Self {
        Self {
            notes,
            previous: None,
        }
    }
}

impl Command<PitchCorrectionMap> for ReplaceDetectedNotes {
    fn apply(&mut self, m: &mut PitchCorrectionMap) -> R {
        self.previous = Some(m.replace_notes(self.notes.clone())?);
        Ok(())
    }
    fn revert(&mut self, m: &mut PitchCorrectionMap) -> R {
        let (notes, edits) = self.previous.take().ok_or_else(missing)?;
        m.restore(notes, edits);
        Ok(())
    }
    fn label(&self) -> &str {
        "Detectar notas"
    }
}

/// Define (ou limpa, com edição neutra) a edição de uma nota.
pub struct SetNoteEdit {
    pub note: DetectedNoteId,
    pub edit: NoteEdit,
    previous: Option<Option<NoteEdit>>,
}

impl SetNoteEdit {
    pub fn new(note: DetectedNoteId, edit: NoteEdit) -> Self {
        Self {
            note,
            edit,
            previous: None,
        }
    }

    /// Remove a edição da nota (volta ao detectado).
    pub fn clear(note: DetectedNoteId) -> Self {
        Self::new(note, NoteEdit::default())
    }
}

impl Command<PitchCorrectionMap> for SetNoteEdit {
    fn apply(&mut self, m: &mut PitchCorrectionMap) -> R {
        self.previous = Some(m.set_edit(self.note, self.edit)?);
        Ok(())
    }
    fn revert(&mut self, m: &mut PitchCorrectionMap) -> R {
        let prev = self.previous.take().ok_or_else(missing)?;
        m.restore_edit(self.note, prev);
        Ok(())
    }
    fn label(&self) -> &str {
        if self.edit.is_neutral() {
            "Limpar edição da nota"
        } else {
            "Editar nota"
        }
    }
}

/// Altera a correção automática por escala.
pub struct SetScaleCorrection {
    scale: ScaleCorrection,
    previous: Option<ScaleCorrection>,
}

impl SetScaleCorrection {
    pub fn new(scale: ScaleCorrection) -> Self {
        Self {
            scale,
            previous: None,
        }
    }
}

impl Command<PitchCorrectionMap> for SetScaleCorrection {
    fn apply(&mut self, m: &mut PitchCorrectionMap) -> R {
        self.previous = Some(m.set_scale(self.scale.clone())?);
        Ok(())
    }
    fn revert(&mut self, m: &mut PitchCorrectionMap) -> R {
        m.set_scale(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Correção por escala"
    }
}

/// Altera a referência do Lá4 do afinador.
pub struct SetTunerReference {
    pub a4_hz: f64,
    previous: Option<f64>,
}

impl SetTunerReference {
    pub fn new(a4_hz: f64) -> Self {
        Self {
            a4_hz,
            previous: None,
        }
    }
}

impl Command<TunerSettings> for SetTunerReference {
    fn apply(&mut self, t: &mut TunerSettings) -> R {
        self.previous = Some(t.set_a4(self.a4_hz)?);
        Ok(())
    }
    fn revert(&mut self, t: &mut TunerSettings) -> R {
        t.set_a4(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Referência do afinador"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use shared_kernel::command::History;
    use shared_kernel::id::AudioSourceId;
    use shared_kernel::music::{MidiNote, PitchClass};

    fn notes() -> Vec<DetectedNote> {
        vec![
            DetectedNote::new(0, 1000, 60.3).unwrap(),
            DetectedNote::new(1000, 2000, 64.0).unwrap(),
        ]
    }

    /// Aplica → desfaz → refaz e compara estados.
    fn roundtrip<T: Clone + PartialEq + std::fmt::Debug + 'static>(
        target: &mut T,
        cmd: Box<dyn Command<T>>,
    ) {
        let before = target.clone();
        let mut h = History::new();
        h.execute(target, cmd, None).unwrap();
        let after = target.clone();
        assert_ne!(before, after, "o comando deveria mudar o estado");
        h.undo(target).unwrap();
        assert_eq!(*target, before);
        h.redo(target).unwrap();
        assert_eq!(*target, after);
    }

    #[test]
    fn replace_notes_roundtrip_restores_edits() {
        let mut m = PitchCorrectionMap::new(AudioSourceId::new());
        let ns = notes();
        m.replace_notes(ns.clone()).unwrap();
        m.set_edit(ns[0].id, NoteEdit::to_target(MidiNote(60)))
            .unwrap();
        let fresh = vec![DetectedNote::new(0, 500, 59.9).unwrap()];
        roundtrip(&mut m, Box::new(ReplaceDetectedNotes::new(fresh)));
        assert!(m.edits().is_empty(), "após refazer, edições órfãs somem");
    }

    #[test]
    fn set_and_clear_note_edit_roundtrip() {
        let mut m = PitchCorrectionMap::new(AudioSourceId::new());
        let ns = notes();
        m.replace_notes(ns.clone()).unwrap();
        roundtrip(
            &mut m,
            Box::new(SetNoteEdit::new(
                ns[1].id,
                NoteEdit {
                    cents_offset: 25.0,
                    flatten: 0.5,
                    ..Default::default()
                },
            )),
        );
        roundtrip(&mut m, Box::new(SetNoteEdit::clear(ns[1].id)));
        assert_eq!(
            SetNoteEdit::clear(ns[1].id).label(),
            "Limpar edição da nota"
        );
    }

    #[test]
    fn scale_and_tuner_roundtrip() {
        let mut m = PitchCorrectionMap::new(AudioSourceId::new());
        let mut s = ScaleCorrection::major(PitchClass::parse("G").unwrap());
        s.enabled = true;
        roundtrip(&mut m, Box::new(SetScaleCorrection::new(s.clone())));
        assert_eq!(m.scale(), &s, "estado após refazer = escala aplicada");
        let mut t = TunerSettings::default();
        roundtrip(&mut t, Box::new(SetTunerReference::new(442.0)));
        assert!((t.a4_hz() - 442.0).abs() < 1e-9);
    }

    #[test]
    fn failing_commands_change_nothing() {
        let mut m = PitchCorrectionMap::new(AudioSourceId::new());
        let before = m.clone();
        let mut h = History::new();
        let err = h
            .execute(
                &mut m,
                Box::new(SetNoteEdit::new(
                    DetectedNoteId::new(),
                    NoteEdit::to_target(MidiNote(60)),
                )),
                None,
            )
            .unwrap_err();
        assert_eq!(err.code, "tuning.note_not_found");
        assert_eq!(m, before);
        assert!(!h.can_undo());
        let mut t = TunerSettings::default();
        let mut c = SetTunerReference::new(500.0);
        assert_eq!(
            c.apply(&mut t).unwrap_err().code,
            "tuning.invalid_reference"
        );
        assert!(c.revert(&mut t).is_err(), "desfazer sem aplicar é erro");
    }

    proptest! {
        /// Qualquer sequência de edições válidas, desfeita por completo, volta ao início.
        #[test]
        fn undo_all_restores_initial(ops in prop::collection::vec((0usize..2, -1200.0f64..1200.0, 0.0f64..1.0, prop::option::of(0u8..128)), 1..20)) {
            let mut m = PitchCorrectionMap::new(AudioSourceId::new());
            let ns = notes();
            m.replace_notes(ns.clone()).unwrap();
            let initial = m.clone();
            let mut h = History::new();
            for (i, cents, flat, target) in ops {
                let edit = NoteEdit { target: target.map(MidiNote), cents_offset: cents, flatten: flat, formant_semitones: 0.0 };
                h.execute(&mut m, Box::new(SetNoteEdit::new(ns[i].id, edit)), None).unwrap();
                prop_assert!(m.validate().is_ok());
            }
            while h.can_undo() {
                h.undo(&mut m).unwrap();
            }
            prop_assert_eq!(m, initial);
        }
    }
}
