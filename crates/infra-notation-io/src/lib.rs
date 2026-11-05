//! I/O de notação (Pure Fabrication): converte o agregado `Score` (ctx-notation) de/para formatos
//! de arquivo. Nenhuma regra de domínio aqui — só mapeamento, validação de entrada e limites.
//!
//! - [`midi`]: Standard MIDI File (formato 0/1) com quantização configurável na importação.
//! - [`musicxml`]: MusicXML 4.0 partwise, `.musicxml` e `.mxl`.
//! - [`gp`]: Guitar Pro 7+ (`.gp` = zip com `Content/score.gpif`). GP3–GP6 são convertidos para
//!   GP7 na UI pelo alphaTab (ADR-0020).
//!
//! Segurança: limites de tamanho (arquivo, entrada descompactada, nº de entradas, profundidade e
//! nº de elementos XML, nº de notas); nenhuma entrada malformada causa pânico (testes de fuzz).

pub mod archive;
pub mod error;
pub mod gp;
pub mod midi;
pub mod musicxml;
pub mod xml;

pub use error::{Imported, IoError};

/// Formato detectado pelo conteúdo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationFormat {
    Midi,
    MusicXml,
    Mxl,
    GuitarPro7,
}

/// Detecta o formato pelos bytes iniciais (e, para zip, pelas entradas).
pub fn detect_format(bytes: &[u8]) -> Option<NotationFormat> {
    if bytes.starts_with(b"MThd") {
        return Some(NotationFormat::Midi);
    }
    if bytes.starts_with(b"PK") {
        let names = archive::Archive::open(bytes).ok()?.names();
        if names.iter().any(|n| n == gp::GPIF_ENTRY) {
            return Some(NotationFormat::GuitarPro7);
        }
        return Some(NotationFormat::Mxl);
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    if head.contains("<score-partwise") || head.contains("<score-timewise") {
        return Some(NotationFormat::MusicXml);
    }
    if head.contains("<GPIF") {
        return Some(NotationFormat::GuitarPro7);
    }
    None
}

/// Importa qualquer formato suportado (detecção automática).
pub fn import_any(bytes: &[u8]) -> Result<Imported, IoError> {
    error::check_size(bytes)?;
    match detect_format(bytes) {
        Some(NotationFormat::Midi) => midi::import_midi(bytes, &midi::MidiImportOptions::default()),
        Some(NotationFormat::MusicXml) => musicxml::import_musicxml(bytes),
        Some(NotationFormat::Mxl) => musicxml::import_mxl(bytes),
        Some(NotationFormat::GuitarPro7) if bytes.starts_with(b"PK") => gp::import_gp(bytes),
        Some(NotationFormat::GuitarPro7) => gp::import_gpif(bytes),
        None => Err(IoError::Unsupported("formato desconhecido".into())),
    }
}
