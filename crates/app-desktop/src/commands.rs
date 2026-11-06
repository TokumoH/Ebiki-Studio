//! Comandos IPC (adaptador de entrada). Finos: validam/convertem DTOs e delegam ao `Studio`.

use crate::audio::{AudioRuntime, AudioStatus};
use crate::ipc_args::{bad, id, user_path, Res};
use crate::state::{lock, AppState};
use app_core::ports::{ExportFormatKind, ExportSettings};
use app_core::*;
use audio_engine::device::{DeviceList, DeviceRequest};
use ctx_mixing::advisor::{Advice, MixAnalysis};
use ctx_mixing::commands::{CreateBus, DeleteBus, EditStrip, RenameBus};
use ctx_mixing::presets::InstrumentPreset;
use ctx_mixing::{Bus, BusId, ChannelStrip, StripEdit, StripTarget};
use ctx_project::commands::*;
use ctx_project::ClipId;
use ctx_recording::commands::*;
use ctx_recording::{InputAssignment, MetronomeConfig, MonitorMode, TakeId};
use infra_persistence::library::RecentProject;
use serde::{Deserialize, Serialize};
use shared_kernel::command::{Command, Composite};
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::{AudioFormat, BitDepth, Db, SampleRate, Tick};
use std::path::Path;
use tauri::State;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn touch_recent(state: &AppState, v: &SessionView) {
    if let Some(db) = lock(&state.library).as_ref() {
        let _ = db.touch_recent(&v.project.root, &v.project.name, now());
    }
}

// ------------------------------------------------------------------ áudio

#[tauri::command]
pub fn audio_devices() -> DeviceList {
    AudioRuntime::devices()
}

#[tauri::command]
pub fn audio_status(state: State<AppState>) -> AudioStatus {
    lock(&state.audio).status.clone()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfigReq {
    pub output_id: Option<String>,
    pub input_id: Option<String>,
    pub buffer_size: Option<u32>,
}

#[tauri::command]
pub fn audio_configure(state: State<AppState>, req: AudioConfigReq) -> Res<AudioStatus> {
    if let Some(b) = req.buffer_size {
        if !(16..=8192).contains(&b) || !b.is_power_of_two() {
            return Err(bad(
                "tamanho de buffer deve ser potência de 2 entre 16 e 8192",
            ));
        }
    }
    let mut audio = lock(&state.audio);
    let mut studio = lock(&state.studio);
    let sr = studio
        .session()
        .map_or(48_000, |s| s.project.format.sample_rate.hz());
    let status = audio.start(
        DeviceRequest {
            output_id: req.output_id,
            input_id: req.input_id,
            buffer_size: req.buffer_size,
            sample_rate: sr,
        },
        &mut studio,
    );
    if let Some(db) = lock(&state.library).as_ref() {
        let _ = db.set_setting("audio.request", &serde_json::json!({
            "outputId": audio.request.output_id, "inputId": audio.request.input_id, "bufferSize": audio.request.buffer_size
        }).to_string());
    }
    Ok(status)
}

// ---------------------------------------------------------------- projeto

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectReq {
    pub dir: String,
    pub name: String,
    pub sample_rate: u32,
    pub bit_depth: u16,
}

#[tauri::command]
pub fn project_create(state: State<AppState>, req: CreateProjectReq) -> Res<SessionView> {
    let format = AudioFormat {
        sample_rate: SampleRate::new(req.sample_rate).map_err(|e| bad(&e.to_string()))?,
        bit_depth: BitDepth::new(req.bit_depth).map_err(|e| bad(&e.to_string()))?,
    };
    let dir = user_path(&req.dir)?;
    let mut audio = lock(&state.audio);
    let mut studio = lock(&state.studio);
    audio.ensure_rate(format.sample_rate.hz(), &mut studio);
    let v = studio.create_project(&dir, &req.name, format)?;
    drop(studio);
    touch_recent(&state, &v);
    Ok(v)
}

#[tauri::command]
pub fn project_open(state: State<AppState>, path: String) -> Res<SessionView> {
    let path = user_path(&path)?;
    let mut audio = lock(&state.audio);
    let mut studio = lock(&state.studio);
    let v = studio.open_project(&path)?;
    audio.ensure_rate(v.project.sample_rate, &mut studio);
    let v = studio.view()?;
    drop(studio);
    touch_recent(&state, &v);
    Ok(v)
}

#[tauri::command]
pub fn project_view(state: State<AppState>) -> Res<SessionView> {
    lock(&state.studio).view()
}

#[tauri::command]
pub fn project_save(state: State<AppState>) -> Res<SessionView> {
    lock(&state.studio).save()
}

#[tauri::command]
pub fn project_close(state: State<AppState>) -> Res<()> {
    lock(&state.studio).close()
}

#[tauri::command]
pub fn project_recent(state: State<AppState>) -> Vec<RecentProject> {
    lock(&state.library)
        .as_ref()
        .and_then(|db| db.recent().ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn project_forget_recent(state: State<AppState>, path: String) -> Res<()> {
    if let Some(db) = lock(&state.library).as_ref() {
        db.remove_recent(&path)
            .map_err(|e| AppError::new(e.code(), e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn project_recovery_check(path: String) -> Res<bool> {
    let root = user_path(&path)?;
    let root = if root.ends_with(DB_FILE) {
        root.parent().map(Path::to_path_buf).unwrap_or(root)
    } else {
        root
    };
    Ok(Studio::recovery_candidate(&root).is_some())
}

#[tauri::command]
pub fn project_recover(state: State<AppState>, path: String) -> Res<SessionView> {
    let root = user_path(&path)?;
    let mut audio = lock(&state.audio);
    let mut studio = lock(&state.studio);
    let v = studio.recover(&root)?;
    audio.ensure_rate(v.project.sample_rate, &mut studio);
    studio.view()
}

#[tauri::command]
pub fn project_discard_recovery(path: String) -> Res<()> {
    Studio::discard_recovery(&user_path(&path)?)
}

#[tauri::command]
pub fn project_rename(state: State<AppState>, name: String) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(RenameProject::new(name)), None)
}

// -------------------------------------------------------------- histórico

#[tauri::command]
pub fn history_undo(state: State<AppState>) -> Res<SessionView> {
    lock(&state.studio).undo()
}

#[tauri::command]
pub fn history_redo(state: State<AppState>) -> Res<SessionView> {
    lock(&state.studio).redo()
}

#[tauri::command]
pub fn history_end_gesture(state: State<AppState>) {
    lock(&state.studio).end_gesture()
}

// ----------------------------------------------------------- faixas/clipes

#[tauri::command]
pub fn track_create(
    state: State<AppState>,
    name: String,
    index: Option<usize>,
) -> Res<SessionView> {
    lock(&state.studio)
        .create_track(&name, index)
        .map(|(_, v)| v)
}

#[tauri::command]
pub fn track_delete(state: State<AppState>, track_id: String) -> Res<SessionView> {
    lock(&state.studio).delete_track(id(&track_id)?)
}

#[tauri::command]
pub fn track_rename(state: State<AppState>, track_id: String, name: String) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(RenameTrack::new(id(&track_id)?, name)), None)
}

#[tauri::command]
pub fn track_move(state: State<AppState>, track_id: String, to: usize) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(MoveTrack::new(id(&track_id)?, to)), None)
}

#[tauri::command]
pub fn audio_import(
    state: State<AppState>,
    paths: Vec<String>,
    track_id: Option<String>,
    at_tick: i64,
) -> Res<SessionView> {
    if paths.len() > 256 {
        return Err(bad("no máximo 256 arquivos por vez"));
    }
    let paths = paths
        .iter()
        .map(|p| user_path(p))
        .collect::<Res<Vec<_>>>()?;
    let track = track_id.map(|t| id::<TrackId>(&t)).transpose()?;
    lock(&state.studio).import_audio(&paths, track, Tick(at_tick.max(0)))
}

#[tauri::command]
pub fn clip_move(
    state: State<AppState>,
    clip_id: String,
    track_id: String,
    start: i64,
    gesture: Option<u64>,
) -> Res<SessionView> {
    let cmd = on_project(MoveClip::new(
        id::<ClipId>(&clip_id)?,
        id(&track_id)?,
        Tick(start.max(0)),
    ));
    lock(&state.studio).execute(cmd, gesture)
}

#[tauri::command]
pub fn clip_delete(state: State<AppState>, clip_id: String) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(DeleteClip::new(id(&clip_id)?)), None)
}

#[tauri::command]
pub fn clip_set_gain(
    state: State<AppState>,
    clip_id: String,
    gain_db: f32,
    gesture: Option<u64>,
) -> Res<SessionView> {
    lock(&state.studio).execute(
        on_project(SetClipGain::new(id(&clip_id)?, Db(gain_db))),
        gesture,
    )
}

// ------------------------------------------------------------------ tempo

#[tauri::command]
pub fn tempo_set(state: State<AppState>, tick: i64, bpm: f64) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(SetTempo::new(Tick(tick), bpm)), None)
}

#[tauri::command]
pub fn tempo_remove(state: State<AppState>, tick: i64) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(RemoveTempo::new(Tick(tick))), None)
}

#[tauri::command]
pub fn meter_set(
    state: State<AppState>,
    tick: i64,
    numerator: u8,
    denominator: u8,
) -> Res<SessionView> {
    lock(&state.studio).execute(
        on_project(SetMeter::new(Tick(tick), numerator, denominator)),
        None,
    )
}

#[tauri::command]
pub fn meter_remove(state: State<AppState>, tick: i64) -> Res<SessionView> {
    lock(&state.studio).execute(on_project(RemoveMeter::new(Tick(tick))), None)
}

// ------------------------------------------------------------------ mixer

#[tauri::command]
pub fn mixer_edit(
    state: State<AppState>,
    target: StripTarget,
    edit: StripEdit,
    gesture: Option<u64>,
) -> Res<SessionView> {
    lock(&state.studio).execute(on_mixer(EditStrip::new(target, edit)), gesture)
}

#[tauri::command]
pub fn bus_create(state: State<AppState>, name: String) -> Res<SessionView> {
    let bus = Bus {
        id: BusId::new(),
        name,
        strip: ChannelStrip::default(),
    };
    lock(&state.studio).execute(on_mixer(CreateBus::new(bus, usize::MAX)), None)
}

#[tauri::command]
pub fn bus_delete(state: State<AppState>, bus_id: String) -> Res<SessionView> {
    lock(&state.studio).execute(on_mixer(DeleteBus::new(id(&bus_id)?)), None)
}

#[tauri::command]
pub fn bus_rename(state: State<AppState>, bus_id: String, name: String) -> Res<SessionView> {
    lock(&state.studio).execute(on_mixer(RenameBus::new(id(&bus_id)?, name)), None)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInfo {
    pub id: InstrumentPreset,
    pub label: &'static str,
}

#[tauri::command]
pub fn mixer_presets() -> Vec<PresetInfo> {
    InstrumentPreset::ALL
        .iter()
        .map(|p| PresetInfo {
            id: *p,
            label: p.label(),
        })
        .collect()
}

/// Aplica um preset: substitui os efeitos do canal pela cadeia do preset (um único desfazer).
#[tauri::command]
pub fn mixer_apply_preset(
    state: State<AppState>,
    target: StripTarget,
    preset: InstrumentPreset,
) -> Res<SessionView> {
    let mut studio = lock(&state.studio);
    let current: Vec<_> = studio
        .session()
        .ok_or_else(|| AppError::new("app.no_project", "Nenhum projeto aberto."))?
        .mixer
        .strip(target)
        .map_err(|e| AppError::new(e.code(), e.to_string()))?
        .inserts
        .iter()
        .map(|s| s.id)
        .collect();
    let mut steps: Vec<Box<dyn Command<Session>>> = current
        .into_iter()
        .map(|eid| on_mixer(EditStrip::new(target, StripEdit::RemoveEffect { id: eid })))
        .collect();
    for slot in preset.chain() {
        steps.push(on_mixer(EditStrip::new(
            target,
            StripEdit::AddEffect { slot, index: None },
        )));
    }
    studio.execute(
        Box::new(Composite::new(format!("Preset: {}", preset.label()), steps)),
        None,
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixReport {
    pub analysis: MixAnalysis,
    pub advice: Vec<Advice>,
}

#[tauri::command]
pub async fn mix_analyze(state: State<'_, AppState>, target_lufs: Option<f32>) -> Res<MixReport> {
    let (analysis, advice) = lock(&state.studio).analyze_mix(target_lufs.unwrap_or(-14.0))?;
    Ok(MixReport { analysis, advice })
}

// --------------------------------------------------------------- gravação

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRecordingReq {
    pub track_id: String,
    pub armed: Option<bool>,
    /// `Some(None)` remove a entrada.
    #[serde(default, with = "double_option")]
    pub input: Option<Option<InputAssignment>>,
    pub monitor: Option<MonitorMode>,
}

mod double_option {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Option::<T>::deserialize(d).map(Some)
    }
}

#[tauri::command]
pub fn recording_track(state: State<AppState>, req: TrackRecordingReq) -> Res<SessionView> {
    let track: TrackId = id(&req.track_id)?;
    let mut studio = lock(&state.studio);
    let mut tr = studio
        .session()
        .ok_or_else(|| AppError::new("app.no_project", "Nenhum projeto aberto."))?
        .recording
        .track(track);
    if let Some(a) = req.armed {
        tr.armed = a;
    }
    if let Some(i) = req.input {
        tr.input = i;
    }
    if let Some(m) = req.monitor {
        tr.monitor = m;
    }
    studio.execute(
        on_recording(SetTrackRecording::new(track, tr, "Configurar gravação")),
        None,
    )
}

#[tauri::command]
pub fn recording_metronome(state: State<AppState>, config: MetronomeConfig) -> Res<SessionView> {
    lock(&state.studio).execute(on_recording(SetMetronome::new(config)), None)
}

#[tauri::command]
pub fn recording_punch(state: State<AppState>, range: Option<(i64, i64)>) -> Res<SessionView> {
    lock(&state.studio).execute(
        on_recording(SetPunch::new(range.map(|(a, b)| (Tick(a), Tick(b))))),
        None,
    )
}

#[tauri::command]
pub fn recording_loop(state: State<AppState>, range: Option<(i64, i64)>) -> Res<SessionView> {
    lock(&state.studio).execute(
        on_recording(SetLoop::new(range.map(|(a, b)| (Tick(a), Tick(b))))),
        None,
    )
}

#[tauri::command]
pub fn recording_latency(state: State<AppState>, frames: u32) -> Res<SessionView> {
    lock(&state.studio).execute(on_recording(SetLatencyCompensation::new(frames)), None)
}

#[tauri::command]
pub fn take_select(state: State<AppState>, track_id: String, take_id: String) -> Res<SessionView> {
    lock(&state.studio).set_active_take(id(&track_id)?, id::<TakeId>(&take_id)?)
}

// ------------------------------------------------------------- transporte

#[tauri::command]
pub fn transport_play(state: State<AppState>) -> Res<()> {
    lock(&state.studio).play()
}

#[tauri::command]
pub fn transport_pause(state: State<AppState>) -> Res<()> {
    lock(&state.studio).pause()
}

#[tauri::command]
pub fn transport_stop(state: State<AppState>, return_to_start: bool) -> Res<Option<SessionView>> {
    lock(&state.studio).stop(return_to_start)
}

#[tauri::command]
pub fn transport_seek(state: State<AppState>, tick: i64) -> Res<()> {
    lock(&state.studio).seek(Tick(tick))
}

#[tauri::command]
pub fn transport_record(state: State<AppState>, at_tick: i64, count_in: bool) -> Res<()> {
    lock(&state.studio).start_recording(Tick(at_tick.max(0)), count_in)
}

#[tauri::command]
pub fn transport_state(state: State<AppState>) -> Res<TransportView> {
    lock(&state.studio).transport()
}

/// Picos da forma de onda em binário (f32 little-endian: mín, máx, mín, máx…).
#[tauri::command]
pub fn waveform_peaks(
    state: State<AppState>,
    source_id: String,
    frames_per_peak: u32,
) -> Res<tauri::ipc::Response> {
    let src: AudioSourceId = id(&source_id)?;
    let peaks = lock(&state.studio).waveform(src, frames_per_peak)?;
    let bytes: Vec<u8> = peaks.iter().flat_map(|x| x.to_le_bytes()).collect();
    Ok(tauri::ipc::Response::new(bytes))
}

// ------------------------------------------------------------- exportação

#[tauri::command]
pub fn export_formats(state: State<AppState>) -> Vec<ExportFormatKind> {
    lock(&state.studio).available_formats()
}

#[tauri::command]
pub async fn export_mix(
    state: State<'_, AppState>,
    path: String,
    settings: ExportSettings,
    range: Option<(i64, i64)>,
) -> Res<()> {
    let file = user_path(&path)?;
    lock(&state.studio).export_mix(&file, &settings, range.map(|(a, b)| (Tick(a), Tick(b))))
}

#[tauri::command]
pub async fn export_stems(
    state: State<'_, AppState>,
    dir: String,
    settings: ExportSettings,
) -> Res<Vec<String>> {
    let d = user_path(&dir)?;
    Ok(lock(&state.studio)
        .export_stems(&d, &settings)?
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_option_distinguishes_missing_from_null() {
        let a: TrackRecordingReq = serde_json::from_str(r#"{"trackId":"x"}"#).unwrap();
        assert!(a.input.is_none());
        let b: TrackRecordingReq = serde_json::from_str(r#"{"trackId":"x","input":null}"#).unwrap();
        assert_eq!(b.input, Some(None));
        let c: TrackRecordingReq =
            serde_json::from_str(r#"{"trackId":"x","input":{"left":0,"right":1}}"#).unwrap();
        assert_eq!(
            c.input,
            Some(Some(InputAssignment {
                left: 0,
                right: Some(1)
            }))
        );
    }
}
