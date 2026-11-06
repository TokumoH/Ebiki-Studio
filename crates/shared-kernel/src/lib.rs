//! Shared Kernel do Ebiki Studio.
//!
//! Contém apenas conceitos estáveis e compartilhados por vários bounded contexts.
//! Nada aqui faz I/O.

pub mod command;
pub mod gain;
pub mod id;
pub mod music;
pub mod tempo;
pub mod time;

pub use gain::{Db, Pan};
pub use tempo::{BarBeat, MeterEvent, TempoError, TempoEvent, TempoMap};
pub use time::{AudioFormat, BitDepth, FormatError, SampleRate, Tick, PPQ};
