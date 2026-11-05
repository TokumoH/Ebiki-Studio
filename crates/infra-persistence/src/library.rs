//! `library.db`: dados globais do usuário (projetos recentes, preferências). Migração 2: esquema de
//! harmonia e bases (`ctx_harmony::LIBRARY_SCHEMA_V1`); repositórios em `harmony_repo`.

use crate::{migrate, open, PersistenceError};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const LIBRARY_MIGRATIONS: &[&str] = &[
    "
CREATE TABLE recent_project (
    path      TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    opened_at INTEGER NOT NULL
);
CREATE INDEX recent_by_time ON recent_project (opened_at DESC);
CREATE TABLE setting (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
",
    // v2 — Fase 4/5: harmonia + bases (DDL normalizado do contexto Harmony).
    ctx_harmony::LIBRARY_SCHEMA_V1,
];

pub const MAX_RECENT: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub opened_at: i64,
}

pub struct LibraryDb {
    conn: Connection,
}

impl LibraryDb {
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        let mut conn = open(path)?;
        migrate(&mut conn, LIBRARY_MIGRATIONS)?;
        let db = Self { conn };
        // vocabulário + catálogo embutido (idempotente; só regrava se a versão aumentou)
        db.seed_catalog()?;
        Ok(db)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Registra abertura de projeto (mantém só os [`MAX_RECENT`] mais recentes).
    pub fn touch_recent(&self, path: &str, name: &str, at: i64) -> Result<(), PersistenceError> {
        self.conn.execute(
            "INSERT INTO recent_project (path, name, opened_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name, opened_at = excluded.opened_at",
            params![path, name, at],
        )?;
        self.conn.execute(
            "DELETE FROM recent_project WHERE path NOT IN (SELECT path FROM recent_project ORDER BY opened_at DESC LIMIT ?1)",
            params![MAX_RECENT as i64],
        )?;
        Ok(())
    }

    pub fn remove_recent(&self, path: &str) -> Result<(), PersistenceError> {
        self.conn
            .execute("DELETE FROM recent_project WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn recent(&self) -> Result<Vec<RecentProject>, PersistenceError> {
        let mut st = self
            .conn
            .prepare("SELECT path, name, opened_at FROM recent_project ORDER BY opened_at DESC")?;
        let rows = st.query_map([], |r| {
            Ok(RecentProject {
                path: r.get(0)?,
                name: r.get(1)?,
                opened_at: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        self.conn.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM setting WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_projects_are_capped_and_ordered() {
        let dir = tempfile::tempdir().unwrap();
        let db = LibraryDb::open(&dir.path().join("library.db")).unwrap();
        for i in 0..20 {
            db.touch_recent(&format!("/p/{i}"), &format!("P{i}"), i)
                .unwrap();
        }
        db.touch_recent("/p/3", "Renomeado", 100).unwrap();
        let r = db.recent().unwrap();
        assert_eq!(r.len(), MAX_RECENT);
        assert_eq!(r[0].path, "/p/3");
        assert_eq!(r[0].name, "Renomeado");
    }

    #[test]
    fn settings_are_parameterized_against_injection() {
        let dir = tempfile::tempdir().unwrap();
        let db = LibraryDb::open(&dir.path().join("library.db")).unwrap();
        let evil = "x'); DROP TABLE setting; --";
        db.set_setting(evil, evil).unwrap();
        assert_eq!(db.setting(evil).unwrap().as_deref(), Some(evil));
        db.set_setting("tema", "escuro").unwrap();
        assert_eq!(db.setting("tema").unwrap().as_deref(), Some("escuro"));
    }
}
