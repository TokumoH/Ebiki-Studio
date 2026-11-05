//! Portas (interfaces) implementadas pela infraestrutura.

use crate::session::Session;
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
