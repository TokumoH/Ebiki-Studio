//! Engine de áudio do AppMusica (Pure Fabrication).
//!
//! Arquitetura (ADR-0005):
//! - A thread de controle **compila** um [`graph::RenderGraph`] a partir do domínio e o envia por fila
//!   lock-free para o [`core::EngineCore`], que roda na thread de áudio.
//! - O núcleo **nunca aloca, bloqueia ou libera memória**: objetos substituídos voltam para a thread de
//!   controle por uma fila de "lixo" e são liberados lá ([`handle::EngineHandle::collect_garbage`]).
//! - Estado de efeitos e medidores é **herdado** entre grafos por chave estável (id da faixa/efeito),
//!   então editar a mixagem não corta caudas de reverb nem reinicia compressores.
//! - Telemetria (posição, picos) por atômicos ([`telemetry::Telemetry`]).
//! - O mesmo código processa em tempo real (backend) e offline ([`offline`]), o que permite simulação
//!   determinística nos testes.

pub mod core;
pub mod graph;
pub mod handle;
pub mod metronome;
pub mod offline;
pub mod recorder;
pub mod telemetry;
pub mod transport;

#[cfg(feature = "device")]
pub mod device;

pub use crate::core::EngineCore;
pub use graph::*;
pub use handle::{create_engine, EngineConfig, EngineHandle};
pub use telemetry::{Telemetry, TelemetrySnapshot};
pub use transport::TransportState;

/// Maior bloco processado de uma vez (callbacks maiores são divididos).
pub const MAX_BLOCK: usize = 1024;
/// Máximo de canais (faixas + buses) com medidor.
pub const MAX_STRIPS: usize = 512;
