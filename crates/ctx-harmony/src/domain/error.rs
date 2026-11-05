//! Erros do contexto Harmony.

use shared_kernel::command::CommandError;

/// Tamanho máximo do nome de uma progressão.
pub const MAX_NAME_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HarmonyError {
    #[error("cifra inválida: {0}")]
    InvalidChord(String),
    #[error("grau romano inválido: {0}")]
    InvalidRoman(String),
    #[error("grau inválido: use 1 a 7 com no máximo dois acidentes")]
    InvalidDegree,
    #[error("tonalidade inválida: {0}")]
    InvalidKey(String),
    #[error("modo desconhecido: {0}")]
    UnknownMode(String),
    #[error("gênero desconhecido: {0}")]
    UnknownGenre(String),
    #[error("nome inválido: deve ter de 1 a {MAX_NAME_LEN} caracteres visíveis")]
    InvalidName,
    #[error("duração inválida: cada acorde deve durar de 1 a 64 tempos")]
    InvalidDuration,
    #[error("compasso inválido: de 1 a 16 tempos por compasso")]
    InvalidBeatsPerBar,
    #[error("a progressão precisa ter ao menos um acorde")]
    EmptyProgression,
    #[error("a progressão tem acordes demais (máximo 64)")]
    TooManySteps,
    #[error("passo inválido: {0}")]
    InvalidStep(String),
    #[error("progressão não encontrada")]
    ProgressionNotFound,
    #[error("já existe uma progressão com este identificador")]
    DuplicateId,
    #[error("já existe uma progressão do usuário com este nome")]
    DuplicateName,
    #[error("apenas progressões do usuário podem ser cadastradas ou editadas")]
    NotUserProgression,
    #[error("faixa de voicing inválida: use notas MIDI de 21 a 108 com ao menos uma oitava")]
    InvalidVoicingRange,
}

impl HarmonyError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidChord(_) => "harmony.invalid_chord",
            Self::InvalidRoman(_) => "harmony.invalid_roman",
            Self::InvalidDegree => "harmony.invalid_degree",
            Self::InvalidKey(_) => "harmony.invalid_key",
            Self::UnknownMode(_) => "harmony.unknown_mode",
            Self::UnknownGenre(_) => "harmony.unknown_genre",
            Self::InvalidName => "harmony.invalid_name",
            Self::InvalidDuration => "harmony.invalid_duration",
            Self::InvalidBeatsPerBar => "harmony.invalid_beats_per_bar",
            Self::EmptyProgression => "harmony.empty_progression",
            Self::TooManySteps => "harmony.too_many_steps",
            Self::InvalidStep(_) => "harmony.invalid_step",
            Self::ProgressionNotFound => "harmony.progression_not_found",
            Self::DuplicateId => "harmony.duplicate_id",
            Self::DuplicateName => "harmony.duplicate_name",
            Self::NotUserProgression => "harmony.not_user_progression",
            Self::InvalidVoicingRange => "harmony.invalid_voicing_range",
        }
    }
}

impl From<HarmonyError> for CommandError {
    fn from(e: HarmonyError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

/// Valida e normaliza um nome (sem espaços nas pontas, sem caracteres de controle).
pub fn validate_name(name: &str) -> Result<String, HarmonyError> {
    let trimmed = name.trim();
    let len = trimmed.chars().count();
    if len == 0 || len > MAX_NAME_LEN || trimmed.chars().any(char::is_control) {
        return Err(HarmonyError::InvalidName);
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_namespaced_and_convert_to_command_error() {
        let e = HarmonyError::InvalidChord("X".into());
        assert_eq!(e.code(), "harmony.invalid_chord");
        let c: CommandError = e.into();
        assert_eq!(c.code, "harmony.invalid_chord");
        assert!(c.message.contains("cifra inválida"));
    }

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(validate_name("  Minha  ").unwrap(), "Minha");
        assert_eq!(validate_name(""), Err(HarmonyError::InvalidName));
        assert_eq!(validate_name("a\u{7}"), Err(HarmonyError::InvalidName));
        assert!(validate_name(&"x".repeat(64)).is_ok());
        assert!(validate_name(&"x".repeat(65)).is_err());
    }
}
