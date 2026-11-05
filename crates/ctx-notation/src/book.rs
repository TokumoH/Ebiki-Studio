//! Coleção de partituras de um projeto ([`ScoreBook`]) e comandos sobre ela.
//!
//! Cada partitura recebe um [`ScoreId`] estável (a posição na lista é só a ordem de exibição).
//! [`OnScore`] adapta qualquer `Command<Score>` (ex.: [`crate::commands::EditVoice`]) para a coleção,
//! localizando a partitura pelo id — assim o histórico da aplicação desfaz edições de partituras
//! diferentes sem depender de índices.

use crate::error::NotationError;
use crate::model::Score;
use serde::{Deserialize, Serialize};
use shared_kernel::command::{Command, CommandError};
use shared_kernel::define_id;

define_id!(
    /// Identificador estável de uma partitura dentro do projeto.
    ScoreId
);

/// Máximo de partituras por projeto (proteção contra crescimento sem limite).
pub const MAX_SCORES: usize = 256;

/// Partitura com identidade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreEntry {
    pub id: ScoreId,
    pub score: Score,
}

/// Partituras do projeto, em ordem de exibição.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBook {
    entries: Vec<ScoreEntry>,
}

fn not_found() -> NotationError {
    NotationError::Inconsistent("partitura não encontrada")
}

impl ScoreBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstrói a coleção (ex.: persistência), validando ids únicos e invariantes de cada partitura.
    pub fn from_entries(entries: Vec<ScoreEntry>) -> Result<Self, NotationError> {
        if entries.len() > MAX_SCORES {
            return Err(NotationError::LimitExceeded("partituras"));
        }
        let mut ids: Vec<_> = entries.iter().map(|e| e.id).collect();
        ids.sort();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(NotationError::Inconsistent("partitura repetida"));
        }
        for e in &entries {
            e.score.check_invariants()?;
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[ScoreEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn position(&self, id: ScoreId) -> Result<usize, NotationError> {
        self.entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(not_found)
    }

    pub fn get(&self, id: ScoreId) -> Result<&Score, NotationError> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| &e.score)
            .ok_or_else(not_found)
    }

    pub fn get_mut(&mut self, id: ScoreId) -> Result<&mut Score, NotationError> {
        self.entries
            .iter_mut()
            .find(|e| e.id == id)
            .map(|e| &mut e.score)
            .ok_or_else(not_found)
    }

    /// Insere em `index` (limitado ao fim). A partitura precisa ser válida e o id inédito.
    pub fn insert(&mut self, index: usize, entry: ScoreEntry) -> Result<usize, NotationError> {
        if self.entries.len() >= MAX_SCORES {
            return Err(NotationError::LimitExceeded("partituras"));
        }
        if self.entries.iter().any(|e| e.id == entry.id) {
            return Err(NotationError::Inconsistent("partitura repetida"));
        }
        entry.score.check_invariants()?;
        let i = index.min(self.entries.len());
        self.entries.insert(i, entry);
        Ok(i)
    }

    pub fn remove(&mut self, id: ScoreId) -> Result<(usize, ScoreEntry), NotationError> {
        let i = self.position(id)?;
        Ok((i, self.entries.remove(i)))
    }
}

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Adiciona uma partitura (criação, importação, acordes → partitura).
pub struct AddScore {
    entry: Option<ScoreEntry>,
    index: usize,
    id: ScoreId,
    label: String,
}

impl AddScore {
    pub fn new(id: ScoreId, score: Score, index: usize, label: impl Into<String>) -> Self {
        Self {
            entry: Some(ScoreEntry { id, score }),
            index,
            id,
            label: label.into(),
        }
    }
}

impl Command<ScoreBook> for AddScore {
    fn apply(&mut self, b: &mut ScoreBook) -> Result<(), CommandError> {
        let entry = self.entry.take().ok_or_else(missing)?;
        match b.insert(self.index, entry.clone()) {
            Ok(i) => {
                self.index = i;
                Ok(())
            }
            Err(e) => {
                self.entry = Some(entry);
                Err(e.into())
            }
        }
    }
    fn revert(&mut self, b: &mut ScoreBook) -> Result<(), CommandError> {
        let (_, entry) = b.remove(self.id)?;
        self.entry = Some(entry);
        Ok(())
    }
    fn label(&self) -> &str {
        &self.label
    }
}

/// Remove uma partitura (o desfazer a reinsere na mesma posição).
pub struct RemoveScore {
    pub id: ScoreId,
    removed: Option<(usize, ScoreEntry)>,
}

impl RemoveScore {
    pub fn new(id: ScoreId) -> Self {
        Self { id, removed: None }
    }
}

impl Command<ScoreBook> for RemoveScore {
    fn apply(&mut self, b: &mut ScoreBook) -> Result<(), CommandError> {
        self.removed = Some(b.remove(self.id)?);
        Ok(())
    }
    fn revert(&mut self, b: &mut ScoreBook) -> Result<(), CommandError> {
        let (i, e) = self.removed.take().ok_or_else(missing)?;
        b.insert(i, e)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover partitura"
    }
}

/// Aplica um comando de partitura à partitura `id` da coleção.
pub struct OnScore {
    pub id: ScoreId,
    inner: Box<dyn Command<Score>>,
}

impl OnScore {
    pub fn new(id: ScoreId, inner: impl Command<Score> + 'static) -> Self {
        Self {
            id,
            inner: Box::new(inner),
        }
    }

    pub fn boxed(id: ScoreId, inner: Box<dyn Command<Score>>) -> Self {
        Self { id, inner }
    }
}

impl Command<ScoreBook> for OnScore {
    fn apply(&mut self, b: &mut ScoreBook) -> Result<(), CommandError> {
        self.inner.apply(b.get_mut(self.id)?)
    }
    fn revert(&mut self, b: &mut ScoreBook) -> Result<(), CommandError> {
        self.inner.revert(b.get_mut(self.id)?)
    }
    fn label(&self) -> &str {
        self.inner.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{EditScore, ScoreEdit};
    use crate::model::Meter;
    use shared_kernel::command::History;

    fn score(title: &str) -> Score {
        Score::new(title, 120.0, Meter::new(4, 4).unwrap()).unwrap()
    }

    #[test]
    fn add_remove_and_edit_by_id_roundtrip_through_history() {
        let mut book = ScoreBook::new();
        let mut h = History::new();
        let a = ScoreId::new();
        let b = ScoreId::new();
        h.execute(
            &mut book,
            Box::new(AddScore::new(a, score("A"), 0, "Nova")),
            None,
        )
        .unwrap();
        h.execute(
            &mut book,
            Box::new(AddScore::new(b, score("B"), 0, "Nova")),
            None,
        )
        .unwrap();
        assert_eq!(book.position(b).unwrap(), 0, "inserida no início");
        h.execute(
            &mut book,
            Box::new(OnScore::new(
                a,
                EditScore::new(ScoreEdit::SetHeader {
                    title: "A2".into(),
                    composer: "C".into(),
                }),
            )),
            None,
        )
        .unwrap();
        assert_eq!(book.get(a).unwrap().title, "A2");
        let after = book.clone();
        h.execute(&mut book, Box::new(RemoveScore::new(b)), None)
            .unwrap();
        assert_eq!(book.len(), 1);
        h.undo(&mut book).unwrap();
        assert_eq!(book, after, "remoção desfeita volta à mesma posição");
        h.undo(&mut book).unwrap();
        assert_eq!(book.get(a).unwrap().title, "A");
        h.undo(&mut book).unwrap();
        h.undo(&mut book).unwrap();
        assert!(book.is_empty());
        for _ in 0..4 {
            h.redo(&mut book).unwrap();
        }
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn rejects_duplicates_and_missing_ids() {
        let id = ScoreId::new();
        let e = ScoreEntry {
            id,
            score: score("x"),
        };
        assert!(ScoreBook::from_entries(vec![e.clone(), e.clone()]).is_err());
        let mut book = ScoreBook::from_entries(vec![e.clone()]).unwrap();
        assert!(book.insert(0, e).is_err());
        let mut cmd = OnScore::new(
            ScoreId::new(),
            EditScore::new(ScoreEdit::SetTempo {
                tick: shared_kernel::Tick(0),
                bpm: 90.0,
            }),
        );
        assert!(cmd.apply(&mut book).is_err());
    }

    #[test]
    fn edits_serialize_with_kind_tag() {
        let json = serde_json::to_value(ScoreEdit::SetHeader {
            title: "t".into(),
            composer: "c".into(),
        })
        .unwrap();
        assert_eq!(json["kind"], "setHeader");
        let back: ScoreEdit = serde_json::from_value(json).unwrap();
        assert!(matches!(back, ScoreEdit::SetHeader { .. }));
        let v: crate::commands::VoiceEdit =
            serde_json::from_str(r#"{"kind":"delete","index":2}"#).unwrap();
        assert_eq!(v, crate::commands::VoiceEdit::Delete { index: 2 });
    }
}
