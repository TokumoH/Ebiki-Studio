//! Modelo de domínio do contexto Harmony.

pub mod catalog;
pub mod chord;
pub mod error;
pub mod field;
pub mod progression;
pub mod realize;
pub mod roman;
pub mod scale;
pub mod suggest;
pub mod workspace;

pub use catalog::{builtin_catalog, CATALOG_VERSION};
pub use chord::{Chord, ChordQuality, Tension};
pub use error::HarmonyError;
pub use field::{diatonic_quality, harmonic_field, FieldDegree};
pub use progression::{
    Genre, Notation, Progression, ProgressionId, ProgressionKey, ProgressionStep,
};
pub use realize::{realize, RealizedChord, TimedChord, VoicingOptions};
pub use roman::{Degree, QualitySpec, RomanChord};
pub use scale::{Key, Mode, ScaleFamily};
pub use suggest::{
    ChordSuggestion, ProgressionQuery, ProgressionSuggestion, RuleBasedSuggester,
    SuggestionProvider, SuggestionReason,
};
pub use workspace::HarmonyWorkspace;
