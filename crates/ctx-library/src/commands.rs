//! Comandos desfazíveis sobre o agregado [`BaseCatalog`].

use crate::domain::{Base, BaseCatalog, BaseId, BaseKindTag, LibraryError};
use shared_kernel::command::{Command, CommandError};

type R = Result<(), CommandError>;

fn missing_state() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Adiciona uma base do usuário ou gerada (bases CC0 entram só pela semeadura).
pub struct AddBase {
    base: Base,
    index: usize,
}

impl AddBase {
    pub fn new(base: Base, index: usize) -> Self {
        Self { base, index }
    }

    pub fn base_id(&self) -> BaseId {
        self.base.id
    }
}

impl Command<BaseCatalog> for AddBase {
    fn apply(&mut self, c: &mut BaseCatalog) -> R {
        if self.base.kind.tag() == BaseKindTag::BundledCc0 {
            return Err(LibraryError::BundledReadOnly.into());
        }
        self.index = c.insert(self.base.clone(), self.index)?;
        Ok(())
    }
    fn revert(&mut self, c: &mut BaseCatalog) -> R {
        c.remove(self.base.id)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Adicionar base"
    }
}

/// Edita metadados (ou a receita) de uma base.
pub struct UpdateBase {
    updated: Base,
    previous: Option<Base>,
}

impl UpdateBase {
    pub fn new(updated: Base) -> Self {
        Self {
            updated,
            previous: None,
        }
    }
}

impl Command<BaseCatalog> for UpdateBase {
    fn apply(&mut self, c: &mut BaseCatalog) -> R {
        self.previous = Some(c.replace(self.updated.clone())?);
        Ok(())
    }
    fn revert(&mut self, c: &mut BaseCatalog) -> R {
        let prev = self.previous.take().ok_or_else(missing_state)?;
        c.replace(prev)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Editar base"
    }
}

/// Remove uma base do usuário ou gerada (o arquivo de áudio fica em disco para refazer funcionar).
pub struct RemoveBase {
    pub id: BaseId,
    removed: Option<(Base, usize)>,
}

impl RemoveBase {
    pub fn new(id: BaseId) -> Self {
        Self { id, removed: None }
    }
}

impl Command<BaseCatalog> for RemoveBase {
    fn apply(&mut self, c: &mut BaseCatalog) -> R {
        self.removed = Some(c.remove(self.id)?);
        Ok(())
    }
    fn revert(&mut self, c: &mut BaseCatalog) -> R {
        let (b, i) = self.removed.take().ok_or_else(missing_state)?;
        c.insert(b, i)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover base"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AudioBase, AudioPath, BaseKind, Meter, Tag};
    use shared_kernel::command::History;

    fn user_base(name: &str, path: &str) -> Base {
        Base::new(
            name,
            BaseKind::UserAudio(AudioBase {
                path: AudioPath::new(path).unwrap(),
                progression: None,
            }),
            100.0,
            None,
            Meter::FOUR_FOUR,
        )
        .unwrap()
    }

    #[test]
    fn add_update_remove_roundtrips() {
        let mut c = BaseCatalog::new();
        let mut h = History::new();
        let a = user_base("A", "a.wav");
        let b = user_base("B", "b.wav");
        let s0 = c.clone();
        h.execute(&mut c, Box::new(AddBase::new(a.clone(), 0)), None)
            .unwrap();
        h.execute(&mut c, Box::new(AddBase::new(b.clone(), 9)), None)
            .unwrap();
        let s1 = c.clone();
        assert_eq!(c.bases().len(), 2);

        let mut a2 = a.clone();
        a2.bpm = 128.0;
        a2.tags.insert(Tag::new("dança").unwrap());
        h.execute(&mut c, Box::new(UpdateBase::new(a2.clone())), None)
            .unwrap();
        let s2 = c.clone();
        assert_eq!(c.get(a.id), Some(&a2));

        h.execute(&mut c, Box::new(RemoveBase::new(a.id)), None)
            .unwrap();
        let s3 = c.clone();
        assert_eq!(c.bases().len(), 1);

        h.undo(&mut c).unwrap();
        assert_eq!(c, s2, "remoção desfeita volta à mesma posição");
        h.undo(&mut c).unwrap();
        assert_eq!(c, s1);
        h.undo(&mut c).unwrap();
        h.undo(&mut c).unwrap();
        assert_eq!(c, s0);
        for _ in 0..4 {
            h.redo(&mut c).unwrap();
        }
        assert_eq!(c, s3);
    }

    #[test]
    fn bundled_bases_cannot_be_added_by_command() {
        let mut c = BaseCatalog::new();
        let mut h = History::new();
        let mut b = user_base("CC0", "cc0/x.wav");
        b.kind = BaseKind::BundledCc0(AudioBase {
            path: AudioPath::new("cc0/x.wav").unwrap(),
            progression: None,
        });
        let err = h
            .execute(&mut c, Box::new(AddBase::new(b, 0)), None)
            .unwrap_err();
        assert_eq!(err.code, "library.bundled_read_only");
        assert!(!h.can_undo());
        let mut r = RemoveBase::new(BaseId::new());
        assert_eq!(
            r.revert(&mut c).unwrap_err().code,
            "internal.revert_without_apply"
        );
    }
}
