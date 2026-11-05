//! Bounded context **Library**: biblioteca de bases instrumentais (Fase 4).
//!
//! - `domain`: puro (sem I/O). Entidade [`Base`] (CC0 embutida, gerada ou do usuário), agregado
//!   [`BaseCatalog`] com busca/filtros, gerador de bases MIDI ([`generator`]) e a política de
//!   divergência de BPM/tom ao inserir uma base no projeto ([`mismatch`]).
//! - `commands`: operações desfazíveis sobre o [`BaseCatalog`].
//!
//! O gerador **não** depende do contexto Harmony (ADR-0019): recebe acordes já realizados
//! ([`generator::ChordSlot`]); a camada de aplicação compõe Harmony + Library.

pub mod commands;
pub mod domain;

pub use domain::*;
