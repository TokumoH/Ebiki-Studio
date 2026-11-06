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

/// Destino de escrita vindo da UI: além de absoluto, precisa ter uma das extensões do formato.
/// Mesmo com a webview comprometida, uma exportação não consegue sobrescrever arquivos
/// arbitrários do usuário (ex.: `~/.bashrc`, executáveis).
pub(crate) fn output_file(p: &str, extensions: &[&str]) -> Res<PathBuf> {
    let mut path = user_path(p)?;
    let Some(ext) = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
    else {
        // nome digitado sem extensão: completa com a do formato
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        if name.to_string_lossy().starts_with('.') || name.is_empty() {
            return Err(bad("nome de arquivo inválido"));
        }
        name.push(".");
        name.push(extensions[0]);
        path.set_file_name(name);
        return Ok(path);
    };
    if !extensions.contains(&ext.as_str()) {
        return Err(bad(&format!(
            "extensão de arquivo inválida (esperado: {})",
            extensions.join(", ")
        )));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_files_must_carry_the_format_extension() {
        let abs = if cfg!(windows) { "C:\\x\\" } else { "/x/" };
        assert!(output_file(&format!("{abs}mix.WAV"), &["wav"]).is_ok());
        assert!(output_file(&format!("{abs}.bashrc"), &["wav"]).is_err());
        assert_eq!(
            output_file(&format!("{abs}mix"), &["wav"]).unwrap(),
            std::path::PathBuf::from(format!("{abs}mix.wav")),
            "sem extensão: completa"
        );
        assert!(output_file(&format!("{abs}mix.wav.exe"), &["wav"]).is_err());
        assert!(output_file("mix.wav", &["wav"]).is_err(), "relativo");
    }

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
