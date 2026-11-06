//! DSP de altura do Ebiki Studio (Pure Fabrication, depende apenas do `shared-kernel`).
//!
//! - [`detector`]: porta [`PitchDetector`] (AI-ready, ADR-0011) + YIN.
//! - [`tuner`]: frequência → nota/cents com Lá4 configurável; afinador de tempo real.
//! - [`shifter`]: porta [`PitchShifter`] + PSOLA de latência constante e razão contínua.
//! - [`corrector`]: correção de afinação em tempo real por escala (estilo "auto-tune").
//! - [`analysis`]: trilha de altura, segmentação em notas e render de correções manuais.
//! - [`stretch`]: time-stretch WSOLA (multicanal coerente) e pitch shift offline.
//! - [`signal`]: geradores determinísticos para testes e simulação.
//!
//! Regra de tempo real: tudo que tem `process*` não aloca, não bloqueia e não entra em pânico
//! após `prepare`/`new` (comprovado por `tests/no_alloc.rs`).

pub mod analysis;
pub mod corrector;
pub mod detector;
pub mod shifter;
pub mod signal;
pub mod stretch;
pub mod tuner;

pub use analysis::{
    analyze_notes, analyze_notes_with, pitch_track, render_note_corrections, DetectedNote,
    NoteAnalysisConfig, NoteCorrection, PitchFrame,
};
pub use corrector::{CorrectorSettings, PitchCorrector, ScaleMask};
pub use detector::{cents_between, PitchDetector, PitchEstimate, YinConfig, YinDetector};
pub use shifter::{semitones_to_ratio, PitchShifter, PsolaConfig, PsolaShifter};
pub use stretch::{pitch_shift_offline, resample, time_stretch, TimeStretcher, Wsola};
pub use tuner::{Tuner, TunerProcessor, TunerReading};
