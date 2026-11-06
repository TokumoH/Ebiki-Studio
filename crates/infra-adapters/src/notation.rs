//! `NotationFormats` sobre o crate `infra-notation-io` (MusicXML, MXL, MIDI, Guitar Pro 7).

use app_core::ports::{ImportedScore, NotationFormats, PortError, ScoreFileFormat};
use ctx_notation::Score;
use infra_notation_io::midi::{export_midi, MidiExportOptions};
use infra_notation_io::musicxml::{export_musicxml, export_mxl};
use infra_notation_io::IoError;

#[derive(Debug, Default, Clone, Copy)]
pub struct NotationIoFormats;

fn port(e: IoError) -> PortError {
    PortError::new(e.code(), e.to_string())
}

impl NotationFormats for NotationIoFormats {
    fn import(&self, bytes: &[u8]) -> Result<ImportedScore, PortError> {
        let imported = infra_notation_io::import_any(bytes).map_err(port)?;
        Ok(ImportedScore {
            score: imported.score,
            warnings: imported.warnings,
        })
    }

    fn export(&self, score: &Score, format: ScoreFileFormat) -> Result<Vec<u8>, PortError> {
        match format {
            ScoreFileFormat::MusicXml => Ok(export_musicxml(score).into_bytes()),
            ScoreFileFormat::Mxl => export_mxl(score).map_err(port),
            ScoreFileFormat::Midi => {
                export_midi(score, &MidiExportOptions::default()).map_err(port)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_and_reexport_through_the_port() {
        let f = NotationIoFormats;
        let sample = include_bytes!("../../infra-notation-io/tests/data/sample.musicxml");
        let imported = f.import(sample).expect("amostra MusicXML importa");
        assert!(!imported.score.parts.is_empty());
        for format in [
            ScoreFileFormat::MusicXml,
            ScoreFileFormat::Mxl,
            ScoreFileFormat::Midi,
        ] {
            let bytes = f.export(&imported.score, format).unwrap();
            let again = f.import(&bytes).expect("reimporta o que exportou");
            assert_eq!(
                again.score.parts.len(),
                imported.score.parts.len(),
                "{format:?}"
            );
        }
    }

    #[test]
    fn garbage_is_a_typed_error() {
        let err = NotationIoFormats.import(b"lixo").unwrap_err();
        assert!(!err.code.is_empty());
    }
}
