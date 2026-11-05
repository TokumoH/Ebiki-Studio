//! Estado global do app (composition root).

use crate::audio::AudioRuntime;
use app_core::ports::SystemClock;
use app_core::{Studio, StudioDeps};
use infra_adapters::{CodecsMedia, DspProcessorFactory};
use infra_persistence::library::LibraryDb;
use infra_persistence::SqliteSessionRepository;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct AppState {
    pub studio: Mutex<Studio>,
    pub audio: Mutex<AudioRuntime>,
    pub library: Mutex<Option<LibraryDb>>,
}

impl AppState {
    pub fn new(library: Option<LibraryDb>) -> Self {
        let deps = StudioDeps {
            repo: Arc::new(SqliteSessionRepository),
            media: Arc::new(CodecsMedia),
            factory: Arc::new(DspProcessorFactory),
            clock: Arc::new(SystemClock),
        };
        Self { studio: Mutex::new(Studio::new(deps)), audio: Mutex::new(AudioRuntime::default()), library: Mutex::new(library) }
    }
}

/// Trava ignorando envenenamento (um pânico anterior não deve travar o app inteiro).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
