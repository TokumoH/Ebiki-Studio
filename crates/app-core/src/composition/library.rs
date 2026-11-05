//! Biblioteca de composição do usuário: agregado [`Library`] (progressões do usuário + catálogo de
//! bases), porta de persistência [`LibraryStore`] e o serviço [`Composer`] com histórico próprio.
//!
//! **Desfazer:** a biblioteca é global (não pertence a um projeto), por isso tem um histórico
//! separado do `Studio` (`library_undo`/`library_redo` na IPC). Cada alteração é gravada logo
//! após aplicada; se a gravação falhar, a alteração é desfeita e o erro volta para a UI.

use crate::ports::PortError;
use crate::studio::{AppError, AppResult};
use ctx_harmony::{HarmonyWorkspace, Progression, RuleBasedSuggester, SuggestionProvider};
use ctx_library::BaseCatalog;
use serde::Serialize;
use shared_kernel::command::{Command, CommandError, History};
use std::path::PathBuf;
use std::sync::Mutex;

/// Estado persistido da biblioteca do usuário.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    pub harmony: HarmonyWorkspace,
    pub bases: BaseCatalog,
}

/// Porta de persistência da biblioteca (implementada pelo `library.db`).
pub trait LibraryStore: Send {
    /// Progressões do usuário e todas as bases (incluídas, geradas e do usuário).
    fn load(&self) -> Result<Library, PortError>;
    /// Regrava os dados do usuário (as bases CC0 incluídas pertencem à semeadura).
    fn save(&self, library: &Library) -> Result<(), PortError>;
}

/// Armazenamento em memória (fallback quando o `library.db` não abre; testes).
#[derive(Debug, Default)]
pub struct MemoryLibraryStore {
    data: Mutex<Library>,
    fail_saves: std::sync::atomic::AtomicBool,
}

impl MemoryLibraryStore {
    pub fn new(initial: Library) -> Self {
        Self {
            data: Mutex::new(initial),
            fail_saves: Default::default(),
        }
    }

    /// Simula falha de gravação (testes de consistência).
    pub fn set_fail_saves(&self, fail: bool) {
        self.fail_saves
            .store(fail, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Library {
        self.data.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl LibraryStore for MemoryLibraryStore {
    fn load(&self) -> Result<Library, PortError> {
        Ok(self.snapshot())
    }
    fn save(&self, library: &Library) -> Result<(), PortError> {
        if self.fail_saves.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(PortError::new("persistence.sql", "falha simulada"));
        }
        *self.data.lock().unwrap_or_else(|e| e.into_inner()) = library.clone();
        Ok(())
    }
}

impl<T: LibraryStore + Sync> LibraryStore for std::sync::Arc<T> {
    fn load(&self) -> Result<Library, PortError> {
        (**self).load()
    }
    fn save(&self, library: &Library) -> Result<(), PortError> {
        (**self).save(library)
    }
}

/// Adapta comandos do `HarmonyWorkspace` para a `Library`.
pub(crate) struct OnHarmony(pub Box<dyn Command<HarmonyWorkspace>>);

impl Command<Library> for OnHarmony {
    fn apply(&mut self, l: &mut Library) -> Result<(), CommandError> {
        self.0.apply(&mut l.harmony)
    }
    fn revert(&mut self, l: &mut Library) -> Result<(), CommandError> {
        self.0.revert(&mut l.harmony)
    }
    fn label(&self) -> &str {
        self.0.label()
    }
}

/// Adapta comandos do `BaseCatalog` para a `Library`.
pub(crate) struct OnBases(pub Box<dyn Command<BaseCatalog>>);

impl Command<Library> for OnBases {
    fn apply(&mut self, l: &mut Library) -> Result<(), CommandError> {
        self.0.apply(&mut l.bases)
    }
    fn revert(&mut self, l: &mut Library) -> Result<(), CommandError> {
        self.0.revert(&mut l.bases)
    }
    fn label(&self) -> &str {
        self.0.label()
    }
}

/// Estado do histórico da biblioteca (para a UI).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryHistoryView {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
}

/// Pastas onde ficam os áudios das bases.
///
/// - `user`: `<dados do app>/bases/` — bases de áudio do usuário são **copiadas** para cá na
///   importação (o arquivo original pode sumir sem quebrar a biblioteca).
/// - `bundled`: pasta de recursos do app (`assets/bases/`) com os áudios CC0 do manifesto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseRoots {
    pub user: PathBuf,
    pub bundled: PathBuf,
}

/// Fábrica do provedor de sugestões (troca por IA local sem mudar a aplicação — ADR-0011).
pub type ProviderFactory = fn(&[Progression]) -> Box<dyn SuggestionProvider>;

fn rule_based(user: &[Progression]) -> Box<dyn SuggestionProvider> {
    Box::new(RuleBasedSuggester::with_user(user))
}

/// Serviço de aplicação da biblioteca de composição.
pub struct Composer {
    store: Box<dyn LibraryStore>,
    pub(crate) library: Library,
    history: History<Library>,
    provider: Box<dyn SuggestionProvider>,
    factory: ProviderFactory,
    roots: BaseRoots,
}

impl Composer {
    /// Carrega a biblioteca do armazenamento.
    pub fn open(store: Box<dyn LibraryStore>, roots: BaseRoots) -> AppResult<Self> {
        let library = store.load()?;
        let factory: ProviderFactory = rule_based;
        let provider = factory(library.harmony.user_progressions());
        Ok(Self {
            store,
            library,
            history: History::new(),
            provider,
            factory,
            roots,
        })
    }

    /// Troca a implementação de sugestões (ex.: IA local).
    pub fn with_provider_factory(mut self, factory: ProviderFactory) -> Self {
        self.factory = factory;
        self.refresh();
        self
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn provider(&self) -> &dyn SuggestionProvider {
        self.provider.as_ref()
    }

    pub fn roots(&self) -> &BaseRoots {
        &self.roots
    }

    fn refresh(&mut self) {
        self.provider = (self.factory)(self.library.harmony.user_progressions());
    }

    /// Aplica, grava e (se a gravação falhar) desfaz.
    pub(crate) fn execute(&mut self, cmd: Box<dyn Command<Library>>) -> AppResult<()> {
        self.history.execute(&mut self.library, cmd, None)?;
        if let Err(e) = self.store.save(&self.library) {
            let _ = self.history.undo(&mut self.library);
            self.refresh();
            return Err(AppError::from(e));
        }
        self.refresh();
        Ok(())
    }

    pub fn undo(&mut self) -> AppResult<LibraryHistoryView> {
        if self.history.undo(&mut self.library)?.is_some() {
            if let Err(e) = self.store.save(&self.library) {
                let _ = self.history.redo(&mut self.library);
                self.refresh();
                return Err(AppError::from(e));
            }
        }
        self.refresh();
        Ok(self.history_view())
    }

    pub fn redo(&mut self) -> AppResult<LibraryHistoryView> {
        if self.history.redo(&mut self.library)?.is_some() {
            if let Err(e) = self.store.save(&self.library) {
                let _ = self.history.undo(&mut self.library);
                self.refresh();
                return Err(AppError::from(e));
            }
        }
        self.refresh();
        Ok(self.history_view())
    }

    pub fn history_view(&self) -> LibraryHistoryView {
        LibraryHistoryView {
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            undo_label: self.history.undo_label().map(str::to_owned),
            redo_label: self.history.redo_label().map(str::to_owned),
        }
    }
}
