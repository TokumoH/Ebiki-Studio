//! `Studio`: serviço de aplicação (Pure Fabrication / Controller do GRASP) que coordena casos de uso,
//! histórico, persistência, mídia e engine. Não contém regra de negócio — ela fica nos contextos.

use crate::compile::{compile, EffectKey, MeterMap};
use crate::ports::*;
use crate::session::*;
use crate::view::*;
use audio_engine::recorder::{RecordingSession, TapConfig};
use audio_engine::{EngineHandle, SourceBuffer, TransportState};
use ctx_mixing::commands::{AddStrip, RemoveStrip};
use ctx_mixing::ChannelStrip;
use ctx_project::commands::{AddClip, AddSource, CreateTrack, DeleteClip, DeleteTrack};
use ctx_project::{AudioSource, Clip, ClipId, Project, RelPath, Track};
use ctx_recording::commands::SetTrackRecording;
use ctx_recording::{Take, TakeId, TrackRecording};
use shared_kernel::command::{Command, CommandError, Composite, History};
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::{AudioFormat, Db, Tick};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub const PROJECT_EXT: &str = "amproj";
pub const DB_FILE: &str = "project.db";
pub const AUTOSAVE_KEEP: usize = 5;

/// Erro de caso de uso (código estável + mensagem PT-BR).
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize)]
#[error("{message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    fn no_project() -> Self {
        Self::new("app.no_project", "Nenhum projeto aberto.")
    }
    fn io(e: std::io::Error) -> Self {
        let msg = if e.kind() == std::io::ErrorKind::StorageFull {
            "Disco cheio.".to_owned()
        } else {
            e.to_string()
        };
        Self::new("app.io", format!("Erro de arquivo: {msg}"))
    }
}

impl From<CommandError> for AppError {
    fn from(e: CommandError) -> Self {
        Self::new(e.code, e.message)
    }
}

impl From<PortError> for AppError {
    fn from(e: PortError) -> Self {
        Self::new(e.code, e.message)
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Dependências injetadas (portas).
#[derive(Clone)]
pub struct StudioDeps {
    pub repo: Arc<dyn SessionRepository>,
    pub media: Arc<dyn MediaPort>,
    pub factory: Arc<dyn ProcessorFactory>,
    pub clock: Arc<dyn Clock>,
    pub synth: Arc<dyn MidiRenderer>,
}

pub(crate) struct OpenProject {
    pub(crate) root: PathBuf,
    pub(crate) session: Session,
    pub(crate) history: History<Session>,
    pub(crate) sources: HashMap<AudioSourceId, Arc<SourceBuffer>>,
    /// Verdadeiro após recuperar um autosave (há mudanças em relação ao arquivo salvo).
    pub(crate) force_dirty: bool,
}

struct ActiveRecording {
    session: RecordingSession,
    tracks: Vec<(TrackId, PathBuf)>,
}

pub struct Studio {
    pub(crate) deps: StudioDeps,
    pub(crate) open: Option<OpenProject>,
    pub(crate) engine: Option<EngineHandle>,
    pub(crate) input_channels: usize,
    sent_effects: HashSet<EffectKey>,
    meters: MeterMap,
    recording: Option<ActiveRecording>,
    /// Afinador de entrada ativo (Fase 8).
    pub(crate) tuner: Option<crate::tuning_service::TunerSession>,
}

/// Nome de pasta seguro a partir do nome do projeto.
pub fn sanitize_folder_name(name: &str) -> String {
    let mut s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.trim_matches(|c| c == '.' || c == ' ').to_owned();
    let upper = s.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "LPT1", "LPT2", "LPT3",
    ];
    if s.is_empty() || reserved.contains(&upper.as_str()) {
        s = format!("_{s}");
    }
    s.chars().take(100).collect()
}

impl Studio {
    pub fn new(deps: StudioDeps) -> Self {
        Self {
            deps,
            open: None,
            engine: None,
            input_channels: 0,
            sent_effects: HashSet::new(),
            meters: MeterMap::default(),
            recording: None,
            tuner: None,
        }
    }

    // ------------------------------------------------------------------ engine

    /// Conecta (ou troca) a engine. O grafo atual é reenviado com processadores novos.
    pub fn attach_engine(&mut self, handle: EngineHandle, input_channels: usize) -> AppResult<()> {
        self.engine = Some(handle);
        self.input_channels = input_channels;
        // a derivação do afinador pertencia à engine anterior
        self.tuner = None;
        self.sent_effects.clear();
        self.sync()
    }

    pub fn detach_engine(&mut self) -> Option<EngineHandle> {
        self.sent_effects.clear();
        self.engine.take()
    }

    fn engine(&mut self) -> AppResult<&mut EngineHandle> {
        self.engine
            .as_mut()
            .ok_or_else(|| AppError::new("app.no_engine", "Engine de áudio indisponível."))
    }

    fn sync(&mut self) -> AppResult<()> {
        let (Some(op), Some(engine)) = (self.open.as_ref(), self.engine.as_mut()) else {
            return Ok(());
        };
        let c = compile(
            &op.session,
            &op.sources,
            self.deps.factory.as_ref(),
            &self.sent_effects,
            self.input_channels,
        );
        engine
            .swap_graph(c.graph)
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        self.sent_effects = c.effects;
        self.meters = c.meters;
        Ok(())
    }

    // --------------------------------------------------------------- projetos

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn project_root(&self) -> Option<&Path> {
        self.open.as_ref().map(|o| o.root.as_path())
    }

    /// Cria `<dir>/<nome>.amproj/` com a estrutura padrão e abre.
    pub fn create_project(
        &mut self,
        dir: &Path,
        name: &str,
        format: AudioFormat,
    ) -> AppResult<SessionView> {
        let project =
            Project::new(name, format).map_err(|e| AppError::from(CommandError::from(e)))?;
        if !dir.is_dir() {
            return Err(AppError::new(
                "app.invalid_dir",
                "A pasta de destino não existe.",
            ));
        }
        let root = dir.join(format!("{}.{PROJECT_EXT}", sanitize_folder_name(name)));
        if root.exists() {
            return Err(AppError::new(
                "app.exists",
                "Já existe um projeto com este nome nesta pasta.",
            ));
        }
        for sub in ["audio/imported", "audio/recorded", "cache", "backups"] {
            std::fs::create_dir_all(root.join(sub)).map_err(AppError::io)?;
        }
        let mut session = Session::new(project);
        // canal master com padrão seguro
        session.mixer = ctx_mixing::Mixer::new();
        self.deps.repo.save(&root.join(DB_FILE), &session)?;
        self.install(root, session, HashMap::new())
    }

    /// Abre a partir da pasta `.amproj` (ou do `project.db` dentro dela).
    pub fn open_project(&mut self, path: &Path) -> AppResult<SessionView> {
        let root = if path.file_name().is_some_and(|f| f == DB_FILE) {
            path.parent().unwrap_or(path).to_path_buf()
        } else {
            path.to_path_buf()
        };
        let session = self.deps.repo.load(&root.join(DB_FILE))?;
        let sources = self.decode_sources(&root, &session.project)?;
        self.install(root, session, sources)
    }

    fn decode_sources(
        &self,
        root: &Path,
        p: &Project,
    ) -> AppResult<HashMap<AudioSourceId, Arc<SourceBuffer>>> {
        let sr = p.format.sample_rate.hz();
        let mut out = HashMap::new();
        for s in p.sources() {
            let file = safe_join(root, &s.rel_path)?;
            match self.deps.media.decode(&file, sr) {
                Ok(buf) => {
                    out.insert(s.id, Arc::new(buf));
                }
                // arquivo ausente/corrompido: o clipe fica mudo, mas o projeto abre
                Err(_) => continue,
            }
        }
        Ok(out)
    }

    fn install(
        &mut self,
        root: PathBuf,
        session: Session,
        sources: HashMap<AudioSourceId, Arc<SourceBuffer>>,
    ) -> AppResult<SessionView> {
        self.open = Some(OpenProject {
            root,
            session,
            history: History::new(),
            sources,
            force_dirty: false,
        });
        self.sent_effects.clear();
        if let Some(e) = self.engine.as_mut() {
            let _ = e.stop(Some(0));
        }
        self.sync()?;
        self.view()
    }

    pub fn save(&mut self) -> AppResult<SessionView> {
        let op = self.open.as_mut().ok_or_else(AppError::no_project)?;
        self.deps.repo.save(&op.root.join(DB_FILE), &op.session)?;
        op.history.mark_saved();
        op.force_dirty = false;
        self.view()
    }

    pub fn close(&mut self) -> AppResult<()> {
        if self.recording.is_some() {
            self.stop_recording()?;
        }
        if let Some(e) = self.engine.as_mut() {
            let _ = e.stop(Some(0));
        }
        self.open = None;
        self.sent_effects.clear();
        let _ = self.sync();
        if let Some(e) = self.engine.as_mut() {
            let _ = e.swap_graph(audio_engine::RenderGraph::empty(e.sample_rate()));
        }
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|o| o.history.is_dirty() || o.force_dirty)
    }

    /// Grava um snapshot em `backups/` se houver alterações não salvas. Retorna se gravou.
    pub fn autosave(&mut self) -> AppResult<bool> {
        let now = self.deps.clock.now_unix();
        let Some(op) = self.open.as_mut() else {
            return Ok(false);
        };
        if !(op.history.is_dirty() || op.force_dirty) {
            return Ok(false);
        }
        let dir = op.root.join("backups");
        std::fs::create_dir_all(&dir).map_err(AppError::io)?;
        let file = dir.join(format!("autosave-{now:012}.db"));
        self.deps.repo.save(&file, &op.session)?;
        rotate_backups(&dir, AUTOSAVE_KEEP).map_err(AppError::io)?;
        Ok(true)
    }

    /// Autosave mais novo que o `project.db` (projeto não foi salvo após a última alteração)?
    pub fn recovery_candidate(root: &Path) -> Option<PathBuf> {
        let db_time = std::fs::metadata(root.join(DB_FILE))
            .and_then(|m| m.modified())
            .ok()?;
        let newest = list_backups(&root.join("backups"))
            .ok()?
            .into_iter()
            .last()?;
        let t = std::fs::metadata(&newest).and_then(|m| m.modified()).ok()?;
        (t > db_time).then_some(newest)
    }

    /// Abre o projeto usando o autosave mais recente (fica marcado como modificado).
    pub fn recover(&mut self, root: &Path) -> AppResult<SessionView> {
        let backup = Self::recovery_candidate(root).ok_or_else(|| {
            AppError::new(
                "app.no_recovery",
                "Não há autosave mais recente para recuperar.",
            )
        })?;
        let session = self.deps.repo.load(&backup)?;
        let sources = self.decode_sources(root, &session.project)?;
        self.install(root.to_path_buf(), session, sources)?;
        if let Some(op) = self.open.as_mut() {
            op.force_dirty = true;
        }
        self.view()
    }

    /// Descarta autosaves (o usuário escolheu manter o arquivo salvo).
    pub fn discard_recovery(root: &Path) -> AppResult<()> {
        for f in list_backups(&root.join("backups")).map_err(AppError::io)? {
            std::fs::remove_file(f).map_err(AppError::io)?;
        }
        Ok(())
    }

    // ----------------------------------------------------------------- leitura

    fn history_view(&self) -> HistoryView {
        let Some(op) = self.open.as_ref() else {
            return HistoryView::default();
        };
        HistoryView {
            can_undo: op.history.can_undo(),
            can_redo: op.history.can_redo(),
            undo_label: op.history.undo_label().map(str::to_owned),
            redo_label: op.history.redo_label().map(str::to_owned),
            dirty: op.history.is_dirty() || op.force_dirty,
        }
    }

    pub fn view(&self) -> AppResult<SessionView> {
        let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
        Ok(session_view(
            &op.session,
            &op.root.to_string_lossy(),
            &self.meters,
            self.history_view(),
        ))
    }

    pub fn session(&self) -> Option<&Session> {
        self.open.as_ref().map(|o| &o.session)
    }

    // ---------------------------------------------------------------- comandos

    /// Executa um comando desfazível. `gesture` agrupa ações contínuas num único desfazer.
    pub fn execute(
        &mut self,
        cmd: Box<dyn Command<Session>>,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        let op = self.open.as_mut().ok_or_else(AppError::no_project)?;
        op.history.execute(&mut op.session, cmd, gesture)?;
        self.sync()?;
        self.view()
    }

    pub fn end_gesture(&mut self) {
        if let Some(op) = self.open.as_mut() {
            op.history.end_gesture();
        }
    }

    pub fn undo(&mut self) -> AppResult<SessionView> {
        let op = self.open.as_mut().ok_or_else(AppError::no_project)?;
        op.history.undo(&mut op.session)?;
        self.sync()?;
        self.view()
    }

    pub fn redo(&mut self) -> AppResult<SessionView> {
        let op = self.open.as_mut().ok_or_else(AppError::no_project)?;
        op.history.redo(&mut op.session)?;
        self.sync()?;
        self.view()
    }

    /// Cria faixa + canal do mixer (um único item de desfazer).
    pub fn create_track(
        &mut self,
        name: &str,
        index: Option<usize>,
    ) -> AppResult<(TrackId, SessionView)> {
        let track = Track::new_audio(name).map_err(|e| AppError::from(CommandError::from(e)))?;
        let id = track.id;
        let idx = index.unwrap_or(usize::MAX);
        let cmd = Composite::new(
            "Criar faixa",
            vec![
                on_project(CreateTrack::new(track, idx)),
                on_mixer(AddStrip::new(id, ChannelStrip::default())),
            ],
        );
        let v = self.execute(Box::new(cmd), None)?;
        Ok((id, v))
    }

    /// Remove faixa, canal e configuração de gravação (um único desfazer).
    pub fn delete_track(&mut self, id: TrackId) -> AppResult<SessionView> {
        let has_strip = self
            .session()
            .is_some_and(|s| s.mixer.strips().contains_key(&id));
        let has_rec = self
            .session()
            .is_some_and(|s| s.recording.tracks().contains_key(&id));
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        if has_rec {
            steps.push(on_recording(SetTrackRecording::new(
                id,
                TrackRecording::default(),
                "Gravação",
            )));
        }
        if has_strip {
            steps.push(on_mixer(RemoveStrip::new(id)));
        }
        steps.push(on_project(DeleteTrack::new(id)));
        self.execute(Box::new(Composite::new("Remover faixa", steps)), None)
    }

    /// Importa arquivos. Sem `track`, cria uma faixa por arquivo. Tudo num único desfazer.
    pub fn import_audio(
        &mut self,
        paths: &[PathBuf],
        track: Option<TrackId>,
        at: Tick,
    ) -> AppResult<SessionView> {
        if paths.is_empty() {
            return self.view();
        }
        let (root, sr) = {
            let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
            (op.root.clone(), op.session.project.format.sample_rate.hz())
        };
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        let mut new_buffers = Vec::new();
        let mut planned_hashes: HashMap<String, AudioSourceId> = HashMap::new();
        for path in paths {
            let m = self.deps.media.import(path, &root, sr)?;
            if m.frames == 0 {
                return Err(AppError::new(
                    "app.empty_audio",
                    format!("O arquivo \"{}\" não tem áudio.", m.original_name),
                ));
            }
            let existing = self
                .session()
                .and_then(|s| s.project.source_by_hash(&m.content_hash).map(|s| s.id));
            let source_id = match existing.or_else(|| planned_hashes.get(&m.content_hash).copied())
            {
                Some(id) => id,
                None => {
                    let rel = RelPath::new(m.rel_path.clone())
                        .map_err(|e| AppError::from(CommandError::from(e)))?;
                    let src = AudioSource {
                        id: AudioSourceId::new(),
                        content_hash: m.content_hash.clone(),
                        rel_path: rel,
                        original_name: m.original_name.clone(),
                        channels: m.channels,
                        length_frames: m.frames,
                    };
                    let file = safe_join(&root, &src.rel_path)?;
                    new_buffers.push((src.id, self.deps.media.decode(&file, sr)?));
                    planned_hashes.insert(m.content_hash.clone(), src.id);
                    let id = src.id;
                    steps.push(on_project(AddSource::new(src)));
                    id
                }
            };
            let target = match track {
                Some(t) => t,
                None => {
                    let stem = Path::new(&m.original_name)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let name: String = stem.chars().filter(|c| !c.is_control()).take(64).collect();
                    let t = Track::new_audio(if name.trim().is_empty() {
                        "Áudio"
                    } else {
                        &name
                    })
                    .map_err(|e| AppError::from(CommandError::from(e)))?;
                    let id = t.id;
                    steps.push(on_project(CreateTrack::new(t, usize::MAX)));
                    steps.push(on_mixer(AddStrip::new(id, ChannelStrip::default())));
                    id
                }
            };
            let clip = Clip {
                id: ClipId::new(),
                source_id,
                start: at,
                offset_frames: 0,
                length_frames: m.frames,
                gain: Db::UNITY,
            };
            steps.push(on_project(AddClip::new(target, clip)));
        }
        if let Some(op) = self.open.as_mut() {
            for (id, buf) in new_buffers {
                op.sources.insert(id, Arc::new(buf));
            }
        }
        self.execute(Box::new(Composite::new("Importar áudio", steps)), None)
    }

    // -------------------------------------------------------------- transporte

    fn tick_to_frame(&self, t: Tick) -> AppResult<u64> {
        let s = self.session().ok_or_else(AppError::no_project)?;
        Ok(s.project
            .tempo()
            .tick_to_frame(t, s.project.format.sample_rate.hz()))
    }

    pub fn play(&mut self) -> AppResult<()> {
        self.engine()?
            .play()
            .map_err(|e| AppError::new("app.engine", e.to_string()))
    }

    pub fn pause(&mut self) -> AppResult<()> {
        self.engine()?
            .pause()
            .map_err(|e| AppError::new("app.engine", e.to_string()))
    }

    pub fn stop(&mut self, return_to_start: bool) -> AppResult<Option<SessionView>> {
        if self.recording.is_some() {
            return self.stop_recording().map(Some);
        }
        self.engine()?
            .stop(return_to_start.then_some(0))
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        Ok(None)
    }

    pub fn seek(&mut self, t: Tick) -> AppResult<()> {
        let f = self.tick_to_frame(Tick(t.0.max(0)))?;
        self.engine()?
            .seek(f)
            .map_err(|e| AppError::new("app.engine", e.to_string()))
    }

    pub fn transport(&self) -> AppResult<TransportView> {
        let s = self.session().ok_or_else(AppError::no_project)?;
        let e = self
            .engine
            .as_ref()
            .ok_or_else(|| AppError::new("app.no_engine", "Engine de áudio indisponível."))?;
        let tel = e.telemetry();
        let frame = tel.position();
        let sr = s.project.format.sample_rate.hz();
        let tempo = s.project.tempo();
        let tick = tempo.frame_to_tick(frame, sr);
        let snap_rs = e.snapshot_record_start();
        Ok(TransportView {
            state: format!("{:?}", tel.state()).to_lowercase(),
            frame,
            tick,
            seconds: frame as f64 / sr as f64,
            bar_beat: tempo.bar_beat(tick),
            bpm: tempo.bpm_at(tick),
            record_start: snap_rs,
        })
    }

    pub fn meters(&self) -> MetersView {
        let Some(e) = self.engine.as_ref() else {
            return MetersView::default();
        };
        let snap = e.snapshot();
        let get = |slot: usize| snap.peaks.get(slot).copied().unwrap_or([0.0, 0.0]);
        MetersView {
            master: get(0),
            tracks: self
                .meters
                .tracks
                .iter()
                .map(|(id, s)| MeterView {
                    id: id.to_string(),
                    peak: get(*s),
                })
                .collect(),
            buses: self
                .meters
                .buses
                .iter()
                .map(|(id, s)| MeterView {
                    id: id.to_string(),
                    peak: get(*s),
                })
                .collect(),
            xruns: snap.xruns,
            record_overflow: snap.record_overflow,
        }
    }

    // --------------------------------------------------------------- gravação

    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Começa a gravar as faixas armadas na posição `at`.
    pub fn start_recording(&mut self, at: Tick, count_in: bool) -> AppResult<()> {
        if self.recording.is_some() {
            return Err(AppError::new("app.already_recording", "Já está gravando."));
        }
        let (root, sr, armed) = {
            let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
            let armed: Vec<_> = op
                .session
                .recording
                .armed_tracks()
                .filter_map(|(id, t)| t.input.map(|i| (id, i)))
                .collect();
            (
                op.root.clone(),
                op.session.project.format.sample_rate.hz(),
                armed,
            )
        };
        if armed.is_empty() {
            return Err(AppError::new(
                "app.nothing_armed",
                "Arme ao menos uma faixa com entrada definida para gravar.",
            ));
        }
        let inputs = self.input_channels;
        if armed.iter().any(|(_, i)| {
            i.left as usize >= inputs || i.right.is_some_and(|r| r as usize >= inputs)
        }) {
            return Err(AppError::new(
                "app.input_unavailable",
                "A entrada escolhida não existe no dispositivo de áudio atual.",
            ));
        }
        let mut taps = Vec::new();
        let mut tracks = Vec::new();
        for (id, inp) in &armed {
            let path = root
                .join("audio/recorded")
                .join(format!("{}.wav", TakeId::new()));
            taps.push(TapConfig {
                key: id.0.as_u128(),
                left: inp.left as usize,
                right: inp.right.map(|r| r as usize),
                path: path.clone(),
            });
            tracks.push((*id, path));
        }
        let (rt, session) = RecordingSession::start(&taps, sr, 8)
            .map_err(|e| AppError::new("app.record_io", e.to_string()))?;
        let start = self.tick_to_frame(at)?;
        let engine = self.engine()?;
        engine
            .attach_recorder(rt)
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        engine
            .record(start, count_in)
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        self.recording = Some(ActiveRecording { session, tracks });
        Ok(())
    }

    /// Para a gravação, finaliza os arquivos e cria takes/clipes (um único desfazer).
    pub fn stop_recording(&mut self) -> AppResult<SessionView> {
        let rec = self
            .recording
            .take()
            .ok_or_else(|| AppError::new("app.not_recording", "Não está gravando."))?;
        let record_start = {
            let engine = self.engine()?;
            let rs = engine.snapshot_record_start();
            engine
                .stop(None)
                .map_err(|e| AppError::new("app.engine", e.to_string()))?;
            engine
                .detach_recorder()
                .map_err(|e| AppError::new("app.engine", e.to_string()))?;
            // espera o núcleo aplicar as mensagens (até 1 s)
            for _ in 0..200 {
                if engine.telemetry().state() == TransportState::Stopped {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            std::thread::sleep(Duration::from_millis(20));
            engine.collect_garbage();
            rs
        };
        let files = rec
            .session
            .finish()
            .map_err(|e| AppError::new("app.record_io", e.to_string()))?;
        let Some(start_frame) = record_start else {
            for f in &files {
                let _ = std::fs::remove_file(&f.path);
            }
            return self.view();
        };
        let (root, sr) = {
            let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
            (op.root.clone(), op.session.project.format.sample_rate.hz())
        };
        let latency = self
            .session()
            .map_or(0, |s| s.recording.latency_compensation_frames as u64);
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        let mut buffers = Vec::new();
        for (track, path) in &rec.tracks {
            let Some(f) = files.iter().find(|f| f.key == track.0.as_u128()) else {
                continue;
            };
            if f.frames <= latency {
                let _ = std::fs::remove_file(&f.path);
                continue;
            }
            let rel = path
                .strip_prefix(&root)
                .map_err(|_| AppError::new("app.path", "Caminho de gravação inválido."))?;
            let rel = RelPath::new(rel.to_string_lossy().replace('\\', "/"))
                .map_err(|e| AppError::from(CommandError::from(e)))?;
            let hash = self.deps.media.hash_file(path)?;
            let src = AudioSource {
                id: AudioSourceId::new(),
                content_hash: hash,
                rel_path: rel,
                original_name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                channels: f.channels,
                length_frames: f.frames,
            };
            buffers.push((src.id, self.deps.media.decode(path, sr)?));
            // posição: tick no início de compasso >= início real; o restante vira offset (alinhamento por amostra)
            let s = self.session().ok_or_else(AppError::no_project)?;
            let tempo = s.project.tempo();
            let mut start_tick = tempo.frame_to_tick(start_frame, sr);
            if tempo.tick_to_frame(start_tick, sr) < start_frame {
                start_tick = Tick(start_tick.0 + 1);
            }
            let start_tick_frame = tempo.tick_to_frame(start_tick, sr);
            let offset = (start_tick_frame - start_frame) + latency;
            if offset >= f.frames {
                continue;
            }
            let mut tr = s.recording.track(*track);
            // a gravação nova substitui o clipe do take ativo anterior
            if let Some(prev) = tr
                .active_take
                .and_then(|a| tr.takes.iter().find(|t| t.id == a))
                .cloned()
            {
                if let Ok(t) = s.project.track(*track) {
                    for c in t.clips.iter().filter(|c| c.source_id == prev.source_id) {
                        steps.push(on_project(DeleteClip::new(c.id)));
                    }
                }
            }
            let take = Take {
                id: TakeId::new(),
                source_id: src.id,
                start: start_tick,
                length_frames: f.frames - offset,
                number: s.recording.next_take_number(*track),
            };
            tr.takes.push(take.clone());
            tr.active_take = Some(take.id);
            let clip = Clip {
                id: ClipId::new(),
                source_id: src.id,
                start: start_tick,
                offset_frames: offset,
                length_frames: f.frames - offset,
                gain: Db::UNITY,
            };
            steps.push(on_project(AddSource::new(src)));
            steps.push(on_project(AddClip::new(*track, clip)));
            steps.push(on_recording(SetTrackRecording::new(*track, tr, "Gravação")));
        }
        if steps.is_empty() {
            return self.view();
        }
        if let Some(op) = self.open.as_mut() {
            for (id, b) in buffers {
                op.sources.insert(id, Arc::new(b));
            }
        }
        self.execute(Box::new(Composite::new("Gravação", steps)), None)
    }

    /// Escolhe outro take como ativo (troca o clipe da faixa).
    pub fn set_active_take(&mut self, track: TrackId, take: TakeId) -> AppResult<SessionView> {
        let s = self.session().ok_or_else(AppError::no_project)?;
        let mut tr = s.recording.track(track);
        let new = tr
            .takes
            .iter()
            .find(|t| t.id == take)
            .cloned()
            .ok_or_else(|| AppError::new("recording.take_not_found", "Take não encontrado."))?;
        let src = s
            .project
            .source(new.source_id)
            .map_err(|e| AppError::from(CommandError::from(e)))?;
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        if let Some(prev) = tr
            .active_take
            .and_then(|a| tr.takes.iter().find(|t| t.id == a))
        {
            if let Ok(t) = s.project.track(track) {
                for c in t.clips.iter().filter(|c| c.source_id == prev.source_id) {
                    steps.push(on_project(DeleteClip::new(c.id)));
                }
            }
        }
        let offset = src.length_frames - new.length_frames;
        steps.push(on_project(AddClip::new(
            track,
            Clip {
                id: ClipId::new(),
                source_id: new.source_id,
                start: new.start,
                offset_frames: offset,
                length_frames: new.length_frames,
                gain: Db::UNITY,
            },
        )));
        tr.active_take = Some(take);
        steps.push(on_recording(SetTrackRecording::new(
            track,
            tr,
            "Escolher take",
        )));
        self.execute(Box::new(Composite::new("Escolher take", steps)), None)
    }

    // --------------------------------------------------- exportação e análise

    /// Renderiza a mixagem final offline (mesma engine do tempo real, processadores novos).
    /// `tail_seconds` acrescenta cauda para reverbs/delays.
    pub fn render_mixdown(
        &self,
        range: Option<(Tick, Tick)>,
        tail_seconds: f64,
    ) -> AppResult<(Vec<f32>, Vec<f32>)> {
        let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
        crate::render::render_session(
            &op.session,
            &op.sources,
            self.deps.factory.as_ref(),
            range,
            tail_seconds,
        )
    }

    /// Exporta a mixagem para `file` no formato pedido.
    pub fn export_mix(
        &self,
        file: &Path,
        settings: &ExportSettings,
        range: Option<(Tick, Tick)>,
    ) -> AppResult<()> {
        let sr = self
            .session()
            .ok_or_else(AppError::no_project)?
            .project
            .format
            .sample_rate
            .hz();
        if settings.format == ExportFormatKind::Mp3 && sr > 48_000 {
            return Err(AppError::new("app.mp3_rate", "MP3 suporta no máximo 48 kHz. Exporte em WAV/FLAC ou use um projeto de 44,1/48 kHz."));
        }
        let (mut l, mut r) = self.render_mixdown(range, 2.0)?;
        if settings.normalize {
            crate::render::normalize(&mut l, &mut r, Db(-1.0).to_linear());
        }
        self.deps.media.encode(file, settings, sr, &[l, r])?;
        Ok(())
    }

    /// Exporta uma faixa por arquivo (stems) em `dir`. Retorna os arquivos criados.
    pub fn export_stems(&self, dir: &Path, settings: &ExportSettings) -> AppResult<Vec<PathBuf>> {
        let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
        let sr = op.session.project.format.sample_rate.hz();
        std::fs::create_dir_all(dir).map_err(AppError::io)?;
        let mut out = Vec::new();
        for (i, t) in op.session.project.tracks().iter().enumerate() {
            let solo = crate::render::solo_session(&op.session, t.id);
            let (l, r) = crate::render::render_session(
                &solo,
                &op.sources,
                self.deps.factory.as_ref(),
                None,
                2.0,
            )?;
            let name = format!(
                "{:02} - {}.{}",
                i + 1,
                sanitize_folder_name(&t.name),
                settings.format.extension()
            );
            let file = dir.join(name);
            self.deps.media.encode(&file, settings, sr, &[l, r])?;
            out.push(file);
        }
        Ok(out)
    }

    /// Analisa a mixagem (offline) e devolve as medições + conselhos por regras.
    pub fn analyze_mix(
        &self,
        target_lufs: f32,
    ) -> AppResult<(
        ctx_mixing::advisor::MixAnalysis,
        Vec<ctx_mixing::advisor::Advice>,
    )> {
        use ctx_mixing::advisor::{MixAdvisor, RuleBasedMixAdvisor};
        let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
        let sr = op.session.project.format.sample_rate.hz();
        let (l, r) = self.render_mixdown(None, 0.5)?;
        let master = crate::render::analyze_master(&l, &r, sr);
        let mut channels = Vec::new();
        for t in op.session.project.tracks().iter().take(64) {
            let solo = crate::render::solo_session(&op.session, t.id);
            let (tl, tr) = crate::render::render_session(
                &solo,
                &op.sources,
                self.deps.factory.as_ref(),
                None,
                0.0,
            )?;
            channels.push(crate::render::analyze_channel(&t.name, &tl, &tr));
        }
        let analysis = ctx_mixing::advisor::MixAnalysis {
            master,
            channels,
            target_lufs,
        };
        let advice = RuleBasedMixAdvisor.advise(&analysis);
        Ok((analysis, advice))
    }

    /// Picos (mín./máx. intercalados por bloco, canais combinados) para desenhar a forma de onda.
    pub fn waveform(&self, source: AudioSourceId, frames_per_peak: u32) -> AppResult<Vec<f32>> {
        let op = self.open.as_ref().ok_or_else(AppError::no_project)?;
        let buf = op.sources.get(&source).ok_or_else(|| {
            AppError::new(
                "app.source_unavailable",
                "Áudio indisponível (arquivo ausente ou corrompido).",
            )
        })?;
        Ok(crate::render::peaks(
            buf,
            frames_per_peak.clamp(16, 65_536) as usize,
        ))
    }

    pub fn available_formats(&self) -> Vec<ExportFormatKind> {
        self.deps.media.available_formats()
    }

    /// Fontes decodificadas (para exportação e análise).
    pub fn sources(&self) -> Option<&HashMap<AudioSourceId, Arc<SourceBuffer>>> {
        self.open.as_ref().map(|o| &o.sources)
    }

    pub fn deps(&self) -> &StudioDeps {
        &self.deps
    }

    /// Registra buffers decodificados de fontes novas (uso dos casos de uso de `composition`).
    pub(crate) fn register_source_buffers(&mut self, buffers: Vec<(AudioSourceId, SourceBuffer)>) {
        if let Some(op) = self.open.as_mut() {
            for (id, buf) in buffers {
                op.sources.insert(id, Arc::new(buf));
            }
        }
    }
}

/// Junta caminho relativo validado à raiz do projeto (defesa em profundidade contra path traversal).
pub fn safe_join(root: &Path, rel: &RelPath) -> AppResult<PathBuf> {
    let p = root.join(rel.as_str());
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
        || !p.starts_with(root)
    {
        return Err(AppError::new("app.path", "Caminho fora do projeto."));
    }
    Ok(p)
}

fn list_backups(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name().is_some_and(|n| {
                    n.to_string_lossy().starts_with("autosave-")
                        && n.to_string_lossy().ends_with(".db")
                })
            })
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    v.sort();
    Ok(v)
}

fn rotate_backups(dir: &Path, keep: usize) -> std::io::Result<()> {
    let files = list_backups(dir)?;
    if files.len() > keep {
        for f in &files[..files.len() - keep] {
            std::fs::remove_file(f)?;
            for ext in ["-wal", "-shm"] {
                let side = PathBuf::from(format!("{}{ext}", f.display()));
                let _ = std::fs::remove_file(side);
            }
        }
    }
    Ok(())
}
