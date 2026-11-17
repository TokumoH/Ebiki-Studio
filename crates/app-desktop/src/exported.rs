//! Arquivos exportados nesta execução (Pure Fabrication): "Mostrar na pasta" só abre o gerenciador
//! de arquivos para o que o próprio app gravou — a interface não ganha permissão de revelar
//! caminhos arbitrários do disco.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Quantos caminhos lembrar (os mais antigos saem primeiro).
const CAPACITY: usize = 256;

#[derive(Default)]
pub struct ExportedFiles {
    paths: Mutex<VecDeque<PathBuf>>,
}

/// Forma comparável do caminho: canônica quando o arquivo existe (resolve `..` e links).
fn key(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

impl ExportedFiles {
    pub fn record(&self, path: &Path) {
        let k = key(path);
        let mut q = self.paths.lock().unwrap_or_else(|e| e.into_inner());
        q.retain(|p| p != &k);
        if q.len() == CAPACITY {
            q.pop_front();
        }
        q.push_back(k);
    }

    /// O caminho foi exportado nesta execução e o arquivo ainda existe?
    pub fn allows(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let k = key(path);
        self.paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|p| p == &k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_recorded_existing_files_are_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("mix.wav");
        let b = dir.path().join("outro.wav");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let log = ExportedFiles::default();
        assert!(!log.allows(&a), "nada exportado ainda");
        log.record(&a);
        assert!(log.allows(&a));
        assert!(!log.allows(&b), "arquivo não exportado pelo app");
        // o mesmo arquivo por um caminho com `..` continua reconhecido
        let indirect = dir.path().join("sub").join("..").join("mix.wav");
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        assert!(log.allows(&indirect));
        // apagado depois de exportar: não revela
        std::fs::remove_file(&a).unwrap();
        assert!(!log.allows(&a));
    }

    #[test]
    fn directories_and_missing_paths_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let log = ExportedFiles::default();
        log.record(dir.path());
        assert!(
            !log.allows(dir.path()),
            "pastas nunca: só arquivos exportados"
        );
        assert!(!log.allows(Path::new("/caminho/que/nao/existe.wav")));
    }

    #[test]
    fn keeps_only_the_most_recent() {
        let dir = tempfile::tempdir().unwrap();
        let log = ExportedFiles::default();
        let first = dir.path().join("0.wav");
        std::fs::write(&first, b"x").unwrap();
        log.record(&first);
        for i in 1..=CAPACITY {
            let p = dir.path().join(format!("{i}.wav"));
            std::fs::write(&p, b"x").unwrap();
            log.record(&p);
        }
        assert!(!log.allows(&first), "o mais antigo saiu");
        assert!(log.allows(&dir.path().join(format!("{CAPACITY}.wav"))));
    }
}
