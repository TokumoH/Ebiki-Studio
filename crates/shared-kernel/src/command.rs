//! Contrato de comandos desfazíveis (Command Pattern) e histórico ilimitado.
//!
//! Cada bounded context implementa [`Command`] para o seu agregado. A camada de aplicação
//! adapta os comandos para o alvo da sessão e usa [`History`].

use serde::{Deserialize, Serialize};

/// Erro de comando com código estável (exposto à UI) e mensagem em PT-BR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }
}

/// Operação reversível sobre `T`.
///
/// Contrato: se `apply` retornou `Ok`, então `revert` restaura exatamente o estado anterior
/// e não falha; `apply` após `revert` volta ao mesmo estado de depois do primeiro `apply`.
pub trait Command<T>: Send {
    fn apply(&mut self, target: &mut T) -> Result<(), CommandError>;
    fn revert(&mut self, target: &mut T) -> Result<(), CommandError>;
    fn label(&self) -> &str;
}

/// Comando composto: aplica em ordem e reverte em ordem inversa. Atômico: se um passo falha,
/// os anteriores são revertidos.
pub struct Composite<T> {
    label: String,
    steps: Vec<Box<dyn Command<T>>>,
}

impl<T> Composite<T> {
    pub fn new(label: impl Into<String>, steps: Vec<Box<dyn Command<T>>>) -> Self {
        Self { label: label.into(), steps }
    }
}

impl<T: 'static> Command<T> for Composite<T> {
    fn apply(&mut self, target: &mut T) -> Result<(), CommandError> {
        for i in 0..self.steps.len() {
            if let Err(e) = self.steps[i].apply(target) {
                for j in (0..i).rev() {
                    self.steps[j].revert(target)?;
                }
                return Err(e);
            }
        }
        Ok(())
    }

    fn revert(&mut self, target: &mut T) -> Result<(), CommandError> {
        for step in self.steps.iter_mut().rev() {
            step.revert(target)?;
        }
        Ok(())
    }

    fn label(&self) -> &str {
        &self.label
    }
}

struct Entry<T> {
    revision: u64,
    gesture: Option<u64>,
    label: String,
    commands: Vec<Box<dyn Command<T>>>,
}

/// Histórico de desfazer/refazer ilimitado (em memória, durante a sessão).
pub struct History<T> {
    undo: Vec<Entry<T>>,
    redo: Vec<Entry<T>>,
    next_revision: u64,
    saved_revision: u64,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), next_revision: 1, saved_revision: 0 }
    }
}

impl<T> History<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Aplica e registra. Comandos consecutivos com o mesmo `gesture` formam um único item.
    pub fn execute(
        &mut self,
        target: &mut T,
        mut command: Box<dyn Command<T>>,
        gesture: Option<u64>,
    ) -> Result<(), CommandError> {
        command.apply(target)?;
        self.redo.clear();
        if let (Some(g), Some(last)) = (gesture, self.undo.last_mut()) {
            if last.gesture == Some(g) {
                last.commands.push(command);
                return Ok(());
            }
        }
        let revision = self.next_revision;
        self.next_revision += 1;
        let label = command.label().to_owned();
        self.undo.push(Entry { revision, gesture, label, commands: vec![command] });
        Ok(())
    }

    /// Desfaz o último item. Retorna o rótulo desfeito.
    pub fn undo(&mut self, target: &mut T) -> Result<Option<String>, CommandError> {
        let Some(mut entry) = self.undo.pop() else { return Ok(None) };
        for c in entry.commands.iter_mut().rev() {
            c.revert(target)?;
        }
        let label = entry.label.clone();
        self.redo.push(entry);
        Ok(Some(label))
    }

    /// Refaz o último item desfeito.
    pub fn redo(&mut self, target: &mut T) -> Result<Option<String>, CommandError> {
        let Some(mut entry) = self.redo.pop() else { return Ok(None) };
        for c in entry.commands.iter_mut() {
            c.apply(target)?;
        }
        let label = entry.label.clone();
        entry.gesture = None; // um gesto refeito não deve absorver novos comandos
        self.undo.push(entry);
        Ok(Some(label))
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }

    fn current_revision(&self) -> u64 {
        self.undo.last().map_or(0, |e| e.revision)
    }

    /// Marca o estado atual como salvo.
    pub fn mark_saved(&mut self) {
        self.saved_revision = self.current_revision();
    }

    /// Há alterações desde o último salvamento?
    pub fn is_dirty(&self) -> bool {
        self.current_revision() != self.saved_revision
    }

    /// Encerra o gesto atual (o próximo comando com o mesmo id abrirá novo item).
    pub fn end_gesture(&mut self) {
        if let Some(last) = self.undo.last_mut() {
            last.gesture = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Add(i32);
    impl Command<i32> for Add {
        fn apply(&mut self, t: &mut i32) -> Result<(), CommandError> {
            if *t + self.0 > 100 {
                return Err(CommandError::new("overflow", "limite"));
            }
            *t += self.0;
            Ok(())
        }
        fn revert(&mut self, t: &mut i32) -> Result<(), CommandError> {
            *t -= self.0;
            Ok(())
        }
        fn label(&self) -> &str {
            "somar"
        }
    }

    #[test]
    fn undo_redo_restores_state() {
        let mut v = 0;
        let mut h = History::new();
        h.execute(&mut v, Box::new(Add(5)), None).unwrap();
        h.execute(&mut v, Box::new(Add(3)), None).unwrap();
        assert_eq!(v, 8);
        h.undo(&mut v).unwrap();
        assert_eq!(v, 5);
        h.redo(&mut v).unwrap();
        assert_eq!(v, 8);
        h.undo(&mut v).unwrap();
        h.undo(&mut v).unwrap();
        assert_eq!(v, 0);
        assert_eq!(h.undo(&mut v).unwrap(), None);
    }

    #[test]
    fn new_command_clears_redo() {
        let mut v = 0;
        let mut h = History::new();
        h.execute(&mut v, Box::new(Add(1)), None).unwrap();
        h.undo(&mut v).unwrap();
        h.execute(&mut v, Box::new(Add(2)), None).unwrap();
        assert!(!h.can_redo());
        assert_eq!(v, 2);
    }

    #[test]
    fn gesture_merges_into_single_entry() {
        let mut v = 0;
        let mut h = History::new();
        for _ in 0..10 {
            h.execute(&mut v, Box::new(Add(1)), Some(7)).unwrap();
        }
        h.execute(&mut v, Box::new(Add(1)), Some(8)).unwrap();
        assert_eq!(v, 11);
        h.undo(&mut v).unwrap();
        assert_eq!(v, 10);
        h.undo(&mut v).unwrap();
        assert_eq!(v, 0, "os 10 passos do gesto 7 são um único item");
    }

    #[test]
    fn redone_gesture_does_not_absorb_new_commands() {
        let mut v = 0;
        let mut h = History::new();
        h.execute(&mut v, Box::new(Add(1)), Some(1)).unwrap();
        h.undo(&mut v).unwrap();
        h.redo(&mut v).unwrap();
        h.execute(&mut v, Box::new(Add(1)), Some(1)).unwrap();
        h.undo(&mut v).unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn failed_command_is_not_recorded() {
        let mut v = 99;
        let mut h = History::new();
        assert!(h.execute(&mut v, Box::new(Add(5)), None).is_err());
        assert_eq!(v, 99);
        assert!(!h.can_undo());
    }

    #[test]
    fn composite_is_atomic() {
        let mut v = 90;
        let mut c = Composite::new("dois", vec![Box::new(Add(5)) as Box<dyn Command<i32>>, Box::new(Add(10))]);
        assert!(c.apply(&mut v).is_err());
        assert_eq!(v, 90, "passo 1 revertido quando o passo 2 falha");
    }

    #[test]
    fn dirty_tracking_follows_saved_revision() {
        let mut v = 0;
        let mut h = History::new();
        assert!(!h.is_dirty());
        h.execute(&mut v, Box::new(Add(1)), None).unwrap();
        assert!(h.is_dirty());
        h.mark_saved();
        assert!(!h.is_dirty());
        h.undo(&mut v).unwrap();
        assert!(h.is_dirty());
        h.redo(&mut v).unwrap();
        assert!(!h.is_dirty(), "refazer até o ponto salvo volta a ficar limpo");
    }
}
