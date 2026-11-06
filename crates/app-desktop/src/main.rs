//! Ebiki Studio — ponto de entrada do aplicativo desktop.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod commands_composition;
mod ipc_args;
/// Contexto do app (configuração, ícones, Info.plist). Expandido **uma única vez** no binário:
/// no macOS cada `generate_context!()` embute o símbolo `_EMBED_INFO_PLIST`, e duas expansões
/// (app + testes de IPC) não linkam.
pub fn app_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg(test)]
mod ipc_tests;
mod state;
mod updates;
// Fases 6–8 (partituras, letra, afinação)
mod commands_music;
// Fase 3 (edição / pós-produção)
mod commands_edit;

use audio_engine::device::DeviceRequest;
use state::{lock, AppState};
use std::time::Duration;
use tauri::{Emitter, Manager};

const TELEMETRY_INTERVAL: Duration = Duration::from_millis(33);
const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(120);

fn main() {
    with_commands(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let library =
                infra_persistence::library::LibraryDb::open(&data_dir.join("library.db")).ok();
            let saved_request = library
                .as_ref()
                .and_then(|db| db.setting("audio.request").ok().flatten())
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            let state = AppState::new(library);
            {
                let mut audio = lock(&state.audio);
                let mut studio = lock(&state.studio);
                let req = DeviceRequest {
                    output_id: saved_request
                        .as_ref()
                        .and_then(|v| v["outputId"].as_str().map(str::to_owned)),
                    input_id: saved_request
                        .as_ref()
                        .and_then(|v| v["inputId"].as_str().map(str::to_owned)),
                    buffer_size: saved_request
                        .as_ref()
                        .and_then(|v| v["bufferSize"].as_u64().map(|b| b as u32)),
                    sample_rate: 48_000,
                };
                audio.start(req, &mut studio);
            }
            app.manage(state);
            spawn_telemetry(app.handle().clone());
            spawn_autosave(app.handle().clone());
            Ok(())
        })
        .run(app_context())
        .expect("falha ao iniciar o Ebiki Studio");
}

/// Registra todos os comandos IPC (usado pelo app e pelos testes de IPC).
pub fn with_commands<R: tauri::Runtime>(b: tauri::Builder<R>) -> tauri::Builder<R> {
    b.invoke_handler(tauri::generate_handler![
        updates::app_update_mode,
        commands::audio_devices,
        commands::audio_status,
        commands::audio_configure,
        commands::project_create,
        commands::project_open,
        commands::project_view,
        commands::project_save,
        commands::project_close,
        commands::project_recent,
        commands::project_forget_recent,
        commands::project_recovery_check,
        commands::project_recover,
        commands::project_discard_recovery,
        commands::project_rename,
        commands::history_undo,
        commands::history_redo,
        commands::history_end_gesture,
        commands::track_create,
        commands::track_delete,
        commands::track_rename,
        commands::track_move,
        commands::audio_import,
        commands::clip_move,
        commands::clip_delete,
        commands::clip_set_gain,
        commands::tempo_set,
        commands::tempo_remove,
        commands::meter_set,
        commands::meter_remove,
        commands::mixer_edit,
        commands::bus_create,
        commands::bus_delete,
        commands::bus_rename,
        commands::mixer_presets,
        commands::mixer_apply_preset,
        commands::mix_analyze,
        commands::recording_track,
        commands::recording_metronome,
        commands::recording_punch,
        commands::recording_loop,
        commands::recording_latency,
        commands::take_select,
        commands::transport_play,
        commands::transport_pause,
        commands::transport_stop,
        commands::transport_seek,
        commands::transport_record,
        commands::transport_state,
        commands::waveform_peaks,
        commands_edit::clip_trim,
        commands_edit::clip_split,
        commands_edit::clip_set_fades,
        commands_edit::clip_set_polarity,
        commands_edit::clip_set_reversed,
        commands_edit::clip_crossfade,
        commands_edit::clip_normalize,
        commands_edit::source_spectrogram,
        commands::export_formats,
        commands::export_mix,
        commands::export_stems,
        commands_music::score_list,
        commands_music::score_create,
        commands_music::score_create_from_chords,
        commands_music::score_get,
        commands_music::score_delete,
        commands_music::score_edit_voice,
        commands_music::score_edit,
        commands_music::score_import,
        commands_music::score_import_bytes,
        commands_music::score_musicxml,
        commands_music::score_export,
        commands_music::score_audition,
        commands_music::score_render_to_track,
        commands_music::score_validate,
        commands_music::lyrics_set_text,
        commands_music::lyrics_set_span,
        commands_music::lyrics_clear_span,
        commands_music::lyrics_shift,
        commands_music::lyrics_apply_tap_sync,
        commands_music::lyrics_export,
        commands_music::lyrics_import,
        commands_music::tuning_analyze,
        commands_music::tuning_get,
        commands_music::tuning_edit_note,
        commands_music::tuning_set_scale,
        commands_music::tuning_set_reference,
        commands_music::tuning_apply,
        commands_music::tuning_clear,
        commands_music::tuner_start,
        commands_music::tuner_read,
        commands_music::tuner_stop,
        commands_music::clip_time_stretch,
        commands_composition::harmony_keys,
        commands_composition::harmony_field,
        commands_composition::harmony_progressions,
        commands_composition::harmony_user_progressions,
        commands_composition::harmony_suggest_next,
        commands_composition::harmony_realize,
        commands_composition::harmony_audition,
        commands_composition::harmony_insert_progression,
        commands_composition::harmony_user_progression_add,
        commands_composition::harmony_user_progression_edit,
        commands_composition::harmony_user_progression_remove,
        commands_composition::library_history,
        commands_composition::library_undo,
        commands_composition::library_redo,
        commands_composition::bases_options,
        commands_composition::bases_search,
        commands_composition::bases_add_user,
        commands_composition::bases_generate,
        commands_composition::bases_generated_audition,
        commands_composition::bases_save_generated,
        commands_composition::bases_remove,
        commands_composition::bases_audition,
        commands_composition::bases_mismatch,
        commands_composition::bases_insert,
        commands_composition::synth_status,
        commands_composition::synth_set_soundfont,
    ])
}

/// Publica posição e medidores (~30 Hz) enquanto houver projeto aberto.
fn spawn_telemetry(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("ebiki-telemetry".into())
        .spawn(move || loop {
            std::thread::sleep(TELEMETRY_INTERVAL);
            let state = app.state::<AppState>();
            let (transport, meters) = {
                let studio = lock(&state.studio);
                if !studio.is_open() {
                    continue;
                }
                (studio.transport().ok(), studio.meters())
            };
            if let Some(t) = transport {
                let _ = app.emit("transport", t);
            }
            let _ = app.emit("meters", meters);
        })
        .expect("thread de telemetria");
}

fn spawn_autosave(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("ebiki-autosave".into())
        .spawn(move || loop {
            std::thread::sleep(AUTOSAVE_INTERVAL);
            let state = app.state::<AppState>();
            let r = lock(&state.studio).autosave();
            if let Err(e) = r {
                let _ = app.emit("app-error", e);
            }
        })
        .expect("thread de autosave");
}
