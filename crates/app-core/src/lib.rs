//! Camada de aplicação do AppMusica.
//!
//! - [`session::Session`]: conjunto de agregados de um projeto aberto (alvo dos comandos).
//! - [`ports`]: interfaces que a infraestrutura implementa (persistência, mídia, engine).

pub mod ports;
pub mod session;

pub use session::*;
