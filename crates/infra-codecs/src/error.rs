//! Erros do serviço de codecs, com código estável (exposto à UI) e mensagem em PT-BR.

use shared_kernel::command::CommandError;
use std::io;
use std::path::Path;

/// Erro de probe, decodificação, importação, picos ou exportação.
///
/// Nenhuma função pública deste crate entra em pânico com entrada malformada: tudo vira
/// uma variante deste enum.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    #[error("formato de áudio não suportado: {0}")]
    UnsupportedFormat(String),
    #[error("arquivo de áudio corrompido ou ilegível: {0}")]
    Corrupt(String),
    #[error("erro de leitura/gravação: {0}")]
    Io(String),
    #[error("arquivo grande demais ({size} bytes; limite de {limit} bytes)")]
    TooLarge { size: u64, limit: u64 },
    #[error("sem espaço em disco ao gravar {0}")]
    DiskFull(String),
    #[error("permissão negada: {0}")]
    PermissionDenied(String),
    #[error("arquivo não encontrado: {0}")]
    NotFound(String),
    #[error("caminho inválido: {0}")]
    InvalidPath(String),
    #[error("parâmetro inválido: {0}")]
    InvalidParameter(String),
    #[error("falha na conversão de taxa de amostragem: {0}")]
    Resample(String),
    #[error("falha no codificador: {0}")]
    Encoder(String),
}

impl CodecError {
    /// Código estável no formato `codec.<snake_case>`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat(_) => "codec.unsupported_format",
            Self::Corrupt(_) => "codec.corrupt",
            Self::Io(_) => "codec.io",
            Self::TooLarge { .. } => "codec.too_large",
            Self::DiskFull(_) => "codec.disk_full",
            Self::PermissionDenied(_) => "codec.permission_denied",
            Self::NotFound(_) => "codec.not_found",
            Self::InvalidPath(_) => "codec.invalid_path",
            Self::InvalidParameter(_) => "codec.invalid_parameter",
            Self::Resample(_) => "codec.resample",
            Self::Encoder(_) => "codec.encoder",
        }
    }

    /// Converte um erro de E/S em erro de codec, preservando a causa relevante para o usuário
    /// (disco cheio, permissão, inexistente).
    pub fn from_io(err: &io::Error, path: &Path) -> Self {
        let shown = path.display().to_string();
        if is_disk_full(err) {
            return Self::DiskFull(shown);
        }
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound(shown),
            io::ErrorKind::PermissionDenied => Self::PermissionDenied(shown),
            _ => Self::Io(format!("{shown}: {err}")),
        }
    }
}

/// Disco cheio: `ENOSPC` (28) em Linux/macOS; `ERROR_HANDLE_DISK_FULL` (39) e
/// `ERROR_DISK_FULL` (112) no Windows.
fn is_disk_full(err: &io::Error) -> bool {
    match err.raw_os_error() {
        #[cfg(unix)]
        Some(28) => true,
        #[cfg(windows)]
        Some(39) | Some(112) => true,
        _ => false,
    }
}

impl From<CodecError> for CommandError {
    fn from(e: CodecError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_follow_convention() {
        let all = [
            CodecError::UnsupportedFormat("x".into()),
            CodecError::Corrupt("x".into()),
            CodecError::Io("x".into()),
            CodecError::TooLarge { size: 2, limit: 1 },
            CodecError::DiskFull("x".into()),
            CodecError::PermissionDenied("x".into()),
            CodecError::NotFound("x".into()),
            CodecError::InvalidPath("x".into()),
            CodecError::InvalidParameter("x".into()),
            CodecError::Resample("x".into()),
            CodecError::Encoder("x".into()),
        ];
        for e in all {
            let code = e.code();
            assert!(code.starts_with("codec."));
            assert!(code[6..]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_'));
            let ce: CommandError = e.clone().into();
            assert_eq!(ce.code, code);
            assert_eq!(ce.message, e.to_string());
        }
    }

    #[test]
    fn io_errors_are_mapped_by_kind() {
        let p = Path::new("a.wav");
        let nf = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(CodecError::from_io(&nf, p).code(), "codec.not_found");
        let pd = io::Error::from(io::ErrorKind::PermissionDenied);
        assert_eq!(
            CodecError::from_io(&pd, p).code(),
            "codec.permission_denied"
        );
        let other = io::Error::other("boom");
        assert_eq!(CodecError::from_io(&other, p).code(), "codec.io");
        #[cfg(unix)]
        {
            let full = io::Error::from_raw_os_error(28);
            assert_eq!(CodecError::from_io(&full, p).code(), "codec.disk_full");
        }
    }

    #[test]
    fn messages_are_portuguese() {
        let e = CodecError::TooLarge { size: 10, limit: 5 };
        assert!(e.to_string().contains("grande demais"));
    }
}
