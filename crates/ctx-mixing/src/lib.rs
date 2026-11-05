//! Bounded context **Mixing**.
//!
//! - `domain`: mixer, canais, buses, sends, efeitos (parâmetros validados).
//! - `commands`: operações desfazíveis.
//! - `presets`: cadeias de efeitos por instrumento (voz, violão, guitarra…).
//! - `advisor`: auxílio de mixagem por regras (porta `MixAdvisor`, preparada para IA).

pub mod advisor;
pub mod commands;
pub mod domain;
pub mod effects;
pub mod presets;

pub use domain::*;
pub use effects::*;
