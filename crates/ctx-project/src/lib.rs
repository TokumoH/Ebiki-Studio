//! Bounded context **Project**: o arranjo musical (faixas, clipes, fontes de áudio, mapa de tempo).
//!
//! `domain` é puro (sem I/O). `commands` contém as operações desfazíveis sobre o agregado.

pub mod commands;
pub mod domain;

pub use domain::*;
