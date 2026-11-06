//! Atualização do app (ADR-0034). A verificação, o download e a validação da assinatura são do
//! `tauri-plugin-updater` (Pure Fabrication); aqui só decidimos **como** esta instalação pode ser
//! atualizada. Pacotes `.deb`/`.rpm` pertencem ao gerenciador do sistema e não se substituem
//! sozinhos: nesses casos a UI oferece a página de download.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateMode {
    /// Baixa, verifica a assinatura e instala dentro do app.
    InApp,
    /// Só avisa e abre a página da release.
    Manual,
}

/// Modo de atualização para o sistema `os` (valores de `std::env::consts::OS`).
/// No Linux só o AppImage (variável `APPIMAGE` definida pelo runtime) se atualiza sozinho.
pub fn update_mode(os: &str, appimage: Option<&str>) -> UpdateMode {
    match os {
        "windows" | "macos" => UpdateMode::InApp,
        "linux" if appimage.is_some_and(|p| !p.is_empty()) => UpdateMode::InApp,
        _ => UpdateMode::Manual,
    }
}

#[tauri::command]
pub fn app_update_mode() -> UpdateMode {
    let appimage = std::env::var("APPIMAGE").ok();
    update_mode(std::env::consts::OS, appimage.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_and_macos_update_in_app() {
        assert_eq!(update_mode("windows", None), UpdateMode::InApp);
        assert_eq!(update_mode("macos", None), UpdateMode::InApp);
    }

    #[test]
    fn linux_updates_in_app_only_as_appimage() {
        assert_eq!(
            update_mode("linux", Some("/home/u/Ebiki.AppImage")),
            UpdateMode::InApp
        );
        assert_eq!(update_mode("linux", Some("")), UpdateMode::Manual);
        assert_eq!(update_mode("linux", None), UpdateMode::Manual);
    }

    #[test]
    fn unknown_systems_fall_back_to_manual() {
        assert_eq!(update_mode("freebsd", None), UpdateMode::Manual);
    }

    #[test]
    fn serializes_as_camel_case_for_the_ui() {
        assert_eq!(
            serde_json::to_string(&UpdateMode::InApp).unwrap(),
            "\"inApp\""
        );
        assert_eq!(
            serde_json::to_string(&UpdateMode::Manual).unwrap(),
            "\"manual\""
        );
    }
}
