//! Validação comum dos argumentos recebidos pela IPC (compartilhada por todos os módulos de comandos).

use app_core::AppError;
use std::path::PathBuf;

pub(crate) type Res<T> = Result<T, AppError>;

pub(crate) fn bad(msg: &str) -> AppError {
    AppError::new("api.invalid_argument", msg)
}

/// Identificador (UUID) vindo da UI.
pub(crate) fn id<T: std::str::FromStr>(s: &str) -> Res<T> {
    s.parse().map_err(|_| bad("identificador inválido"))
}

/// Caminho absoluto vindo da UI (diálogo nativo). Rejeita relativos e caracteres de controle.
pub(crate) fn user_path(p: &str) -> Res<PathBuf> {
    let path = PathBuf::from(p);
    if p.is_empty() || p.chars().any(char::is_control) || !path.is_absolute() {
        return Err(bad("caminho inválido"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_paths_must_be_absolute_and_clean() {
        assert!(user_path("relativo/x.wav").is_err());
        assert!(user_path("").is_err());
        assert!(user_path("/tmp/a\u{0}b").is_err());
        let abs = if cfg!(windows) {
            "C:\\x\\y.wav"
        } else {
            "/x/y.wav"
        };
        assert!(user_path(abs).is_ok());
    }

    #[test]
    fn ids_must_be_uuids() {
        assert!(id::<uuid::Uuid>("não-é-uuid").is_err());
        assert!(id::<uuid::Uuid>("0196f7b4-0000-7000-8000-000000000001").is_ok());
    }
}
