//! Modelo de domínio do contexto Project.

use serde::{Deserialize, Serialize};
use shared_kernel::command::CommandError;
use shared_kernel::define_id;
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::tempo::TempoError;
use shared_kernel::{AudioFormat, Db, TempoMap, Tick};

mod edit;
pub use edit::{ClipEdit, FadeShape};

define_id!(
    /// Identificador do projeto.
    ProjectId
);
define_id!(
    /// Identificador de clipe.
    ClipId
);

pub const MAX_NAME_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProjectError {
    #[error("nome inválido: deve ter de 1 a {MAX_NAME_LEN} caracteres visíveis")]
    InvalidName,
    #[error("faixa não encontrada")]
    TrackNotFound,
    #[error("clipe não encontrado")]
    ClipNotFound,
    #[error("fonte de áudio não encontrada")]
    SourceNotFound,
    #[error("já existe uma fonte de áudio com este conteúdo")]
    DuplicateSource,
    #[error("a fonte de áudio ainda é usada por clipes")]
    SourceInUse,
    #[error("já existe um item com este identificador")]
    DuplicateId,
    #[error("trecho do clipe fora dos limites da fonte de áudio")]
    ClipOutOfBounds,
    #[error("posição inválida")]
    InvalidPosition,
    #[error("caminho de arquivo inválido: {0}")]
    InvalidPath(String),
    #[error("hash de conteúdo inválido")]
    InvalidHash,
    #[error(
        "fades inválidos: fade de entrada + fade de saída não podem exceder a duração do clipe"
    )]
    InvalidFade,
    #[error("ponto de divisão fora do clipe")]
    InvalidSplit,
    #[error("os clipes precisam estar na mesma faixa e se sobrepor")]
    ClipsDoNotOverlap,
    #[error(transparent)]
    Tempo(#[from] TempoError),
}

impl ProjectError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidName => "project.invalid_name",
            Self::TrackNotFound => "project.track_not_found",
            Self::ClipNotFound => "project.clip_not_found",
            Self::SourceNotFound => "project.source_not_found",
            Self::DuplicateSource => "project.duplicate_source",
            Self::SourceInUse => "project.source_in_use",
            Self::DuplicateId => "project.duplicate_id",
            Self::ClipOutOfBounds => "project.clip_out_of_bounds",
            Self::InvalidPosition => "project.invalid_position",
            Self::InvalidPath(_) => "project.invalid_path",
            Self::InvalidHash => "project.invalid_hash",
            Self::InvalidFade => "project.invalid_fade",
            Self::InvalidSplit => "project.invalid_split",
            Self::ClipsDoNotOverlap => "project.clips_do_not_overlap",
            Self::Tempo(_) => "project.tempo",
        }
    }
}

impl From<ProjectError> for CommandError {
    fn from(e: ProjectError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

/// Valida e normaliza um nome (faixa ou projeto).
pub fn validate_name(name: &str) -> Result<String, ProjectError> {
    let trimmed = name.trim();
    let len = trimmed.chars().count();
    if len == 0 || len > MAX_NAME_LEN || trimmed.chars().any(char::is_control) {
        return Err(ProjectError::InvalidName);
    }
    Ok(trimmed.to_owned())
}

/// Caminho relativo seguro dentro da pasta do projeto (sem `..`, sem raiz, separador `/`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RelPath(String);

impl RelPath {
    pub fn new(p: impl Into<String>) -> Result<Self, ProjectError> {
        let p: String = p.into();
        let bad = p.is_empty()
            || p.starts_with('/')
            || p.contains('\\')
            || p.contains(':')
            || p.chars().any(char::is_control)
            || p.split('/')
                .any(|seg| seg.is_empty() || seg == "." || seg == "..");
        if bad {
            return Err(ProjectError::InvalidPath(p));
        }
        Ok(Self(p))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RelPath {
    type Error = ProjectError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        RelPath::new(s)
    }
}

impl From<RelPath> for String {
    fn from(p: RelPath) -> String {
        p.0
    }
}

/// Fonte de áudio copiada para dentro do projeto. `length_frames` está na taxa do projeto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSource {
    pub id: AudioSourceId,
    pub content_hash: String,
    pub rel_path: RelPath,
    pub original_name: String,
    pub channels: u16,
    pub length_frames: u64,
}

impl AudioSource {
    pub fn validate(&self) -> Result<(), ProjectError> {
        let hash_ok = self.content_hash.len() == 64
            && self.content_hash.chars().all(|c| c.is_ascii_hexdigit());
        if !hash_ok {
            return Err(ProjectError::InvalidHash);
        }
        if self.channels == 0 || self.length_frames == 0 {
            return Err(ProjectError::ClipOutOfBounds);
        }
        Ok(())
    }
}

/// Trecho de uma fonte de áudio posicionado no tempo musical.
///
/// Edição não destrutiva (RNF-07): a fonte nunca é alterada; `offset_frames`/`length_frames`
/// recortam a região e [`ClipEdit`] descreve o processamento aplicado na reprodução.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: ClipId,
    pub source_id: AudioSourceId,
    pub start: Tick,
    pub offset_frames: u64,
    pub length_frames: u64,
    pub gain: Db,
    /// Fades, polaridade e reverso (Fase 3).
    #[serde(flatten)]
    pub edit: ClipEdit,
}

impl Clip {
    /// Clipe sem processamento (ganho unitário, sem fades).
    pub fn new(
        source_id: AudioSourceId,
        start: Tick,
        offset_frames: u64,
        length_frames: u64,
    ) -> Self {
        Self {
            id: ClipId::new(),
            source_id,
            start,
            offset_frames,
            length_frames,
            gain: Db::UNITY,
            edit: ClipEdit::default(),
        }
    }

    /// Fim da região na fonte (sem overflow).
    pub fn source_end(&self) -> u64 {
        self.offset_frames.saturating_add(self.length_frames)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub kind: TrackKind,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new_audio(name: &str) -> Result<Self, ProjectError> {
        Ok(Self {
            id: TrackId::new(),
            name: validate_name(name)?,
            kind: TrackKind::Audio,
            clips: Vec::new(),
        })
    }
}

/// Agregado raiz do arranjo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    name: String,
    pub format: AudioFormat,
    tempo: TempoMap,
    tracks: Vec<Track>,
    sources: Vec<AudioSource>,
}

impl Project {
    pub fn new(name: &str, format: AudioFormat) -> Result<Self, ProjectError> {
        Ok(Self {
            id: ProjectId::new(),
            name: validate_name(name)?,
            format,
            tempo: TempoMap::default(),
            tracks: Vec::new(),
            sources: Vec::new(),
        })
    }

    /// Reconstrói a partir de dados persistidos, validando todas as invariantes.
    pub fn restore(
        id: ProjectId,
        name: String,
        format: AudioFormat,
        tempo: TempoMap,
        tracks: Vec<Track>,
        sources: Vec<AudioSource>,
    ) -> Result<Self, ProjectError> {
        let mut p = Self {
            id,
            name: validate_name(&name)?,
            format,
            tempo,
            tracks: Vec::new(),
            sources: Vec::new(),
        };
        for s in sources {
            p.add_source(s)?;
        }
        for t in tracks {
            let clips = t.clips.clone();
            p.insert_track(
                Track {
                    clips: Vec::new(),
                    ..t
                },
                usize::MAX,
            )?;
            let tid = p
                .tracks
                .last()
                .map(|t| t.id)
                .ok_or(ProjectError::TrackNotFound)?;
            for c in clips {
                p.insert_clip(tid, c, usize::MAX)?;
            }
        }
        Ok(p)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tempo(&self) -> &TempoMap {
        &self.tempo
    }

    pub fn tempo_mut(&mut self) -> &mut TempoMap {
        &mut self.tempo
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn sources(&self) -> &[AudioSource] {
        &self.sources
    }

    pub fn track(&self, id: TrackId) -> Result<&Track, ProjectError> {
        self.tracks
            .iter()
            .find(|t| t.id == id)
            .ok_or(ProjectError::TrackNotFound)
    }

    pub fn track_index(&self, id: TrackId) -> Result<usize, ProjectError> {
        self.tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or(ProjectError::TrackNotFound)
    }

    pub fn source(&self, id: AudioSourceId) -> Result<&AudioSource, ProjectError> {
        self.sources
            .iter()
            .find(|s| s.id == id)
            .ok_or(ProjectError::SourceNotFound)
    }

    pub fn source_by_hash(&self, hash: &str) -> Option<&AudioSource> {
        self.sources.iter().find(|s| s.content_hash == hash)
    }

    /// Localiza um clipe: (índice da faixa, índice do clipe).
    pub fn find_clip(&self, id: ClipId) -> Result<(usize, usize), ProjectError> {
        for (ti, t) in self.tracks.iter().enumerate() {
            if let Some(ci) = t.clips.iter().position(|c| c.id == id) {
                return Ok((ti, ci));
            }
        }
        Err(ProjectError::ClipNotFound)
    }

    pub fn clip(&self, id: ClipId) -> Result<&Clip, ProjectError> {
        let (t, c) = self.find_clip(id)?;
        Ok(&self.tracks[t].clips[c])
    }

    /// Renomeia o projeto. Retorna o nome anterior.
    pub fn rename(&mut self, name: &str) -> Result<String, ProjectError> {
        let n = validate_name(name)?;
        Ok(std::mem::replace(&mut self.name, n))
    }

    /// Insere uma faixa na posição (limitada ao fim). Retorna o índice efetivo.
    pub fn insert_track(&mut self, track: Track, index: usize) -> Result<usize, ProjectError> {
        validate_name(&track.name)?;
        if self.tracks.iter().any(|t| t.id == track.id) {
            return Err(ProjectError::DuplicateId);
        }
        for c in &track.clips {
            self.check_clip(c)?;
        }
        let i = index.min(self.tracks.len());
        self.tracks.insert(i, track);
        Ok(i)
    }

    /// Remove uma faixa (com seus clipes). Retorna a faixa e o índice para desfazer.
    pub fn remove_track(&mut self, id: TrackId) -> Result<(Track, usize), ProjectError> {
        let i = self.track_index(id)?;
        Ok((self.tracks.remove(i), i))
    }

    pub fn rename_track(&mut self, id: TrackId, name: &str) -> Result<String, ProjectError> {
        let n = validate_name(name)?;
        let i = self.track_index(id)?;
        Ok(std::mem::replace(&mut self.tracks[i].name, n))
    }

    /// Move a faixa para `to` (limitado). Retorna o índice anterior.
    pub fn move_track(&mut self, id: TrackId, to: usize) -> Result<usize, ProjectError> {
        let from = self.track_index(id)?;
        let t = self.tracks.remove(from);
        let to = to.min(self.tracks.len());
        self.tracks.insert(to, t);
        Ok(from)
    }

    pub fn add_source(&mut self, source: AudioSource) -> Result<(), ProjectError> {
        source.validate()?;
        if self.sources.iter().any(|s| s.id == source.id) {
            return Err(ProjectError::DuplicateId);
        }
        if self
            .sources
            .iter()
            .any(|s| s.content_hash == source.content_hash || s.rel_path == source.rel_path)
        {
            return Err(ProjectError::DuplicateSource);
        }
        self.sources.push(source);
        Ok(())
    }

    /// Remove uma fonte que não esteja em uso.
    pub fn remove_source(&mut self, id: AudioSourceId) -> Result<AudioSource, ProjectError> {
        if self
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .any(|c| c.source_id == id)
        {
            return Err(ProjectError::SourceInUse);
        }
        let i = self
            .sources
            .iter()
            .position(|s| s.id == id)
            .ok_or(ProjectError::SourceNotFound)?;
        Ok(self.sources.remove(i))
    }

    fn check_clip(&self, clip: &Clip) -> Result<(), ProjectError> {
        let src = self.source(clip.source_id)?;
        if clip.start.0 < 0 {
            return Err(ProjectError::InvalidPosition);
        }
        let end = clip
            .offset_frames
            .checked_add(clip.length_frames)
            .ok_or(ProjectError::ClipOutOfBounds)?;
        if clip.length_frames == 0 || end > src.length_frames || !clip.gain.0.is_finite() {
            return Err(ProjectError::ClipOutOfBounds);
        }
        clip.edit.validate(clip.length_frames)
    }

    /// Insere um clipe numa faixa (índice limitado ao fim). Retorna o índice.
    pub fn insert_clip(
        &mut self,
        track: TrackId,
        clip: Clip,
        index: usize,
    ) -> Result<usize, ProjectError> {
        self.check_clip(&clip)?;
        if self.find_clip(clip.id).is_ok() {
            return Err(ProjectError::DuplicateId);
        }
        let ti = self.track_index(track)?;
        let clips = &mut self.tracks[ti].clips;
        let i = index.min(clips.len());
        clips.insert(i, clip);
        Ok(i)
    }

    /// Remove um clipe. Retorna (faixa, clipe, índice) para desfazer.
    pub fn remove_clip(&mut self, id: ClipId) -> Result<(TrackId, Clip, usize), ProjectError> {
        let (ti, ci) = self.find_clip(id)?;
        let track = self.tracks[ti].id;
        Ok((track, self.tracks[ti].clips.remove(ci), ci))
    }

    /// Move um clipe para outra posição/faixa. Retorna (faixa anterior, início anterior, índice anterior).
    pub fn move_clip(
        &mut self,
        id: ClipId,
        to_track: TrackId,
        start: Tick,
    ) -> Result<(TrackId, Tick, usize), ProjectError> {
        if start.0 < 0 {
            return Err(ProjectError::InvalidPosition);
        }
        self.track_index(to_track)?;
        let (from_track, mut clip, index) = self.remove_clip(id)?;
        let old_start = clip.start;
        clip.start = start;
        self.insert_clip(to_track, clip, usize::MAX)?;
        Ok((from_track, old_start, index))
    }

    /// Restaura um clipe exatamente em (faixa, índice) — usado ao desfazer.
    pub fn place_clip(
        &mut self,
        track: TrackId,
        clip: Clip,
        index: usize,
    ) -> Result<(), ProjectError> {
        self.insert_clip(track, clip, index).map(|_| ())
    }

    pub fn set_clip_gain(&mut self, id: ClipId, gain: Db) -> Result<Db, ProjectError> {
        if !gain.0.is_finite() {
            return Err(ProjectError::ClipOutOfBounds);
        }
        let (ti, ci) = self.find_clip(id)?;
        Ok(std::mem::replace(
            &mut self.tracks[ti].clips[ci].gain,
            gain.clamp_fader(),
        ))
    }

    /// Fim do conteúdo do projeto em amostras (último fim de clipe).
    pub fn content_end_frame(&self) -> u64 {
        let sr = self.format.sample_rate.hz();
        self.tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| {
                self.tempo
                    .tick_to_frame(c.start, sr)
                    .saturating_add(c.length_frames)
            })
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub fn source(frames: u64) -> AudioSource {
        let id = AudioSourceId::new();
        let hash = format!("{:0>64}", id.0.simple().to_string());
        AudioSource {
            id,
            rel_path: RelPath::new(format!("audio/imported/{hash}.wav")).unwrap(),
            content_hash: hash,
            original_name: "voz.wav".into(),
            channels: 2,
            length_frames: frames,
        }
    }

    pub fn clip(source: &AudioSource, start: i64, len: u64) -> Clip {
        Clip {
            id: ClipId::new(),
            source_id: source.id,
            start: Tick(start),
            offset_frames: 0,
            length_frames: len,
            gain: Db::UNITY,
            edit: ClipEdit::default(),
        }
    }

    pub fn project_with_track() -> (Project, TrackId, AudioSource) {
        let mut p = Project::new("Demo", AudioFormat::default()).unwrap();
        let s = source(48_000);
        p.add_source(s.clone()).unwrap();
        let t = Track::new_audio("Voz").unwrap();
        let id = t.id;
        p.insert_track(t, 0).unwrap();
        (p, id, s)
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn names_are_validated_and_trimmed() {
        assert_eq!(validate_name("  Voz  ").unwrap(), "Voz");
        assert!(validate_name("   ").is_err());
        assert!(validate_name("a\u{0007}b").is_err());
        assert!(validate_name(&"x".repeat(65)).is_err());
        assert!(
            validate_name(&"ç".repeat(64)).is_ok(),
            "conta caracteres, não bytes"
        );
    }

    #[test]
    fn rel_path_rejects_traversal_and_absolute() {
        for bad in [
            "../x.wav",
            "/etc/passwd",
            "audio/../../x",
            "C:\\x",
            "a//b",
            "a/./b",
            "",
            "a\\b",
        ] {
            assert!(RelPath::new(bad).is_err(), "{bad} deveria ser rejeitado");
        }
        assert!(RelPath::new("audio/imported/abc.wav").is_ok());
    }

    #[test]
    fn clip_must_fit_source() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 0, 48_000);
        c.offset_frames = 1;
        assert_eq!(p.insert_clip(t, c, 0), Err(ProjectError::ClipOutOfBounds));
        assert_eq!(
            p.insert_clip(t, clip(&s, 0, 0), 0),
            Err(ProjectError::ClipOutOfBounds)
        );
        assert_eq!(
            p.insert_clip(t, clip(&s, -1, 10), 0),
            Err(ProjectError::InvalidPosition)
        );
        let mut huge = clip(&s, 0, 10);
        huge.offset_frames = u64::MAX;
        assert_eq!(
            p.insert_clip(t, huge, 0),
            Err(ProjectError::ClipOutOfBounds),
            "sem overflow"
        );
        assert!(p.insert_clip(t, clip(&s, 0, 48_000), 0).is_ok());
    }

    #[test]
    fn source_in_use_cannot_be_removed_and_duplicates_rejected() {
        let (mut p, t, s) = project_with_track();
        p.insert_clip(t, clip(&s, 0, 10), 0).unwrap();
        assert_eq!(p.remove_source(s.id), Err(ProjectError::SourceInUse));
        let mut dup = source(10);
        dup.content_hash = s.content_hash.clone();
        assert_eq!(p.add_source(dup), Err(ProjectError::DuplicateSource));
        let mut bad = source(10);
        bad.content_hash = "zz".into();
        assert_eq!(p.add_source(bad), Err(ProjectError::InvalidHash));
    }

    #[test]
    fn move_clip_between_tracks() {
        let (mut p, t1, s) = project_with_track();
        let t2 = Track::new_audio("Violão").unwrap();
        let t2id = t2.id;
        p.insert_track(t2, 1).unwrap();
        let c = clip(&s, 0, 10);
        let cid = c.id;
        p.insert_clip(t1, c, 0).unwrap();
        let (from, old, idx) = p.move_clip(cid, t2id, Tick(960)).unwrap();
        assert_eq!((from, old, idx), (t1, Tick(0), 0));
        assert_eq!(p.track(t2id).unwrap().clips[0].start, Tick(960));
        assert!(p.track(t1).unwrap().clips.is_empty());
        assert_eq!(
            p.move_clip(cid, TrackId::new(), Tick(0)),
            Err(ProjectError::TrackNotFound)
        );
        assert_eq!(
            p.clip(cid).unwrap().start,
            Tick(960),
            "falha não altera estado"
        );
    }

    #[test]
    fn content_end_uses_tempo_map() {
        let (mut p, t, s) = project_with_track();
        p.insert_clip(t, clip(&s, 960 * 2, 1000), 0).unwrap(); // 2 semínimas @120 = 1 s = 48000
        assert_eq!(p.content_end_frame(), 49_000);
    }

    #[test]
    fn content_end_saturates_instead_of_overflowing() {
        let (mut p, t, _) = project_with_track();
        let big = source(u64::MAX);
        p.add_source(big.clone()).unwrap();
        p.insert_clip(t, clip(&big, i64::MAX, u64::MAX), 0).unwrap();
        assert_eq!(p.content_end_frame(), u64::MAX);
    }

    #[test]
    fn restore_validates_everything() {
        let (mut p, t, s) = project_with_track();
        p.insert_clip(t, clip(&s, 0, 10), 0).unwrap();
        let r = Project::restore(
            p.id,
            p.name().into(),
            p.format,
            p.tempo().clone(),
            p.tracks().to_vec(),
            p.sources().to_vec(),
        )
        .unwrap();
        assert_eq!(r, p);
        let bad = Project::restore(
            p.id,
            p.name().into(),
            p.format,
            p.tempo().clone(),
            p.tracks().to_vec(),
            vec![],
        );
        assert_eq!(bad, Err(ProjectError::SourceNotFound));
    }
}
