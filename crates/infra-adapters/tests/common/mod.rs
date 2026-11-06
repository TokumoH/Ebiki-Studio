//! Montagem compartilhada pelos testes de integração: o `Studio` com as implementações reais
//! (as mesmas que o app desktop usa no composition root).
use app_core::ports::SystemClock;
use app_core::{Studio, StudioDeps};
use infra_adapters::{CodecsMedia, DspProcessorFactory};
use std::sync::Arc;

pub fn studio() -> Studio {
    Studio::new(StudioDeps {
        repo: Arc::new(infra_persistence::SqliteSessionRepository),
        media: Arc::new(CodecsMedia),
        factory: Arc::new(DspProcessorFactory),
        clock: Arc::new(SystemClock),
        synth: Arc::new(infra_adapters::SynthRenderer),
        notation: Arc::new(infra_adapters::NotationIoFormats),
    })
}
