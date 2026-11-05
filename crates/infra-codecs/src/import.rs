//! Importação de arquivos de áudio para a pasta do projeto.
//!
//! Regras (spec da Fase 0, RF-0.2, e Fase 9):
//! - o arquivo é **copiado** para `audio/imported/<sha256>.<ext>`; o original fica intocado;
//! - o hash é SHA-256 (hex minúsculo) dos bytes originais, calculado durante a cópia (streaming);
//! - mesmo conteúdo ⇒ mesmo nome ⇒ deduplicação (o arquivo existente é reutilizado);
//! - cópia para temporário na mesma pasta + rename atômico; nada é escrito fora de
//!   `project_root/audio/imported` (pastas canonizadas e verificadas);
//! - extensão em lista branca e canonizada; tamanho máximo configurável (padrão 2 GiB);
//! - o conteúdo é validado (probe) **antes** da cópia.

use crate::decode::{probe_with, AudioInfo, DecodeLimits};
use crate::error::CodecError;
use crate::format::{display_name, sanitize_extension};
use crate::fsutil::{ensure_dir_within, persist, temp_in};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

/// Pasta (relativa à raiz do projeto) onde ficam os áudios importados.
pub const IMPORTED_DIR: [&str; 2] = ["audio", "imported"];

/// Opções de importação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportOptions {
    /// Tamanho máximo do arquivo de origem em bytes.
    pub max_bytes: u64,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            max_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

/// Resultado de uma importação.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFile {
    /// SHA-256 dos bytes originais, hex minúsculo (64 caracteres).
    pub content_hash: String,
    /// Caminho relativo à raiz do projeto, sempre com `/`: `audio/imported/<hash>.<ext>`.
    pub rel_path: String,
    /// Nome do arquivo de origem (para exibição).
    pub original_name: String,
    pub info: AudioInfo,
    /// `true` se o conteúdo já existia no projeto e foi reutilizado.
    pub deduplicated: bool,
}

/// Importa `src` para o projeto em `project_root` com as opções padrão.
pub fn import_into_project(src: &Path, project_root: &Path) -> Result<ImportedFile, CodecError> {
    import_into_project_with(src, project_root, &ImportOptions::default())
}

pub fn import_into_project_with(
    src: &Path,
    project_root: &Path,
    options: &ImportOptions,
) -> Result<ImportedFile, CodecError> {
    let meta = fs::metadata(src).map_err(|e| CodecError::from_io(&e, src))?;
    if !meta.is_file() {
        return Err(CodecError::InvalidPath(format!(
            "\"{}\" não é um arquivo",
            src.display()
        )));
    }
    if meta.len() > options.max_bytes {
        return Err(CodecError::TooLarge {
            size: meta.len(),
            limit: options.max_bytes,
        });
    }
    let ext = sanitize_extension(src)?;
    let limits = DecodeLimits {
        max_file_bytes: options.max_bytes,
        ..DecodeLimits::default()
    };
    let info = probe_with(src, &limits)?;

    let dest_dir = ensure_dir_within(project_root, &IMPORTED_DIR)?;
    let mut temp = temp_in(&dest_dir)?;
    let hash = copy_and_hash(src, temp.as_file_mut(), options.max_bytes)?;
    let file_name = format!("{hash}.{ext}");
    let dest = dest_dir.join(&file_name);

    let deduplicated = match fs::symlink_metadata(&dest) {
        Ok(m) if m.is_file() => true,
        Ok(_) => {
            return Err(CodecError::InvalidPath(format!(
                "\"{}\" já existe e não é um arquivo comum",
                dest.display()
            )))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(CodecError::from_io(&e, &dest)),
    };
    if deduplicated {
        drop(temp); // remove o temporário
    } else {
        persist(temp, &dest)?;
    }

    Ok(ImportedFile {
        content_hash: hash,
        rel_path: format!("{}/{}/{file_name}", IMPORTED_DIR[0], IMPORTED_DIR[1]),
        original_name: display_name(src),
        info,
        deduplicated,
    })
}

/// Copia `src` para `dst` em blocos de 1 MiB calculando o SHA-256; respeita `max_bytes` mesmo
/// que o arquivo cresça durante a cópia.
fn copy_and_hash(src: &Path, dst: &mut File, max_bytes: u64) -> Result<String, CodecError> {
    let mut input = File::open(src).map_err(|e| CodecError::from_io(&e, src))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let n = match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(CodecError::from_io(&e, src)),
        };
        total += n as u64;
        if total > max_bytes {
            return Err(CodecError::TooLarge {
                size: total,
                limit: max_bytes,
            });
        }
        hasher.update(&buf[..n]);
        dst.write_all(&buf[..n])
            .map_err(|e| CodecError::from_io(&e, src))?;
    }
    dst.flush().map_err(|e| CodecError::from_io(&e, src))?;
    Ok(to_hex(&hasher.finalize()))
}

/// SHA-256 em streaming de um arquivo (hex minúsculo).
pub fn sha256_file(path: &Path) -> Result<String, CodecError> {
    let mut input = File::open(path).map_err(|e| CodecError::from_io(&e, path))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(CodecError::from_io(&e, path)),
        }
    }
    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[usize::from(b >> 4)] as char);
        s.push(HEX[usize::from(b & 0xf)] as char);
    }
    s
}

/// `true` se `s` tem a forma de um hash de conteúdo (64 hex minúsculos).
pub fn is_content_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_of_known_digest() {
        // SHA-256("abc")
        let d = Sha256::digest(b"abc");
        assert_eq!(
            to_hex(&d),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn content_hash_shape() {
        assert!(is_content_hash(&"a".repeat(64)));
        assert!(!is_content_hash(&"A".repeat(64)));
        assert!(!is_content_hash("../etc"));
        assert!(!is_content_hash(&"a".repeat(63)));
    }
}
