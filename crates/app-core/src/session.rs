//! Sessão: agregados de um projeto aberto e adaptação de comandos de cada contexto.

use ctx_mixing::Mixer;
use ctx_notation::{Score, ScoreBook, ScoreId};
use ctx_project::Project;
use ctx_recording::RecordingSetup;
use ctx_sync::LyricTrack;
use ctx_tuning::{PitchCorrectionMap, TuningBook};
use shared_kernel::command::{Command, CommandError};
use shared_kernel::id::AudioSourceId;

/// Todos os agregados persistidos de um projeto.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub project: Project,
    pub mixer: Mixer,
    pub recording: RecordingSetup,
    /// Letra sincronizada (Fase 6).
    pub lyrics: LyricTrack,
    /// Partituras do projeto (Fase 7).
    pub scores: ScoreBook,
    /// Mapas de correção por fonte e afinador (Fase 8).
    pub tuning: TuningBook,
}

/// Letra vazia (sem linhas): estado inicial de todo projeto.
pub fn empty_lyrics() -> LyricTrack {
    LyricTrack::from_lines(Vec::new()).expect("letra vazia é válida")
}

impl Session {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            mixer: Mixer::new(),
            recording: RecordingSetup::default(),
            lyrics: empty_lyrics(),
            scores: ScoreBook::new(),
            tuning: TuningBook::new(),
        }
    }
}

/// Seleciona um agregado dentro da sessão.
pub trait Lens<A>: Send + 'static {
    fn get(s: &mut Session) -> &mut A;
}

macro_rules! lens {
    ($name:ident, $ty:ty, $field:ident) => {
        pub struct $name;
        impl Lens<$ty> for $name {
            fn get(s: &mut Session) -> &mut $ty {
                &mut s.$field
            }
        }
    };
}

lens!(ProjectLens, Project, project);
lens!(MixerLens, Mixer, mixer);
lens!(RecordingLens, RecordingSetup, recording);
lens!(LyricsLens, LyricTrack, lyrics);
lens!(ScoresLens, ScoreBook, scores);
lens!(TuningLens, TuningBook, tuning);

/// Adapta um `Command<A>` de um contexto para `Command<Session>`.
pub struct Scoped<A, L: Lens<A>> {
    inner: Box<dyn Command<A>>,
    _lens: std::marker::PhantomData<fn() -> L>,
}

impl<A: 'static, L: Lens<A>> Scoped<A, L> {
    pub fn boxed(inner: impl Command<A> + 'static) -> Box<dyn Command<Session>> {
        Box::new(Self {
            inner: Box::new(inner),
            _lens: std::marker::PhantomData,
        })
    }
}

impl<A: 'static, L: Lens<A>> Command<Session> for Scoped<A, L> {
    fn apply(&mut self, s: &mut Session) -> Result<(), CommandError> {
        self.inner.apply(L::get(s))
    }
    fn revert(&mut self, s: &mut Session) -> Result<(), CommandError> {
        self.inner.revert(L::get(s))
    }
    fn label(&self) -> &str {
        self.inner.label()
    }
}

/// Atalhos para envolver comandos de cada contexto.
pub fn on_project(c: impl Command<Project> + 'static) -> Box<dyn Command<Session>> {
    Scoped::<Project, ProjectLens>::boxed(c)
}

pub fn on_mixer(c: impl Command<Mixer> + 'static) -> Box<dyn Command<Session>> {
    Scoped::<Mixer, MixerLens>::boxed(c)
}

pub fn on_recording(c: impl Command<RecordingSetup> + 'static) -> Box<dyn Command<Session>> {
    Scoped::<RecordingSetup, RecordingLens>::boxed(c)
}

pub fn on_lyrics(c: impl Command<LyricTrack> + 'static) -> Box<dyn Command<Session>> {
    Scoped::<LyricTrack, LyricsLens>::boxed(c)
}

pub fn on_scores(c: impl Command<ScoreBook> + 'static) -> Box<dyn Command<Session>> {
    Scoped::<ScoreBook, ScoresLens>::boxed(c)
}

/// Comando de partitura aplicado à partitura `id` (ex.: `EditVoice`, `EditScore`).
pub fn on_score(id: ScoreId, c: impl Command<Score> + 'static) -> Box<dyn Command<Session>> {
    on_scores(ctx_notation::book::OnScore::new(id, c))
}

pub fn on_tuning(c: impl Command<TuningBook> + 'static) -> Box<dyn Command<Session>> {
    Scoped::<TuningBook, TuningLens>::boxed(c)
}

/// Comando de mapa de correção aplicado ao mapa da fonte `source` (ex.: `SetNoteEdit`).
pub fn on_correction_map(
    source: AudioSourceId,
    c: impl Command<PitchCorrectionMap> + 'static,
) -> Box<dyn Command<Session>> {
    on_tuning(ctx_tuning::OnCorrectionMap::new(source, c))
}
