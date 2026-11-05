//! Erros do contexto Notation.

use shared_kernel::command::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NotationError {
    #[error("parte não encontrada: {0}")]
    PartNotFound(usize),
    #[error("compasso não encontrado: {0}")]
    MeasureNotFound(usize),
    #[error("voz não encontrada: {0}")]
    VoiceNotFound(usize),
    #[error("evento não encontrado: {0}")]
    EventNotFound(usize),
    #[error("nota não encontrada no acorde: {0}")]
    NoteNotFound(usize),
    #[error("evento com id {0} não encontrado")]
    EventIdNotFound(u64),
    #[error("limite de vozes por compasso atingido")]
    TooManyVoices,
    #[error("duração inválida: {0}")]
    InvalidDuration(&'static str),
    #[error("altura inválida: {0}")]
    InvalidPitch(&'static str),
    #[error("nota fora da faixa MIDI (0–127): {0}")]
    PitchOutOfRange(i32),
    #[error("armadura inválida: {0} acidentes")]
    InvalidKey(i8),
    #[error("fórmula de compasso inválida: {0}/{1}")]
    InvalidMeter(u8, u8),
    #[error("andamento inválido")]
    InvalidTempo,
    #[error("o evento é uma pausa e não tem notas")]
    NotAChord,
    #[error("a parte não tem a mesma estrutura de compassos da partitura")]
    MeasureMismatch,
    #[error("a partitura precisa ter pelo menos um compasso")]
    LastMeasure,
    #[error("tablatura inválida: {0}")]
    InvalidTab(&'static str),
    #[error("nome inválido")]
    InvalidName,
    #[error("programa MIDI inválido: {0}")]
    InvalidProgram(u8),
    #[error("partitura inconsistente: {0}")]
    Inconsistent(&'static str),
    #[error("limite excedido: {0}")]
    LimitExceeded(&'static str),
}

impl NotationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PartNotFound(_) => "notation.part_not_found",
            Self::MeasureNotFound(_) => "notation.measure_not_found",
            Self::VoiceNotFound(_) => "notation.voice_not_found",
            Self::EventNotFound(_) => "notation.event_not_found",
            Self::NoteNotFound(_) => "notation.note_not_found",
            Self::EventIdNotFound(_) => "notation.event_id_not_found",
            Self::TooManyVoices => "notation.too_many_voices",
            Self::InvalidDuration(_) => "notation.invalid_duration",
            Self::InvalidPitch(_) => "notation.invalid_pitch",
            Self::PitchOutOfRange(_) => "notation.pitch_out_of_range",
            Self::InvalidKey(_) => "notation.invalid_key",
            Self::InvalidMeter(..) => "notation.invalid_meter",
            Self::InvalidTempo => "notation.invalid_tempo",
            Self::NotAChord => "notation.not_a_chord",
            Self::MeasureMismatch => "notation.measure_mismatch",
            Self::LastMeasure => "notation.last_measure",
            Self::InvalidTab(_) => "notation.invalid_tab",
            Self::InvalidName => "notation.invalid_name",
            Self::InvalidProgram(_) => "notation.invalid_program",
            Self::Inconsistent(_) => "notation.inconsistent",
            Self::LimitExceeded(_) => "notation.limit_exceeded",
        }
    }
}

impl From<NotationError> for CommandError {
    fn from(e: NotationError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_command_error_with_stable_code() {
        let e: CommandError = NotationError::TooManyVoices.into();
        assert_eq!(e.code, "notation.too_many_voices");
        assert!(e.message.contains("vozes"));
    }
}
