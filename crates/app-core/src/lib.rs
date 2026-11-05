//! Camada de aplicação do AppMusica.
//!
//! - [`session::Session`]: conjunto de agregados de um projeto aberto (alvo dos comandos).
//! - [`ports`]: interfaces que a infraestrutura implementa (persistência, mídia, engine).

pub mod compile;
pub mod ports;
pub mod render;
pub mod session;
pub mod studio;
pub mod view;

pub use session::*;
pub use studio::*;
pub use view::*;

// Fase 4/5 — composição (harmonia + bases); módulo próprio para merge trivial.
pub mod composition;
