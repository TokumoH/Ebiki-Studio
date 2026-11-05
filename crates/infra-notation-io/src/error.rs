//! Erros de I/O de notação e limites de segurança.

use ctx_notation::NotationError;

/// Tamanho máximo de arquivo lido (bytes).
pub const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
/// Tamanho máximo descompactado de uma entrada de zip (bytes).
pub const MAX_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// Máximo de entradas num zip (.mxl/.gp).
pub const MAX_ZIP_ENTRIES: usize = 2_000;
/// Profundidade máxima da árvore XML.
pub const MAX_XML_DEPTH: usize = 128;
/// Máximo de elementos XML.
pub const MAX_XML_NODES: usize = 2_000_000;
/// Máximo de notas importadas de um MIDI.
pub const MAX_MIDI_NOTES: usize = 500_000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IoError {
    #[error("arquivo grande demais (limite: {0})")]
    TooLarge(&'static str),
    #[error("arquivo malformado: {0}")]
    Malformed(String),
    #[error("formato não suportado: {0}")]
    Unsupported(String),
    #[error("partitura inválida: {0}")]
    Domain(#[from] NotationError),
}

impl IoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge(_) => "notation_io.too_large",
            Self::Malformed(_) => "notation_io.malformed",
            Self::Unsupported(_) => "notation_io.unsupported",
            Self::Domain(e) => e.code(),
        }
    }

    pub(crate) fn malformed(msg: impl std::fmt::Display) -> Self {
        Self::Malformed(msg.to_string())
    }
}

pub(crate) fn check_size(bytes: &[u8]) -> Result<(), IoError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(IoError::TooLarge("32 MiB"));
    }
    Ok(())
}

/// Resultado de importação com avisos (em PT-BR) sobre o que foi aproximado ou descartado.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub score: ctx_notation::Score,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(IoError::TooLarge("x").code(), "notation_io.too_large");
        assert_eq!(
            IoError::from(NotationError::LastMeasure).code(),
            "notation.last_measure"
        );
        assert!(check_size(&vec![0u8; MAX_FILE_BYTES + 1]).is_err());
    }
}
