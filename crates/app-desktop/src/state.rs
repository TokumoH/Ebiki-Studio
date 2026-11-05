//! Estado global do app (composition root).

use crate::audio::AudioRuntime;
use app_core::ports::SystemClock;
use app_core::{Studio, StudioDeps};
use infra_adapters::{CodecsMedia, DspProcessorFactory};
use infra_persistence::library::LibraryDb;
use infra_persistence::SqliteSessionRepository;
use std::sync::{Arc, Mutex, MutexGuard};
// Fase 4/5 (composição)
use app_core::composition::Composer;
use infra_adapters::{ConfigurableSynthRenderer, SOUNDFONT_SETTING};

pub struct AppState {
    pub studio: Mutex<Studio>,
    pub audio: Mutex<AudioRuntime>,
    pub library: Mutex<Option<LibraryDb>>,
    /// Sintetizador (SoundFont opcional) compartilhado com o `Studio`.
    pub synth: Arc<ConfigurableSynthRenderer>,
    /// Biblioteca de composição (harmonia + bases); aberta na primeira chamada.
    pub composer: Mutex<Option<Composer>>,
}

impl AppState {
    pub fn new(library: Option<LibraryDb>) -> Self {
        let synth = Arc::new(ConfigurableSynthRenderer::new());
        if let Some(path) = library
            .as_ref()
            .and_then(|db| db.setting(SOUNDFONT_SETTING).ok().flatten())
            .filter(|p| !p.is_empty())
        {
            // arquivo sumiu/corrompeu: segue com o sintetizador simples
            let _ = synth.set_soundfont(std::path::Path::new(&path));
        }
        let deps = StudioDeps {
            repo: Arc::new(SqliteSessionRepository),
            media: Arc::new(CodecsMedia),
            factory: Arc::new(DspProcessorFactory),
            clock: Arc::new(SystemClock),
            synth: synth.clone(),
        };
        Self {
            studio: Mutex::new(Studio::new(deps)),
            audio: Mutex::new(AudioRuntime::default()),
            library: Mutex::new(library),
            synth,
            composer: Mutex::new(None),
        }
    }
}

/// Trava ignorando envenenamento (um pânico anterior não deve travar o app inteiro).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
