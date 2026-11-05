//! Modelo de domínio do contexto Library.

pub mod base;
pub mod catalog;
pub mod error;
pub mod generator;
pub mod mismatch;

pub use base::{
    AudioBase, AudioPath, Base, BaseId, BaseKey, BaseKind, BaseKindTag, GenerationRecipe,
    GenreCode, Instrument, KeyMode, Meter, Tag,
};
pub use catalog::{BaseCatalog, BaseQuery};
pub use error::LibraryError;
pub use generator::{
    generate, BassPattern, ChordSlot, DrumPattern, GeneratedBase, GeneratedTrack,
    GenerationRequest, NoteEvent, StylePattern, TrackRole,
};
pub use mismatch::{mismatch, MismatchReport};
