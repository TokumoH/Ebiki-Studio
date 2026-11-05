//! Casos de uso de **composição** (Fase 5 Harmonia + Fase 4 Bases) na camada de aplicação.
//!
//! Compõe os contextos `ctx-harmony` e `ctx-library` (que não dependem um do outro — ADR-0019)
//! com o `Studio` (projeto aberto) e as portas de síntese/mídia:
//!
//! - [`Composer`]: serviço da biblioteca do usuário (progressões do usuário + catálogo de bases),
//!   com histórico próprio de desfazer/refazer e persistência pela porta [`LibraryStore`]; consulta
//!   as sugestões pela porta `SuggestionProvider` (pronta para IA — ADR-0011).
//! - Operações no `Studio` (`impl Studio` em `studio_ops`): audição em WAV na memória, inserção de
//!   progressão/base na timeline como **um único** comando desfazível, e a política de divergência
//!   de BPM/tom com decisão explícita (`Keep` | `Adapt`) — ADR-0026.

mod bases;
mod harmony;
mod library;
mod studio_ops;
mod wav;

pub use bases::*;
pub use harmony::*;
pub use library::*;
pub use studio_ops::*;
pub use wav::*;

/// Reexporta os contextos para a camada de entrada (IPC) sem dependências extras.
pub use ctx_harmony;
pub use ctx_library;

use crate::studio::AppError;

pub(crate) fn harmony_err(e: ctx_harmony::HarmonyError) -> AppError {
    AppError::new(e.code(), e.to_string())
}

pub(crate) fn library_err(e: ctx_library::LibraryError) -> AppError {
    AppError::new(e.code(), e.to_string())
}

pub(crate) fn invalid(code: &str, msg: impl Into<String>) -> AppError {
    AppError::new(format!("composition.{code}"), msg)
}
