//! Persistência SQLite (Pure Fabrication). Implementa `app_core::ports::SessionRepository`.
//!
//! - Todas as consultas usam parâmetros (`?N`), nunca concatenação de entrada (sem SQL injection).
//! - Salvar = uma transação que regrava o estado completo (atômico: ou tudo, ou nada).
//! - Versão do schema em `PRAGMA user_version`; migrações em ordem ([`migrate`]).

mod effects;
pub mod library;
mod project_repo;
// Fase 4/5: harmonia + bases no library.db.
pub mod harmony_repo;

pub use project_repo::SqliteSessionRepository;

use rusqlite::Connection;
use std::path::Path;

/// Migrações do `project.db`, em ordem. O índice + 1 é a versão.
const PROJECT_MIGRATIONS: &[&str] = &[include_str!("../migrations/project_v1.sql")];

pub const PROJECT_SCHEMA_VERSION: u32 = PROJECT_MIGRATIONS.len() as u32;

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("erro no banco de dados: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("o projeto foi criado por uma versão mais nova do AppMusica (schema {found}, suportado {supported})")]
    TooNew { found: u32, supported: u32 },
    #[error("dados corrompidos no projeto: {0}")]
    Corrupt(String),
}

impl PersistenceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sql(e) if is_disk_full(e) => "persistence.disk_full",
            Self::Sql(_) => "persistence.sql",
            Self::TooNew { .. } => "persistence.too_new",
            Self::Corrupt(_) => "persistence.corrupt",
        }
    }
}

fn is_disk_full(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::DiskFull)
}

impl From<PersistenceError> for app_core::ports::PortError {
    fn from(e: PersistenceError) -> Self {
        let msg = if e.code() == "persistence.disk_full" {
            "Disco cheio: libere espaço e tente salvar novamente.".to_owned()
        } else {
            e.to_string()
        };
        app_core::ports::PortError::new(e.code(), msg)
    }
}

/// Abre (criando se preciso) com configurações seguras.
pub fn open(path: &Path) -> Result<Connection, PersistenceError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(conn)
}

/// Aplica migrações pendentes numa transação por versão.
pub fn migrate(conn: &mut Connection, migrations: &[&str]) -> Result<u32, PersistenceError> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let supported = migrations.len() as u32;
    if current > supported {
        return Err(PersistenceError::TooNew {
            found: current,
            supported,
        });
    }
    for (i, sql) in migrations.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as u32)?;
        tx.commit()?;
    }
    Ok(supported)
}

pub fn open_project_db(path: &Path) -> Result<Connection, PersistenceError> {
    let mut c = open(path)?;
    migrate(&mut c, PROJECT_MIGRATIONS)?;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent_and_versioned() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("p.db");
        let mut c = open(&p).unwrap();
        assert_eq!(
            migrate(&mut c, PROJECT_MIGRATIONS).unwrap(),
            PROJECT_SCHEMA_VERSION
        );
        assert_eq!(
            migrate(&mut c, PROJECT_MIGRATIONS).unwrap(),
            PROJECT_SCHEMA_VERSION
        );
        let v: u32 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, PROJECT_SCHEMA_VERSION);
    }

    #[test]
    fn newer_schema_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = open(&dir.path().join("p.db")).unwrap();
        c.pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(
            migrate(&mut c, PROJECT_MIGRATIONS),
            Err(PersistenceError::TooNew { found: 99, .. })
        ));
    }
}
