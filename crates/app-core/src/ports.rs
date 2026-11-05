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
