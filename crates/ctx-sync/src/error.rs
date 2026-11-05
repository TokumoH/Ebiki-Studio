//! Erros do contexto Sync.

use shared_kernel::command::CommandError;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SyncError {
    #[error("texto vazio")]
    EmptyText,
    #[error("posição de letra não encontrada")]
    NotFound,
    #[error("intervalo inválido: o início deve ser ≥ 0 e menor que o fim")]
    InvalidSpan,
    #[error("intervalo curto demais para dividir entre {0} sílabas")]
    SpanTooShort(usize),
    #[error("tempos fora de ordem ou sobrepostos na letra")]
    Overlap,
    #[error("erro de leitura na linha {line}: {reason}")]
    Parse { line: usize, reason: String },
    #[error("BPM inválido: {0}")]
    InvalidBpm(f64),
    #[error("marcadores de warp devem ser estritamente crescentes")]
    NonMonotonicMarkers,
    #[error("marcador de warp não encontrado")]
    MarkerNotFound,
    #[error("taxa de amostragem inválida")]
    InvalidSampleRate,
    #[error("a sessão de sincronização não aceita esta ação no estado atual")]
    InvalidSessionState,
    #[error("toque fora de ordem: cada toque deve ser posterior ao anterior")]
    TapOutOfOrder,
    #[error("não há trechos para sincronizar")]
    NothingToSync,
}

impl SyncError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyText => "sync.empty_text",
            Self::NotFound => "sync.not_found",
            Self::InvalidSpan => "sync.invalid_span",
            Self::SpanTooShort(_) => "sync.span_too_short",
            Self::Overlap => "sync.overlap",
            Self::Parse { .. } => "sync.parse_error",
            Self::InvalidBpm(_) => "sync.invalid_bpm",
            Self::NonMonotonicMarkers => "sync.non_monotonic_markers",
            Self::MarkerNotFound => "sync.marker_not_found",
            Self::InvalidSampleRate => "sync.invalid_sample_rate",
            Self::InvalidSessionState => "sync.invalid_session_state",
            Self::TapOutOfOrder => "sync.tap_out_of_order",
            Self::NothingToSync => "sync.nothing_to_sync",
        }
    }
}

impl From<SyncError> for CommandError {
    fn from(e: SyncError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_follow_convention() {
        let e: CommandError = SyncError::Parse {
            line: 3,
            reason: "tempo inválido".into(),
        }
        .into();
        assert_eq!(e.code, "sync.parse_error");
        assert_eq!(e.message, "erro de leitura na linha 3: tempo inválido");
        assert!(SyncError::Overlap.code().starts_with("sync."));
    }
}
