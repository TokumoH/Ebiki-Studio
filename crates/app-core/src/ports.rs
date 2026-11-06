//! Portas (interfaces) implementadas pela infraestrutura.

use crate::session::Session;
use audio_engine::{AudioProcessor, SourceBuffer};
use ctx_mixing::Effect;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct PortError {
    pub code: String,
    pub message: String,
}

impl PortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Persistência da sessão num arquivo de banco.
pub trait SessionRepository: Send + Sync {
    fn save(&self, db_path: &Path, session: &Session) -> Result<(), PortError>;
    fn load(&self, db_path: &Path) -> Result<Session, PortError>;
}

/// Arquivo de áudio já copiado para dentro do projeto.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMedia {
    pub content_hash: String,
    /// Caminho relativo à raiz do projeto (`audio/imported/<hash>.<ext>`).
    pub rel_path: String,
    pub original_name: String,
    pub channels: u16,
    /// Duração na taxa do projeto.
    pub frames: u64,
}

/// Mídia: importação, decodificação e registro de gravações.
pub trait MediaPort: Send + Sync {
    /// Copia `src` para dentro do projeto (deduplicando por hash) e informa os metadados.
    fn import(
        &self,
        src: &Path,
        project_root: &Path,
        sample_rate: u32,
    ) -> Result<ImportedMedia, PortError>;
    /// Decodifica um arquivo do projeto para a taxa do projeto.
    fn decode(&self, file: &Path, sample_rate: u32) -> Result<SourceBuffer, PortError>;
    /// Calcula o hash de um arquivo já dentro do projeto (gravações).
    fn hash_file(&self, file: &Path) -> Result<String, PortError>;
    /// Codifica canais não intercalados num arquivo (escrita atômica).
    fn encode(
        &self,
        file: &Path,
        settings: &ExportSettings,
        sample_rate: u32,
        planes: &[Vec<f32>],
    ) -> Result<(), PortError>;
    /// Formatos que este build consegue exportar.
    fn available_formats(&self) -> Vec<ExportFormatKind>;
    /// Espectrograma (Fase 3, RF-3.5) de um áudio decodificado, no formato binário `AMSG`
    /// documentado em `infra_codecs::spectrogram`. `cache` = (raiz do projeto, hash do conteúdo)
    /// para reaproveitar o cache regenerável em `cache/spectrogram/`.
    fn spectrogram(
        &self,
        _audio: &SourceBuffer,
        _sample_rate: u32,
        _params: SpectrogramRequest,
        _cache: Option<(&Path, &str)>,
    ) -> Result<Vec<u8>, PortError> {
        Err(PortError::new(
            "media.unsupported",
            "Espectrograma indisponível neste build.",
        ))
    }
}

/// Parâmetros do espectrograma pedidos pela UI (validados pela implementação).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectrogramRequest {
    pub fft_size: u32,
    pub hop: u32,
    pub bins: u16,
}

impl Default for SpectrogramRequest {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop: 512,
            bins: 128,
        }
    }
}

/// Formato de exportação (independente da biblioteca de codecs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormatKind {
    Wav,
    Aiff,
    Flac,
    Mp3,
    Ogg,
    Aac,
}

impl ExportFormatKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::Ogg => "ogg",
            Self::Aac => "m4a",
        }
    }
}

/// Configuração de exportação vinda da UI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub format: ExportFormatKind,
    /// 16, 24 ou 32 (float) para WAV/AIFF; 16/24 para FLAC.
    pub bit_depth: u16,
    /// MP3: kbps CBR (128–320); `None` = VBR V0.
    pub mp3_kbps: Option<u16>,
    /// OGG: qualidade −0,2 a 1,0.
    pub vorbis_quality: f32,
    /// Normaliza o pico para −1 dBFS antes de codificar.
    pub normalize: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: ExportFormatKind::Wav,
            bit_depth: 24,
            mp3_kbps: Some(320),
            vorbis_quality: 0.6,
            normalize: false,
        }
    }
}

/// Fábrica de processadores de efeito (adaptador para a biblioteca de DSP).
pub trait ProcessorFactory: Send + Sync {
    /// Tag estável do tipo (efeitos com tags diferentes não herdam estado).
    fn type_tag(&self, effect: &Effect) -> u32;
    /// Parâmetros achatados na ordem que o processador espera.
    fn params(&self, effect: &Effect) -> Vec<f32>;
    /// Cria o processador já preparado para a taxa.
    fn create(&self, effect: &Effect, sample_rate: u32) -> Box<dyn AudioProcessor>;
}

/// Nota MIDI temporizada em ticks (resultado de harmonia, bases geradas ou partitura).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiNoteEvent {
    pub start_tick: i64,
    pub duration_ticks: i64,
    /// 0–15 (9 = percussão General MIDI).
    pub channel: u8,
    /// Programa General MIDI 0–127.
    pub program: u8,
    pub key: u8,
    pub velocity: u8,
}

/// Sintetizador (SoundFont/General MIDI): transforma notas em áudio na taxa do projeto.
pub trait MidiRenderer: Send + Sync {
    fn render(
        &self,
        events: &[MidiNoteEvent],
        tempo: &shared_kernel::TempoMap,
        sample_rate: u32,
        tail_seconds: f64,
    ) -> Result<SourceBuffer, PortError>;
}

/// Relógio (para carimbos de tempo testáveis).
pub trait Clock: Send + Sync {
    fn now_unix(&self) -> i64;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// Formatos de arquivo de partitura tratados pela infraestrutura (PDF é gerado na UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScoreFileFormat {
    MusicXml,
    Mxl,
    Midi,
}

/// Partitura importada e avisos (PT-BR) do que foi aproximado/descartado.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedScore {
    pub score: ctx_notation::Score,
    pub warnings: Vec<String>,
}

/// Conversão de partituras de/para formatos de arquivo (MusicXML, MXL, MIDI, Guitar Pro 7).
pub trait NotationFormats: Send + Sync {
    /// Detecta o formato pelo conteúdo e importa.
    fn import(&self, bytes: &[u8]) -> Result<ImportedScore, PortError>;
    fn export(
        &self,
        score: &ctx_notation::Score,
        format: ScoreFileFormat,
    ) -> Result<Vec<u8>, PortError>;
}
