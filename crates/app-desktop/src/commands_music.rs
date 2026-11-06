//! Comandos IPC das Fases 6–8 (partituras, letra, afinação, afinador, time-stretch).
//! Finos: validam/convertem DTOs (camelCase) e delegam ao `Studio`. Contrato documentado nas specs
//! `docs/specs/2026-11-05-fase-7-notacao.md` e `…-fase-6-8-sincronizacao-e-afinacao.md`
//! (seções "Integração (volta 2)").

use crate::ipc_args::{bad, id, user_path, Res};
use crate::state::{lock, AppState};
use app_core::lyrics_service::LyricsFileFormat;
use app_core::notation_service::{
    ScoreExportFormat, ScoreImportResult, ScoreReport, ScoreTemplate,
};
use app_core::tuning_service::TunerReadingView;
use app_core::{ScoreSummary, SessionView};
use ctx_notation::arrange::ChordSlot;
use ctx_notation::commands::{ScoreEdit, VoiceEdit};
use ctx_notation::{KeySignature, Meter, Mode, Score, ScoreId, VoiceLoc};
use ctx_project::ClipId;
use ctx_sync::{Granularity, LyricUnit, Span, SyllableRef};
use ctx_tuning::{DetectedNoteId, NoteEdit, PitchCorrectionMap, ScaleCorrection};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::Tick;
use tauri::State;

/// Resposta de criação de partitura.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreCreated {
    pub score_id: String,
    pub view: SessionView,
}

// ------------------------------------------------------------- partituras

#[tauri::command]
pub fn score_list(state: State<AppState>) -> Res<Vec<ScoreSummary>> {
    Ok(lock(&state.studio).view()?.scores)
}

#[tauri::command]
pub fn score_create(state: State<AppState>, req: ScoreTemplate) -> Res<ScoreCreated> {
    let (sid, view) = lock(&state.studio).score_create(&req)?;
    Ok(ScoreCreated {
        score_id: sid.to_string(),
        view,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreFromChordsReq {
    pub title: String,
    pub meter_numerator: u8,
    pub meter_denominator: u8,
    #[serde(default)]
    pub key_fifths: i8,
    #[serde(default)]
    pub key_mode: Mode,
    pub slots: Vec<ChordSlot>,
}

#[tauri::command]
pub fn score_create_from_chords(
    state: State<AppState>,
    req: ScoreFromChordsReq,
) -> Res<ScoreCreated> {
    let meter =
        Meter::new(req.meter_numerator, req.meter_denominator).map_err(|e| bad(&e.to_string()))?;
    let key = KeySignature::new(req.key_fifths, req.key_mode).map_err(|e| bad(&e.to_string()))?;
    if req.slots.len() > 10_000 {
        return Err(bad("acordes demais"));
    }
    let (sid, view) =
        lock(&state.studio).score_create_from_chords(&req.title, meter, key, &req.slots)?;
    Ok(ScoreCreated {
        score_id: sid.to_string(),
        view,
    })
}

#[tauri::command]
pub fn score_get(state: State<AppState>, score_id: String) -> Res<Score> {
    lock(&state.studio).score_get(id(&score_id)?)
}

#[tauri::command]
pub fn score_delete(state: State<AppState>, score_id: String) -> Res<SessionView> {
    lock(&state.studio).score_delete(id(&score_id)?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreEditVoiceReq {
    pub score_id: String,
    pub loc: VoiceLoc,
    pub edit: VoiceEdit,
    pub gesture: Option<u64>,
}

#[tauri::command]
pub fn score_edit_voice(state: State<AppState>, req: ScoreEditVoiceReq) -> Res<SessionView> {
    let sid: ScoreId = id(&req.score_id)?;
    lock(&state.studio).score_edit_voice(sid, req.loc, req.edit, req.gesture)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreEditReq {
    pub score_id: String,
    pub edit: ScoreEdit,
}

#[tauri::command]
pub fn score_edit(state: State<AppState>, req: ScoreEditReq) -> Res<SessionView> {
    let sid: ScoreId = id(&req.score_id)?;
    lock(&state.studio).score_edit(sid, req.edit)
}

#[tauri::command]
pub fn score_import(state: State<AppState>, path: String) -> Res<ScoreImportResult> {
    let p = user_path(&path)?;
    lock(&state.studio).score_import(&p)
}

/// Importação a partir de bytes lidos na UI (GP3–GP6 já convertidos para GP7 pelo alphaTab,
/// ADR-0020; também aceita MusicXML/MXL/MIDI/GP7 — detecção pelo conteúdo).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreImportBytesReq {
    /// Nome do arquivo (título padrão).
    pub name: String,
    pub bytes: Vec<u8>,
}

#[tauri::command]
pub fn score_import_bytes(
    state: State<AppState>,
    req: ScoreImportBytesReq,
) -> Res<ScoreImportResult> {
    let stem = req
        .name
        .rsplit_once('.')
        .map_or(req.name.as_str(), |(s, _)| s);
    lock(&state.studio).score_import_bytes(&req.bytes, stem)
}

/// MusicXML da partitura (texto) para o renderizador alphaTab da UI.
#[tauri::command]
pub fn score_musicxml(state: State<AppState>, score_id: String) -> Res<String> {
    lock(&state.studio).score_musicxml(id(&score_id)?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreExportReq {
    pub score_id: String,
    pub path: String,
    pub format: ScoreExportFormat,
}

#[tauri::command]
pub fn score_export(state: State<AppState>, req: ScoreExportReq) -> Res<()> {
    let p = user_path(&req.path)?;
    lock(&state.studio).score_export(id(&req.score_id)?, &p, req.format)
}

/// WAV float 32 (bytes crus → `ArrayBuffer` no JS) para audição imediata.
#[tauri::command]
pub fn score_audition(state: State<AppState>, score_id: String) -> Res<tauri::ipc::Response> {
    let bytes = lock(&state.studio).score_audition(id(&score_id)?)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreRenderReq {
    pub score_id: String,
    /// Faixa de destino; ausente = cria uma faixa nova com o título da partitura.
    pub track_id: Option<String>,
    /// Posição na timeline (ticks, padrão 0).
    #[serde(default)]
    pub at_tick: i64,
}

#[tauri::command]
pub fn score_render_to_track(state: State<AppState>, req: ScoreRenderReq) -> Res<SessionView> {
    let track: Option<TrackId> = req.track_id.as_deref().map(id).transpose()?;
    lock(&state.studio).score_render_to_track(id(&req.score_id)?, track, Tick(req.at_tick))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreValidateReq {
    pub score_id: String,
    /// Classes de altura (0 = Dó … 11 = Si) da escala; ausente = armadura de cada compasso.
    pub scale: Option<Vec<u8>>,
}

#[tauri::command]
pub fn score_validate(state: State<AppState>, req: ScoreValidateReq) -> Res<ScoreReport> {
    lock(&state.studio).score_validate(id(&req.score_id)?, req.scale.as_deref())
}

// ------------------------------------------------------------------ letra

#[tauri::command]
pub fn lyrics_set_text(state: State<AppState>, text: String) -> Res<SessionView> {
    lock(&state.studio).lyrics_set_text(&text)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsSpanReq {
    pub unit: LyricUnit,
    pub start: i64,
    pub end: i64,
    pub gesture: Option<u64>,
}

#[tauri::command]
pub fn lyrics_set_span(state: State<AppState>, req: LyricsSpanReq) -> Res<SessionView> {
    let span = Span::new(Tick(req.start), Tick(req.end))
        .map_err(|_| bad("intervalo inválido (início ≥ 0 e menor que o fim)"))?;
    lock(&state.studio).lyrics_set_span(req.unit, span, req.gesture)
}

#[tauri::command]
pub fn lyrics_clear_span(state: State<AppState>, unit: LyricUnit) -> Res<SessionView> {
    lock(&state.studio).lyrics_clear_span(unit)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsShiftReq {
    pub from: SyllableRef,
    pub to: SyllableRef,
    pub delta: i64,
    pub gesture: Option<u64>,
}

#[tauri::command]
pub fn lyrics_shift(state: State<AppState>, req: LyricsShiftReq) -> Res<SessionView> {
    lock(&state.studio).lyrics_shift(req.from, req.to, req.delta, req.gesture)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TapSyncReq {
    pub granularity: Granularity,
    /// Índice da primeira unidade sincronizada (na granularidade).
    #[serde(default)]
    pub from: usize,
    /// Inícios das unidades (ticks), estritamente crescentes.
    pub taps: Vec<i64>,
    /// Fim da última unidade tocada (ticks).
    pub end: i64,
}

#[tauri::command]
pub fn lyrics_apply_tap_sync(state: State<AppState>, req: TapSyncReq) -> Res<SessionView> {
    if req.taps.len() > 100_000 {
        return Err(bad("toques demais"));
    }
    let taps: Vec<Tick> = req.taps.into_iter().map(Tick).collect();
    lock(&state.studio).lyrics_apply_tap_sync(req.granularity, req.from, &taps, Tick(req.end))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsExportReq {
    pub path: String,
    pub format: LyricsFileFormat,
}

#[tauri::command]
pub fn lyrics_export(state: State<AppState>, req: LyricsExportReq) -> Res<()> {
    let p = user_path(&req.path)?;
    lock(&state.studio).lyrics_export(&p, req.format)
}

#[tauri::command]
pub fn lyrics_import(state: State<AppState>, path: String) -> Res<SessionView> {
    let p = user_path(&path)?;
    lock(&state.studio).lyrics_import(&p)
}

// --------------------------------------------------------------- afinação

#[tauri::command]
pub fn tuning_analyze(state: State<AppState>, source_id: String) -> Res<SessionView> {
    let s: AudioSourceId = id(&source_id)?;
    lock(&state.studio).tuning_analyze(s)
}

#[tauri::command]
pub fn tuning_get(state: State<AppState>, source_id: String) -> Res<PitchCorrectionMap> {
    let s: AudioSourceId = id(&source_id)?;
    lock(&state.studio).tuning_get(s)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuningEditNoteReq {
    pub source_id: String,
    pub note_id: String,
    pub edit: NoteEdit,
    pub gesture: Option<u64>,
}

#[tauri::command]
pub fn tuning_edit_note(state: State<AppState>, req: TuningEditNoteReq) -> Res<SessionView> {
    let s: AudioSourceId = id(&req.source_id)?;
    let n: DetectedNoteId = id(&req.note_id)?;
    lock(&state.studio).tuning_edit_note(s, n, req.edit, req.gesture)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuningScaleReq {
    pub source_id: String,
    pub scale: ScaleCorrection,
}

#[tauri::command]
pub fn tuning_set_scale(state: State<AppState>, req: TuningScaleReq) -> Res<SessionView> {
    let s: AudioSourceId = id(&req.source_id)?;
    lock(&state.studio).tuning_set_scale(s, req.scale)
}

#[tauri::command]
pub fn tuning_set_reference(state: State<AppState>, a4_hz: f64) -> Res<SessionView> {
    lock(&state.studio).tuning_set_reference(a4_hz)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuningApplyReq {
    pub source_id: String,
    pub clip_id: String,
}

#[tauri::command]
pub fn tuning_apply(state: State<AppState>, req: TuningApplyReq) -> Res<SessionView> {
    let s: AudioSourceId = id(&req.source_id)?;
    let c: ClipId = id(&req.clip_id)?;
    lock(&state.studio).tuning_apply(s, c)
}

#[tauri::command]
pub fn tuning_clear(state: State<AppState>, source_id: String) -> Res<SessionView> {
    let s: AudioSourceId = id(&source_id)?;
    lock(&state.studio).tuning_clear(s)
}

#[tauri::command]
pub fn tuner_start(state: State<AppState>, input_channel: usize) -> Res<()> {
    lock(&state.studio).tuner_start(input_channel)
}

#[tauri::command]
pub fn tuner_read(state: State<AppState>) -> Res<Option<TunerReadingView>> {
    lock(&state.studio).tuner_read()
}

#[tauri::command]
pub fn tuner_stop(state: State<AppState>) -> Res<()> {
    lock(&state.studio).tuner_stop()
}

// ------------------------------------------------------------ time-stretch

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipStretchReq {
    pub clip_id: String,
    /// Andamento original do áudio.
    pub source_bpm: f64,
    /// Andamento de destino; ausente = andamento do projeto no início do clipe.
    pub target_bpm: Option<f64>,
}

#[tauri::command]
pub fn clip_time_stretch(state: State<AppState>, req: ClipStretchReq) -> Res<SessionView> {
    if !req.source_bpm.is_finite() || req.target_bpm.is_some_and(|b| !b.is_finite()) {
        return Err(bad("andamento inválido"));
    }
    let c: ClipId = id(&req.clip_id)?;
    lock(&state.studio).clip_time_stretch(c, req.source_bpm, req.target_bpm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_deserialize_from_camel_case_json() {
        let r: ScoreEditVoiceReq = serde_json::from_str(
            r#"{"scoreId":"x","loc":{"part":0,"measure":1,"voice":0},
                "edit":{"kind":"setDynamic","index":0,"dynamic":"ff"},"gesture":null}"#,
        )
        .unwrap();
        assert_eq!(r.loc, VoiceLoc::new(0, 1, 0));
        let r: LyricsSpanReq = serde_json::from_str(
            r#"{"unit":{"kind":"word","line":0,"word":2},"start":0,"end":480}"#,
        )
        .unwrap();
        assert!(matches!(r.unit, LyricUnit::Word(_)));
        let r: LyricsExportReq = serde_json::from_str(
            r#"{"path":"/tmp/a.lrc","format":{"format":"lrc","granularity":"word"}}"#,
        )
        .unwrap();
        assert_eq!(
            r.format,
            LyricsFileFormat::Lrc {
                granularity: Granularity::Word
            }
        );
        let r: TuningEditNoteReq = serde_json::from_str(
            r#"{"sourceId":"a","noteId":"b","edit":{"target":69,"centsOffset":0,"flatten":0.5,"formantSemitones":0}}"#,
        )
        .unwrap();
        assert_eq!(r.edit.flatten, 0.5);
        let t: ScoreTemplate = serde_json::from_str(
            r#"{"title":"t","bpm":100,"meterNumerator":3,"meterDenominator":4,"measures":8,
                "instruments":[{"kind":"guitar"}]}"#,
        )
        .unwrap();
        assert_eq!(t.measures, 8);
        let e: ScoreEdit =
            serde_json::from_str(r#"{"kind":"transpose","scope":{"scope":"score"},"semitones":2}"#)
                .unwrap();
        assert!(matches!(e, ScoreEdit::Transpose { semitones: 2, .. }));
        let r: ScoreImportBytesReq =
            serde_json::from_str(r#"{"name":"riff.gp","bytes":[80,75,3,4]}"#).unwrap();
        assert_eq!(r.bytes, b"PK\x03\x04");
    }

    #[test]
    fn rejects_relative_paths_and_bad_ids() {
        assert!(user_path("rel/a.lrc").is_err());
        assert!(id::<ScoreId>("nope").is_err());
    }
}
