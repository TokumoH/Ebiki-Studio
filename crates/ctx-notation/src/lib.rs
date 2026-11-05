//! Bounded context **Notation** (Fase 7): partitura e tablatura.
//!
//! - `frac`, `duration`, `pitch`: value objects (frações exatas, figuras, alturas grafadas, armaduras).
//! - `model`: agregado [`Score`] → [`Part`] → [`Measure`] → [`Voice`] → [`Event`] → [`Note`].
//! - `commands`: edições desfazíveis (`Command<Score>`).
//! - `tab`: mapeamento tablatura ⇄ altura.
//! - `transpose`: transposição com grafia enarmônica pela armadura de destino.
//! - `analysis`: validação de compasso, preenchimento de pausas, notas fora da escala.
//! - `arrange`: notas temporizadas → compassos/vozes (usado por "acordes → partitura" e importação MIDI).
//! - `playback`: partitura → eventos MIDI temporizados (ligaduras, quiálteras, dinâmicas, repetições).
//!
//! Domínio puro: sem I/O. Formatos de arquivo ficam em `infra-notation-io`.

pub mod analysis;
pub mod arrange;
pub mod commands;
pub mod duration;
pub mod error;
pub mod frac;
pub mod model;
pub mod pitch;
pub mod playback;
pub mod tab;
pub mod transpose;

pub use duration::{Duration, NoteValue, Tuplet};
pub use error::NotationError;
pub use frac::Frac;
pub use model::*;
pub use pitch::{KeySignature, Mode, Pitch, Step};
