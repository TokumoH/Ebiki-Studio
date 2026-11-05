//! Agregado [`HarmonyWorkspace`]: progressões cadastradas pelo usuário (RF-5.2).
//!
//! As progressões embutidas são imutáveis e vivem em [`crate::builtin_catalog`]; aqui ficam só as do
//! usuário, que passam pelos comandos desfazíveis.

use super::error::HarmonyError;
use super::progression::{Progression, ProgressionId, ProgressionKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarmonyWorkspace {
    progressions: Vec<Progression>,
}

fn user_id(p: &Progression) -> Result<ProgressionId, HarmonyError> {
    match p.key {
        ProgressionKey::User(id) => Ok(id),
        ProgressionKey::Builtin(_) => Err(HarmonyError::NotUserProgression),
    }
}

impl HarmonyWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstrói a partir de dados persistidos (valida tudo).
    pub fn from_progressions(list: Vec<Progression>) -> Result<Self, HarmonyError> {
        let mut ws = Self::new();
        for p in list {
            let at = ws.progressions.len();
            ws.insert(p, at)?;
        }
        Ok(ws)
    }

    pub fn user_progressions(&self) -> &[Progression] {
        &self.progressions
    }

    pub fn get(&self, id: ProgressionId) -> Option<&Progression> {
        self.progressions
            .iter()
            .find(|p| p.key == ProgressionKey::User(id))
    }

    fn index_of(&self, id: ProgressionId) -> Result<usize, HarmonyError> {
        self.progressions
            .iter()
            .position(|p| p.key == ProgressionKey::User(id))
            .ok_or(HarmonyError::ProgressionNotFound)
    }

    fn check_name(
        &self,
        p: &Progression,
        except: Option<ProgressionId>,
    ) -> Result<(), HarmonyError> {
        let lower = p.name.to_lowercase();
        let clash = self.progressions.iter().any(|o| {
            o.name.to_lowercase() == lower
                && Some(&o.key) != except.map(ProgressionKey::User).as_ref()
        });
        if clash {
            return Err(HarmonyError::DuplicateName);
        }
        Ok(())
    }

    /// Insere na posição (limitada ao fim). Retorna o índice efetivo.
    pub fn insert(&mut self, p: Progression, index: usize) -> Result<usize, HarmonyError> {
        let id = user_id(&p)?;
        p.validate()?;
        if self.get(id).is_some() {
            return Err(HarmonyError::DuplicateId);
        }
        self.check_name(&p, None)?;
        let at = index.min(self.progressions.len());
        self.progressions.insert(at, p);
        Ok(at)
    }

    /// Remove e retorna a progressão e o índice em que estava.
    pub fn remove(&mut self, id: ProgressionId) -> Result<(Progression, usize), HarmonyError> {
        let i = self.index_of(id)?;
        Ok((self.progressions.remove(i), i))
    }

    /// Substitui a progressão de mesmo id. Retorna a versão anterior.
    pub fn replace(&mut self, updated: Progression) -> Result<Progression, HarmonyError> {
        let id = user_id(&updated)?;
        updated.validate()?;
        let i = self.index_of(id)?;
        self.check_name(&updated, Some(id))?;
        Ok(std::mem::replace(&mut self.progressions[i], updated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::catalog::builtin_catalog;
    use crate::domain::scale::Mode;
    use std::collections::BTreeSet;

    fn user(name: &str) -> Progression {
        Progression::new_user(name, Mode::Ionian, BTreeSet::new(), 4, "I IV V I").unwrap()
    }

    #[test]
    fn insert_get_remove() {
        let mut ws = HarmonyWorkspace::new();
        let a = user("A");
        let ProgressionKey::User(id) = a.key else {
            unreachable!()
        };
        assert_eq!(ws.insert(a.clone(), 99).unwrap(), 0);
        assert_eq!(ws.get(id), Some(&a));
        assert_eq!(ws.insert(a.clone(), 0), Err(HarmonyError::DuplicateId));
        assert_eq!(ws.insert(user("a"), 0), Err(HarmonyError::DuplicateName));
        assert_eq!(ws.remove(id).unwrap(), (a, 0));
        assert_eq!(ws.remove(id), Err(HarmonyError::ProgressionNotFound));
    }

    #[test]
    fn rejects_builtin_and_invalid() {
        let mut ws = HarmonyWorkspace::new();
        assert_eq!(
            ws.insert(builtin_catalog()[0].clone(), 0),
            Err(HarmonyError::NotUserProgression)
        );
        let mut bad = user("B");
        bad.steps.clear();
        assert_eq!(ws.insert(bad, 0), Err(HarmonyError::EmptyProgression));
    }

    #[test]
    fn replace_keeps_position_and_checks_names() {
        let a = user("A");
        let b = user("B");
        let mut ws = HarmonyWorkspace::from_progressions(vec![a.clone(), b.clone()]).unwrap();
        let mut a2 = a.clone();
        a2.name = "A2".into();
        assert_eq!(ws.replace(a2.clone()).unwrap(), a);
        assert_eq!(ws.user_progressions()[0], a2);
        let mut clash = b.clone();
        clash.name = "a2".into();
        assert_eq!(ws.replace(clash), Err(HarmonyError::DuplicateName));
        let mut same = a2.clone();
        same.name = "A2".into();
        assert!(
            ws.replace(same).is_ok(),
            "manter o próprio nome é permitido"
        );
        assert_eq!(
            ws.replace(user("C")),
            Err(HarmonyError::ProgressionNotFound)
        );
    }
}
