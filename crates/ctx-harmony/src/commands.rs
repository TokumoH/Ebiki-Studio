//! Comandos desfazíveis sobre o agregado [`HarmonyWorkspace`].

use crate::domain::{HarmonyWorkspace, Progression, ProgressionId};
use shared_kernel::command::{Command, CommandError};

type R = Result<(), CommandError>;

fn missing_state() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Cadastra uma progressão do usuário (construída antes para que o id seja estável em refazer).
pub struct AddUserProgression {
    progression: Progression,
    index: usize,
}

impl AddUserProgression {
    pub fn new(progression: Progression, index: usize) -> Self {
        Self { progression, index }
    }
}

impl Command<HarmonyWorkspace> for AddUserProgression {
    fn apply(&mut self, ws: &mut HarmonyWorkspace) -> R {
        self.index = ws.insert(self.progression.clone(), self.index)?;
        Ok(())
    }
    fn revert(&mut self, ws: &mut HarmonyWorkspace) -> R {
        let crate::domain::ProgressionKey::User(id) = self.progression.key else {
            return Err(missing_state());
        };
        ws.remove(id)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Cadastrar progressão"
    }
}

/// Edita (substitui) uma progressão do usuário.
pub struct EditUserProgression {
    updated: Progression,
    previous: Option<Progression>,
}

impl EditUserProgression {
    pub fn new(updated: Progression) -> Self {
        Self {
            updated,
            previous: None,
        }
    }
}

impl Command<HarmonyWorkspace> for EditUserProgression {
    fn apply(&mut self, ws: &mut HarmonyWorkspace) -> R {
        self.previous = Some(ws.replace(self.updated.clone())?);
        Ok(())
    }
    fn revert(&mut self, ws: &mut HarmonyWorkspace) -> R {
        let prev = self.previous.take().ok_or_else(missing_state)?;
        ws.replace(prev)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Editar progressão"
    }
}

/// Remove uma progressão do usuário.
pub struct RemoveUserProgression {
    pub id: ProgressionId,
    removed: Option<(Progression, usize)>,
}

impl RemoveUserProgression {
    pub fn new(id: ProgressionId) -> Self {
        Self { id, removed: None }
    }
}

impl Command<HarmonyWorkspace> for RemoveUserProgression {
    fn apply(&mut self, ws: &mut HarmonyWorkspace) -> R {
        self.removed = Some(ws.remove(self.id)?);
        Ok(())
    }
    fn revert(&mut self, ws: &mut HarmonyWorkspace) -> R {
        let (p, i) = self.removed.take().ok_or_else(missing_state)?;
        ws.insert(p, i)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover progressão"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Mode, ProgressionKey};
    use shared_kernel::command::History;
    use std::collections::BTreeSet;

    fn user(name: &str, steps: &str) -> Progression {
        Progression::new_user(name, Mode::Ionian, BTreeSet::new(), 4, steps).unwrap()
    }

    fn id(p: &Progression) -> ProgressionId {
        match p.key {
            ProgressionKey::User(id) => id,
            ProgressionKey::Builtin(_) => unreachable!(),
        }
    }

    #[test]
    fn add_undo_redo_roundtrip() {
        let mut ws = HarmonyWorkspace::new();
        let mut h = History::new();
        let p = user("Minha", "I vi IV V");
        let before = ws.clone();
        h.execute(
            &mut ws,
            Box::new(AddUserProgression::new(p.clone(), 0)),
            None,
        )
        .unwrap();
        let after = ws.clone();
        assert_eq!(ws.get(id(&p)), Some(&p));
        h.undo(&mut ws).unwrap();
        assert_eq!(ws, before);
        h.redo(&mut ws).unwrap();
        assert_eq!(ws, after);
    }

    #[test]
    fn edit_undo_redo_roundtrip() {
        let p = user("Minha", "I vi IV V");
        let mut ws = HarmonyWorkspace::from_progressions(vec![p.clone()]).unwrap();
        let mut h = History::new();
        let mut edited = p.clone();
        edited.name = "Editada".into();
        edited.steps = Progression::parse_steps("ii7 V7 Imaj7:8", Mode::Ionian, 4).unwrap();
        let before = ws.clone();
        h.execute(
            &mut ws,
            Box::new(EditUserProgression::new(edited.clone())),
            None,
        )
        .unwrap();
        assert_eq!(ws.get(id(&p)), Some(&edited));
        let after = ws.clone();
        h.undo(&mut ws).unwrap();
        assert_eq!(ws, before);
        h.redo(&mut ws).unwrap();
        assert_eq!(ws, after);
    }

    #[test]
    fn remove_undo_restores_position() {
        let a = user("A", "I");
        let b = user("B", "IV");
        let c = user("C", "V");
        let mut ws =
            HarmonyWorkspace::from_progressions(vec![a.clone(), b.clone(), c.clone()]).unwrap();
        let mut h = History::new();
        let before = ws.clone();
        h.execute(&mut ws, Box::new(RemoveUserProgression::new(id(&b))), None)
            .unwrap();
        assert_eq!(ws.user_progressions().len(), 2);
        let after = ws.clone();
        h.undo(&mut ws).unwrap();
        assert_eq!(ws, before);
        h.redo(&mut ws).unwrap();
        assert_eq!(ws, after);
    }

    #[test]
    fn invalid_commands_fail_without_recording() {
        let mut ws = HarmonyWorkspace::new();
        let mut h = History::new();
        let ghost = user("Fantasma", "I");
        let err = h
            .execute(
                &mut ws,
                Box::new(RemoveUserProgression::new(id(&ghost))),
                None,
            )
            .unwrap_err();
        assert_eq!(err.code, "harmony.progression_not_found");
        assert!(h
            .execute(&mut ws, Box::new(EditUserProgression::new(ghost)), None)
            .is_err());
        assert!(!h.can_undo());
        let mut cmd = RemoveUserProgression::new(ProgressionId::new());
        assert_eq!(
            cmd.revert(&mut ws).unwrap_err().code,
            "internal.revert_without_apply"
        );
    }
}
