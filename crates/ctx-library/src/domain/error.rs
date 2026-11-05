//! Erros do contexto Library.

use shared_kernel::command::CommandError;

pub const MAX_NAME_LEN: usize = 64;
pub const MAX_TAG_LEN: usize = 32;
pub const MAX_PROGRESSION_TEXT: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryError {
    #[error("nome inválido: deve ter de 1 a {MAX_NAME_LEN} caracteres visíveis")]
    InvalidName,
    #[error("BPM inválido: use de 20 a 999")]
    InvalidBpm,
    #[error("fórmula de compasso inválida")]
    InvalidMeter,
    #[error("caminho de arquivo inválido: {0}")]
    InvalidPath(String),
    #[error("tag inválida: {0}")]
    InvalidTag(String),
    #[error("código de gênero inválido: {0}")]
    InvalidGenre(String),
    #[error("instrumento desconhecido: {0}")]
    UnknownInstrument(String),
    #[error("texto de progressão inválido (até {MAX_PROGRESSION_TEXT} caracteres)")]
    InvalidProgressionText,
    #[error("base gerada precisa de tonalidade")]
    GeneratedNeedsKey,
    #[error("número de compassos inválido: use de 1 a 256")]
    InvalidBars,
    #[error("o padrão {pattern} é para compasso de {expected} tempos, mas a base tem {actual}")]
    PatternMeterMismatch {
        pattern: &'static str,
        expected: u8,
        actual: u8,
    },
    #[error("informe ao menos um acorde para gerar a base")]
    NoChords,
    #[error("acorde inválido para geração: {0}")]
    InvalidChordSlot(String),
    #[error("base não encontrada")]
    BaseNotFound,
    #[error("já existe uma base com este identificador")]
    DuplicateId,
    #[error("já existe uma base usando este arquivo de áudio")]
    DuplicatePath,
    #[error("bases incluídas no aplicativo não podem ser alteradas nem removidas")]
    BundledReadOnly,
    #[error("o tipo da base não pode mudar")]
    KindChange,
    #[error("filtro de busca inválido: {0}")]
    InvalidQuery(String),
}

impl LibraryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidName => "library.invalid_name",
            Self::InvalidBpm => "library.invalid_bpm",
            Self::InvalidMeter => "library.invalid_meter",
            Self::InvalidPath(_) => "library.invalid_path",
            Self::InvalidTag(_) => "library.invalid_tag",
            Self::InvalidGenre(_) => "library.invalid_genre",
            Self::UnknownInstrument(_) => "library.unknown_instrument",
            Self::InvalidProgressionText => "library.invalid_progression_text",
            Self::GeneratedNeedsKey => "library.generated_needs_key",
            Self::InvalidBars => "library.invalid_bars",
            Self::PatternMeterMismatch { .. } => "library.pattern_meter_mismatch",
            Self::NoChords => "library.no_chords",
            Self::InvalidChordSlot(_) => "library.invalid_chord_slot",
            Self::BaseNotFound => "library.base_not_found",
            Self::DuplicateId => "library.duplicate_id",
            Self::DuplicatePath => "library.duplicate_path",
            Self::BundledReadOnly => "library.bundled_read_only",
            Self::KindChange => "library.kind_change",
            Self::InvalidQuery(_) => "library.invalid_query",
        }
    }
}

impl From<LibraryError> for CommandError {
    fn from(e: LibraryError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

/// Valida e normaliza um nome.
pub fn validate_name(name: &str) -> Result<String, LibraryError> {
    let trimmed = name.trim();
    let len = trimmed.chars().count();
    if len == 0 || len > MAX_NAME_LEN || trimmed.chars().any(char::is_control) {
        return Err(LibraryError::InvalidName);
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_conversion() {
        let e = LibraryError::PatternMeterMismatch {
            pattern: "waltz",
            expected: 3,
            actual: 4,
        };
        assert_eq!(e.code(), "library.pattern_meter_mismatch");
        let c: CommandError = e.into();
        assert!(c.message.contains("3 tempos"));
        assert_eq!(validate_name(" Base ").unwrap(), "Base");
        assert!(validate_name(" ").is_err());
    }
}
