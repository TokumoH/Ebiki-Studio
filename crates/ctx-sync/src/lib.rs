//! Bounded context **Sync** (Fase 6): letra sincronizada por palavra/sílaba, exportação
//! LRC/SRT, sessão de sincronização por toque e mapa de warp (grade de BPM / time-stretch).
//!
//! Domínio puro: sem I/O. Conversões de tempo usam o `TempoMap` do `shared-kernel`.

pub mod commands;
pub mod error;
pub mod formats;
pub mod hyphenation;
pub mod lyrics;
pub mod tap;
pub mod warp;

pub use error::SyncError;
pub use formats::{export_lrc, export_srt, import_lrc, import_srt};
pub use hyphenation::split_syllables_pt;
pub use lyrics::{
    Granularity, LyricLine, LyricTrack, LyricTrackId, LyricUnit, LyricWord, Span, Syllable,
    SyllableRef, WordRef,
};
pub use tap::{TapState, TapSyncSession};
pub use warp::{stretch_ratio_for_tempo, WarpMap, WarpMapId, WarpMarker};
