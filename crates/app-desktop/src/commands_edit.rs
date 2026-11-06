//! Comandos IPC da Fase 3 — Edição / Pós-produção. Finos: validam argumentos e delegam aos
//! casos de uso de `app_core::edit_service`.

use crate::ipc_args::{bad, id, Res};
use crate::state::{lock, AppState};
use app_core::edit_service::{FadeRequest, DEFAULT_NORMALIZE_TARGET_DBFS};
use app_core::ports::SpectrogramRequest;
use app_core::AppError;
use app_core::SessionView;
use ctx_project::{ClipId, FadeShape};
use shared_kernel::id::AudioSourceId;
use shared_kernel::Tick;
use tauri::State;

#[tauri::command]
pub fn clip_trim(
    state: State<AppState>,
    clip_id: String,
    start: i64,
    length_frames: u64,
    gesture: Option<u64>,
) -> Res<SessionView> {
    if start < 0 {
        return Err(bad("início negativo"));
    }
    lock(&state.studio).clip_trim(id::<ClipId>(&clip_id)?, Tick(start), length_frames, gesture)
}

#[tauri::command]
pub fn clip_split(state: State<AppState>, clip_id: String, at_tick: i64) -> Res<SessionView> {
    lock(&state.studio).clip_split(id(&clip_id)?, Tick(at_tick))
}

#[tauri::command]
pub fn clip_set_fades(
    state: State<AppState>,
    clip_id: String,
    fades: FadeRequest,
    gesture: Option<u64>,
) -> Res<SessionView> {
    lock(&state.studio).clip_set_fades(id(&clip_id)?, fades, gesture)
}

#[tauri::command]
pub fn clip_set_polarity(
    state: State<AppState>,
    clip_id: String,
    inverted: bool,
) -> Res<SessionView> {
    lock(&state.studio).clip_set_polarity(id(&clip_id)?, inverted)
}

#[tauri::command]
pub fn clip_set_reversed(
    state: State<AppState>,
    clip_id: String,
    reversed: bool,
) -> Res<SessionView> {
    lock(&state.studio).clip_set_reversed(id(&clip_id)?, reversed)
}

#[tauri::command]
pub fn clip_crossfade(
    state: State<AppState>,
    a_id: String,
    b_id: String,
    shape: FadeShape,
) -> Res<SessionView> {
    lock(&state.studio).clips_crossfade(id(&a_id)?, id(&b_id)?, shape)
}

/// Normaliza o pico do clipe para `target_dbfs` (padrão −1 dBFS) via ganho do clipe.
#[tauri::command]
pub fn clip_normalize(
    state: State<AppState>,
    clip_id: String,
    target_dbfs: Option<f32>,
) -> Res<SessionView> {
    lock(&state.studio).clip_normalize(
        id(&clip_id)?,
        target_dbfs.unwrap_or(DEFAULT_NORMALIZE_TARGET_DBFS),
    )
}

/// Espectrograma de uma fonte em binário (formato `AMSG`, ver `infra_codecs::spectrogram`).
/// Assíncrono e calculado fora do lock do `Studio`: não congela a UI nem os demais comandos.
#[tauri::command]
pub async fn source_spectrogram(
    state: State<'_, AppState>,
    source_id: String,
    params: Option<SpectrogramRequest>,
) -> Res<tauri::ipc::Response> {
    let src: AudioSourceId = id(&source_id)?;
    let job = lock(&state.studio).spectrogram_job(src, params.unwrap_or_default())?;
    let bytes = tauri::async_runtime::spawn_blocking(move || job.run())
        .await
        .map_err(|e| AppError::new("app.internal", e.to_string()))??;
    Ok(tauri::ipc::Response::new(bytes))
}
