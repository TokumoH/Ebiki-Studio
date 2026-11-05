//! Adaptadores (hexagonal) que ligam as portas do `app-core` às bibliotecas técnicas.

pub mod dsp_factory;
pub mod media;

pub use dsp_factory::DspProcessorFactory;
pub use media::CodecsMedia;
