//! Formatos de origem reconhecidos e lista branca de extensões de importação.

use crate::error::CodecError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Contêiner detectado no arquivo de origem (pelo conteúdo, não pela extensão).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceFormat {
    Wav,
    Aiff,
    Flac,
    Mp3,
    Ogg,
    /// AAC em fluxo ADTS (`.aac`).
    Aac,
    /// Contêiner ISO/MP4 (`.m4a`/`.mp4`) com AAC ou ALAC.
    M4a,
    Caf,
    Matroska,
    Other,
}

impl SourceFormat {
    /// Mapeia o nome curto do demuxer do symphonia.
    pub(crate) fn from_demuxer_name(name: &str) -> Self {
        match name {
            "wave" => Self::Wav,
            "aiff" => Self::Aiff,
            "flac" => Self::Flac,
            "mp1" | "mp2" | "mp3" => Self::Mp3,
            "ogg" => Self::Ogg,
            "aac" => Self::Aac,
            "isomp4" => Self::M4a,
            "caf" => Self::Caf,
            "matroska" | "mkv" => Self::Matroska,
            _ => Self::Other,
        }
    }
}

/// Extensões aceitas na importação (minúsculas) e a forma canônica usada no nome do arquivo
/// copiado para `audio/imported/`.
pub const IMPORT_EXTENSIONS: &[(&str, &str)] = &[
    ("wav", "wav"),
    ("wave", "wav"),
    ("aif", "aiff"),
    ("aiff", "aiff"),
    ("aifc", "aifc"),
    ("flac", "flac"),
    ("mp3", "mp3"),
    ("ogg", "ogg"),
    ("oga", "ogg"),
    ("m4a", "m4a"),
    ("mp4", "m4a"),
    ("aac", "aac"),
    ("caf", "caf"),
];

/// Extrai a extensão de `path`, valida contra a lista branca e devolve a forma canônica.
///
/// Rejeita extensões ausentes, não ASCII, com separadores ou fora da lista.
pub fn sanitize_extension(path: &Path) -> Result<&'static str, CodecError> {
    let ext = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        CodecError::UnsupportedFormat(format!(
            "\"{}\" não tem extensão de áudio reconhecida",
            display_name(path)
        ))
    })?;
    if ext.is_empty() || ext.len() > 8 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CodecError::UnsupportedFormat(format!(
            "extensão inválida em \"{}\"",
            display_name(path)
        )));
    }
    let lower = ext.to_ascii_lowercase();
    IMPORT_EXTENSIONS
        .iter()
        .find(|(e, _)| *e == lower)
        .map(|(_, canonical)| *canonical)
        .ok_or_else(|| {
            CodecError::UnsupportedFormat(format!(
                "extensão \".{lower}\" não é aceita (use WAV, AIFF, FLAC, MP3, OGG, AAC ou M4A)"
            ))
        })
}

pub(crate) fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelisted_extensions_are_canonicalized() {
        assert_eq!(sanitize_extension(Path::new("a.WAV")).unwrap(), "wav");
        assert_eq!(sanitize_extension(Path::new("a.wave")).unwrap(), "wav");
        assert_eq!(sanitize_extension(Path::new("a.Aif")).unwrap(), "aiff");
        assert_eq!(sanitize_extension(Path::new("x/y.mp4")).unwrap(), "m4a");
        assert_eq!(sanitize_extension(Path::new("y.oga")).unwrap(), "ogg");
    }

    #[test]
    fn rejects_unknown_or_malformed_extensions() {
        for p in [
            "a.exe",
            "a",
            "a.",
            "a.wav ",
            "a.w-v",
            "a.mp3x",
            "a.verylongext",
        ] {
            let e = sanitize_extension(Path::new(p)).unwrap_err();
            assert_eq!(e.code(), "codec.unsupported_format", "{p}");
        }
    }

    #[test]
    fn demuxer_names_map_to_formats() {
        assert_eq!(SourceFormat::from_demuxer_name("wave"), SourceFormat::Wav);
        assert_eq!(SourceFormat::from_demuxer_name("isomp4"), SourceFormat::M4a);
        assert_eq!(SourceFormat::from_demuxer_name("zzz"), SourceFormat::Other);
    }
}
