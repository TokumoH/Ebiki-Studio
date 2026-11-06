//! Comandos IPC de composição (Fase 5 Harmonia + Fase 4 Bases) e do sintetizador.
//!
//! Finos: validam/convertem DTOs (camelCase) e delegam ao `Composer`/`Studio` (`app-core`).
//! Contrato completo em `docs/specs/2026-11-05-fase-5-harmonia-e-4-bases.md` §11.
//!
//! Ordem de travas: `composer` → `studio` (nenhum outro comando trava nessa ordem inversa).
//! Áudio de audição volta como `tauri::ipc::Response` (bytes WAV PCM 16 bits → `ArrayBuffer`).

use crate::ipc_args::{bad, user_path, Res};
use crate::state::{lock, AppState};
use app_core::composition::ctx_harmony::{Genre, Key};
use app_core::composition::ctx_library::{Base, BaseId, BaseKey, BaseQuery, MismatchReport};
use app_core::composition::*;
use app_core::AppError;
use infra_adapters::{SynthStatus, SOUNDFONT_SETTING};
use infra_persistence::library::LibraryDb;
use serde::Deserialize;
use shared_kernel::id::TrackId;
use shared_kernel::Tick;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;
use tauri::{AppHandle, Manager, State};

/// Manifesto das bases CC0 empacotadas (embutido no binário).
const BUNDLED_MANIFEST: &str = include_str!("../../../assets/bases/manifest.json");

fn base_id(s: &str) -> Res<BaseId> {
    s.parse().map_err(|_| bad("identificador de base inválido"))
}

fn track_id(s: &Option<String>) -> Res<Option<TrackId>> {
    s.as_deref()
        .map(|t| {
            t.parse()
                .map_err(|_| bad("identificador de faixa inválido"))
        })
        .transpose()
}

fn tick(t: i64) -> Res<Tick> {
    if t < 0 {
        return Err(bad("posição negativa"));
    }
    Ok(Tick(t))
}

/// Abre a biblioteca de composição em `data_dir` e semeia catálogo, CC0 e bases iniciais.
/// Se o `library.db` não abrir, usa memória (o app continua funcionando, sem persistir).
pub fn open_composer(data_dir: &Path, bundled_dir: PathBuf) -> Res<Composer> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| AppError::new("app.io", format!("Erro de arquivo: {e}")))?;
    let roots = BaseRoots {
        user: data_dir.join("bases"),
        bundled: bundled_dir,
    };
    let store: Box<dyn LibraryStore> = match LibraryDb::open(&data_dir.join("library.db")) {
        Ok(db) => {
            let manifest = BundledManifest::parse(BUNDLED_MANIFEST)?;
            db.seed_bundled_bases(&manifest.bases()?, manifest.version)
                .map_err(|e| AppError::new(e.code(), e.to_string()))?;
            db.seed_starter_bases(&starter_bases()?, STARTER_BASES_VERSION)
                .map_err(|e| AppError::new(e.code(), e.to_string()))?;
            Box::new(db)
        }
        Err(_) => Box::new(MemoryLibraryStore::new(Library {
            bases: app_core::composition::ctx_library::BaseCatalog::from_bases(starter_bases()?)
                .map_err(|e| AppError::new(e.code(), e.to_string()))?,
            ..Library::default()
        })),
    };
    Composer::open(store, roots)
}

/// Trava o `Composer`, abrindo-o na primeira chamada.
fn composer<'a, R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &'a AppState,
) -> Res<MutexGuard<'a, Option<Composer>>> {
    let mut guard = lock(&state.composer);
    if guard.is_none() {
        let data = app
            .path()
            .app_data_dir()
            .map_err(|e| AppError::new("app.io", e.to_string()))?;
        let bundled = app
            .path()
            .resource_dir()
            .map(|d| d.join("assets").join("bases"))
            .unwrap_or_else(|_| data.join("bundled-bases"));
        *guard = Some(open_composer(&data, bundled)?);
    }
    Ok(guard)
}

macro_rules! with_composer {
    ($app:expr, $state:expr, |$c:ident| $body:expr) => {{
        let mut guard = composer(&$app, &$state)?;
        let $c = guard
            .as_mut()
            .ok_or_else(|| AppError::new("app.internal", "Biblioteca indisponível."))?;
        $body
    }};
}

fn project_bpm(state: &AppState) -> f64 {
    lock(&state.studio)
        .project_bpm_at(Tick::ZERO)
        .unwrap_or(120.0)
}

// ------------------------------------------------------------------ harmonia

#[tauri::command]
pub fn harmony_keys() -> KeysCatalog {
    keys_catalog()
}

#[tauri::command]
pub fn harmony_field(key: Key) -> HarmonicFieldView {
    field_view(key)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionsQueryReq {
    pub key: Key,
    #[serde(default)]
    pub genre: Option<Genre>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[tauri::command]
pub fn harmony_progressions<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    req: ProgressionsQueryReq,
) -> Res<Vec<ProgressionSuggestionView>> {
    with_composer!(app, state, |c| Ok(c.suggest_progressions(
        req.key,
        req.genre,
        req.limit.unwrap_or(20)
    )))
}

#[tauri::command]
pub fn harmony_user_progressions<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
) -> Res<Vec<ProgressionView>> {
    with_composer!(app, state, |c| Ok(c.user_progressions()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestNextReq {
    pub key: Key,
    #[serde(default)]
    pub history: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[tauri::command]
pub fn harmony_suggest_next<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    req: SuggestNextReq,
) -> Res<Vec<ChordSuggestionView>> {
    with_composer!(app, state, |c| c.suggest_next(
        req.key,
        &req.history,
        req.limit.unwrap_or(8)
    ))
}

#[tauri::command]
pub fn harmony_realize<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    req: RealizeRequest,
) -> Res<Realization> {
    let bpm = project_bpm(&state);
    with_composer!(app, state, |c| c.realize(&req, bpm))
}

#[tauri::command]
pub async fn harmony_audition<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    req: RealizeRequest,
) -> Res<tauri::ipc::Response> {
    let bpm = project_bpm(&state);
    let r = with_composer!(app, state, |c| c.realize(&req, bpm))?;
    let bytes = lock(&state.studio).audition_realization(&r)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertProgressionReq {
    pub realize: RealizeRequest,
    pub at_tick: i64,
    #[serde(default)]
    pub track_id: Option<String>,
}

#[tauri::command]
pub async fn harmony_insert_progression<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    req: InsertProgressionReq,
) -> Res<InsertedClip> {
    let at = tick(req.at_tick)?;
    let track = track_id(&req.track_id)?;
    with_composer!(app, state, |c| lock(&state.studio).insert_progression(
        c,
        &req.realize,
        track,
        at
    ))
}

#[tauri::command]
pub fn harmony_user_progression_add<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    input: UserProgressionInput,
) -> Res<ProgressionView> {
    with_composer!(app, state, |c| c.add_user_progression(&input))
}

#[tauri::command]
pub fn harmony_user_progression_edit<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    id: String,
    input: UserProgressionInput,
) -> Res<ProgressionView> {
    with_composer!(app, state, |c| c.edit_user_progression(&id, &input))
}

#[tauri::command]
pub fn harmony_user_progression_remove<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    id: String,
) -> Res<LibraryHistoryView> {
    with_composer!(app, state, |c| {
        c.remove_user_progression(&id)?;
        Ok(c.history_view())
    })
}

// --------------------------------------------------------- histórico da biblioteca

#[tauri::command]
pub fn library_history<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
) -> Res<LibraryHistoryView> {
    with_composer!(app, state, |c| Ok(c.history_view()))
}

#[tauri::command]
pub fn library_undo<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
) -> Res<LibraryHistoryView> {
    with_composer!(app, state, |c| c.undo())
}

#[tauri::command]
pub fn library_redo<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
) -> Res<LibraryHistoryView> {
    with_composer!(app, state, |c| c.redo())
}

// ------------------------------------------------------------------ bases

#[tauri::command]
pub fn bases_options() -> BaseOptionsView {
    base_options()
}

#[tauri::command]
pub fn bases_search<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    query: BaseQuery,
) -> Res<Vec<Base>> {
    with_composer!(app, state, |c| c.search_bases(&query))
}

#[tauri::command]
pub fn bases_add_user<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    path: String,
    meta: NewUserBase,
) -> Res<Base> {
    let src = user_path(&path)?;
    with_composer!(app, state, |c| c.add_user_base(&src, &meta))
}

#[tauri::command]
pub fn bases_generate<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    req: GenerateBaseRequest,
) -> Res<GeneratedBaseView> {
    with_composer!(app, state, |c| c.generate_base(&req))
}

#[tauri::command]
pub async fn bases_generated_audition<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    req: GenerateBaseRequest,
) -> Res<tauri::ipc::Response> {
    let g = with_composer!(app, state, |c| c.generate_base(&req))?;
    let tempo = shared_kernel::TempoMap::new(g.bpm, g.meter.numerator(), g.meter.denominator())
        .map_err(|e| bad(&e.to_string()))?;
    let bytes = lock(&state.studio).audition_midi(&g.events, &tempo, AUDITION_TAIL_SECONDS)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn bases_save_generated<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    req: SaveGeneratedBaseRequest,
) -> Res<Base> {
    with_composer!(app, state, |c| c.save_generated_base(&req))
}

#[tauri::command]
pub fn bases_remove<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    id: String,
) -> Res<LibraryHistoryView> {
    let id = base_id(&id)?;
    with_composer!(app, state, |c| {
        c.remove_base(id)?;
        Ok(c.history_view())
    })
}

#[tauri::command]
pub async fn bases_audition<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
) -> Res<tauri::ipc::Response> {
    let id = base_id(&id)?;
    let bytes = with_composer!(app, state, |c| lock(&state.studio).audition_base(c, id))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MismatchReq {
    pub base_id: String,
    pub at_tick: i64,
    #[serde(default)]
    pub project_key: Option<BaseKey>,
}

#[tauri::command]
pub fn bases_mismatch<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<AppState>,
    req: MismatchReq,
) -> Res<MismatchReport> {
    let id = base_id(&req.base_id)?;
    let at = tick(req.at_tick)?;
    with_composer!(app, state, |c| lock(&state.studio).base_mismatch(
        c,
        id,
        at,
        req.project_key
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertBaseReq {
    pub base_id: String,
    pub at_tick: i64,
    #[serde(default)]
    pub track_id: Option<String>,
    #[serde(default)]
    pub project_key: Option<BaseKey>,
    /// `"keep"` | `"adapt"`; obrigatória quando há divergência.
    #[serde(default)]
    pub decision: Option<BaseDecision>,
}

#[tauri::command]
pub async fn bases_insert<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    req: InsertBaseReq,
) -> Res<InsertBaseOutcome> {
    let r = InsertBaseRequest {
        base_id: base_id(&req.base_id)?,
        at: tick(req.at_tick)?,
        track: track_id(&req.track_id)?,
        project_key: req.project_key,
        decision: req.decision,
    };
    with_composer!(app, state, |c| lock(&state.studio).insert_base(c, &r))
}

// --------------------------------------------------------------- sintetizador

#[tauri::command]
pub fn synth_status(state: State<AppState>) -> SynthStatus {
    state.synth.status()
}

/// `path = null` volta ao sintetizador simples. A escolha fica em `library.db` (`synth.soundfont`).
#[tauri::command]
pub async fn synth_set_soundfont(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Res<SynthStatus> {
    let status = match path.as_deref() {
        Some(p) => state.synth.set_soundfont(&user_path(p)?)?,
        None => state.synth.clear_soundfont(),
    };
    if let Some(db) = lock(&state.library).as_ref() {
        // vazio = sintetizador simples (ignorado na inicialização)
        let value = status.soundfont_path.as_deref().unwrap_or("");
        db.set_setting(SOUNDFONT_SETTING, value)
            .map_err(|e| AppError::new(e.code(), e.to_string()))?;
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_composer_seeds_starters_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let c = open_composer(dir.path(), dir.path().join("bundled")).unwrap();
        let n = c.search_bases(&BaseQuery::default()).unwrap().len();
        assert_eq!(n, starter_bases().unwrap().len());
        drop(c);
        let c = open_composer(dir.path(), dir.path().join("bundled")).unwrap();
        assert_eq!(c.search_bases(&BaseQuery::default()).unwrap().len(), n);
        assert_eq!(c.roots().user, dir.path().join("bases"));
    }

    /// Contrato IPC: os exemplos JSON da spec (§11) desserializam nos DTOs.
    #[test]
    fn ipc_contract_examples_deserialize() {
        let realize: RealizeRequest = serde_json::from_str(
            r#"{"progression":{"kind":"builtin","slug":"pop-axis"},
                "key":{"tonic":0,"mode":"ionian"},"bpm":null,"bars":8,
                "voicing":{"low":48,"high":72,"voiceLeading":true,"bass":false,"program":24}}"#,
        )
        .unwrap();
        assert_eq!(realize.bars, Some(8));
        assert_eq!(realize.voicing.program, 24);
        assert_eq!(realize.voicing.bass_program, 33, "padrão");
        let _: RealizeRequest = serde_json::from_str(
            r#"{"progression":{"kind":"roman","steps":"ii7 V7 Imaj7:8","mode":"dorian","beatsPerBar":4},
                "key":{"tonic":2,"mode":"dorian"}}"#,
        )
        .unwrap();
        let _: RealizeRequest = serde_json::from_str(
            r#"{"progression":{"kind":"cifra","chords":"C:4 Am:4 F G7"},"key":{"tonic":0,"mode":"ionian"}}"#,
        )
        .unwrap();
        let _: RealizeRequest = serde_json::from_str(
            r#"{"progression":{"kind":"user","id":"01900000-0000-7000-8000-000000000001"},"key":{"tonic":9,"mode":"aeolian"}}"#,
        )
        .unwrap();
        let ins: InsertProgressionReq = serde_json::from_str(
            r#"{"realize":{"progression":{"kind":"builtin","slug":"pop-axis"},"key":{"tonic":0,"mode":"ionian"}},
                "atTick":3840,"trackId":null}"#,
        )
        .unwrap();
        assert_eq!(ins.at_tick, 3840);
        let q: ProgressionsQueryReq = serde_json::from_str(
            r#"{"key":{"tonic":2,"mode":"ionian"},"genre":"bossaNova","limit":5}"#,
        )
        .unwrap();
        assert_eq!(q.genre, Some(Genre::BossaNova));
        let _: SuggestNextReq =
            serde_json::from_str(r#"{"key":{"tonic":0,"mode":"ionian"},"history":["Dm7","G7"]}"#)
                .unwrap();
        let _: UserProgressionInput = serde_json::from_str(
            r#"{"name":"Minha","mode":"ionian","genres":["samba"],"beatsPerBar":4,"steps":"I VI7 ii7 V7"}"#,
        )
        .unwrap();
        let _: BaseQuery = serde_json::from_str(
            r#"{"text":"bossa","bpmMin":80,"bpmMax":140,"key":{"tonic":2,"mode":"major"},
                "genre":"bossa_nova","instrument":"nylonGuitar","tags":["suave"],"kind":"generated"}"#,
        )
        .unwrap();
        let _: NewUserBase = serde_json::from_str(
            r#"{"name":"Loop","bpm":96,"key":{"tonic":9,"mode":"minor"},"meter":{"numerator":4,"denominator":4},
                "genre":"pop","instruments":["drums","bass"],"tags":["loop"],"progression":"Am F C G"}"#,
        )
        .unwrap();
        let g: GenerateBaseRequest = serde_json::from_str(
            r#"{"progression":{"kind":"builtin","slug":"ii-v-i-major"},"key":{"tonic":2,"mode":"ionian"},
                "bpm":130,"bars":8,"style":"bossaComping","drums":"bossa","bass":null}"#,
        )
        .unwrap();
        let _: SaveGeneratedBaseRequest = serde_json::from_value(serde_json::json!({
            "name": "Bossa", "generation": serde_json::to_value(&g).unwrap(),
            "genre": "bossa_nova", "tags": ["suave"]
        }))
        .unwrap();
        let ib: InsertBaseReq = serde_json::from_str(
            r#"{"baseId":"01900000-0000-7000-8000-000000000001","atTick":0,
                "projectKey":{"tonic":11,"mode":"minor"},"decision":"adapt"}"#,
        )
        .unwrap();
        assert_eq!(ib.decision, Some(BaseDecision::Adapt));
        let _: MismatchReq =
            serde_json::from_str(r#"{"baseId":"01900000-0000-7000-8000-000000000001","atTick":0}"#)
                .unwrap();
    }

    #[test]
    fn outcome_serializes_with_status_tag() {
        let report = MismatchReport {
            bpm_ratio: 1.2,
            semitone_shift: Some(2),
            mode_differs: false,
            needs_decision: true,
        };
        let v = serde_json::to_value(InsertBaseOutcome::NeedsDecision { report }).unwrap();
        assert_eq!(v["status"], "needsDecision");
        assert_eq!(v["report"]["bpmRatio"], 1.2);
        assert_eq!(v["report"]["needsDecision"], true);
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(user_path("relativo.sf2").is_err());
        assert!(tick(-1).is_err());
        assert!(base_id("x").is_err());
        assert!(track_id(&Some("nope".into())).is_err());
    }
}
