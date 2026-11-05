//! Utilitários de arquivo: gravação atômica (temporário + rename) e confinamento de caminhos.

use crate::error::CodecError;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Cria (se preciso) `root/rel_dir`, canoniza e garante que o resultado continua dentro de
/// `root` (mesmo que algum componente seja um link simbólico apontando para fora).
pub(crate) fn ensure_dir_within(root: &Path, rel_dir: &[&str]) -> Result<PathBuf, CodecError> {
    let root = fs::canonicalize(root).map_err(|e| CodecError::from_io(&e, root))?;
    if !root.is_dir() {
        return Err(CodecError::InvalidPath(format!(
            "\"{}\" não é uma pasta",
            root.display()
        )));
    }
    let mut dir = root.clone();
    for part in rel_dir {
        dir.push(part);
    }
    fs::create_dir_all(&dir).map_err(|e| CodecError::from_io(&e, &dir))?;
    let canonical = fs::canonicalize(&dir).map_err(|e| CodecError::from_io(&e, &dir))?;
    if !canonical.starts_with(&root) {
        return Err(CodecError::InvalidPath(format!(
            "\"{}\" aponta para fora da pasta do projeto",
            dir.display()
        )));
    }
    Ok(canonical)
}

/// Cria um arquivo temporário oculto na mesma pasta do destino (para o rename ser atômico).
pub(crate) fn temp_in(dir: &Path) -> Result<NamedTempFile, CodecError> {
    tempfile::Builder::new()
        .prefix(".appmusica-")
        .suffix(".part")
        .tempfile_in(dir)
        .map_err(|e| CodecError::from_io(&e, dir))
}

/// Sincroniza e move o temporário para `dest` (substitui se existir).
pub(crate) fn persist(temp: NamedTempFile, dest: &Path) -> Result<(), CodecError> {
    temp.as_file()
        .sync_all()
        .map_err(|e| CodecError::from_io(&e, dest))?;
    temp.persist(dest)
        .map_err(|e| CodecError::from_io(&e.error, dest))?;
    Ok(())
}
