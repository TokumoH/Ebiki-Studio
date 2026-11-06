//! Adaptadores (hexagonal) que ligam as portas do `app-core` às bibliotecas técnicas.

pub mod dsp_factory;
pub mod media;
pub mod notation;
pub mod synth;

pub use dsp_factory::DspProcessorFactory;
pub use media::CodecsMedia;
pub use notation::NotationIoFormats;
pub use synth::{ConfigurableSynthRenderer, SynthRenderer, SynthStatus, SOUNDFONT_SETTING};
