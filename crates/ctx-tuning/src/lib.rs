//! Bounded context **Tuning** (Fase 8): afinador, correção de afinação em tempo real e em
//! pós-produção, edição manual nota a nota.
//!
//! Domínio puro (sem I/O, sem DSP). O DSP correspondente vive em `dsp-pitch`; a camada de
//! aplicação traduz [`PitchCorrectionMap`] em chamadas de render/correção.

pub mod commands;
pub mod domain;

pub use commands::{ReplaceDetectedNotes, SetNoteEdit, SetScaleCorrection, SetTunerReference};
pub use domain::{
    DetectedNote, DetectedNoteId, NoteEdit, PitchCorrectionMap, PitchCorrectionMapId,
    ScaleCorrection, TunerSettings, TuningError,
};
