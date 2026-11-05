//! `MediaPort` sobre o crate `infra-codecs`.

use app_core::ports::{ExportFormatKind, ExportSettings, ImportedMedia, MediaPort, PortError};
use audio_engine::SourceBuffer;
use infra_codecs::{CodecError, EncodeOptions, Encoder, ExportFormat, Mp3Mode, SampleDepth};
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct CodecsMedia;

fn port(e: CodecError) -> PortError {
    PortError::new(e.code(), e.to_string())
}

fn to_codec_format(k: ExportFormatKind) -> ExportFormat {
    match k {
        ExportFormatKind::Wav => ExportFormat::Wav,
        ExportFormatKind::Aiff => ExportFormat::Aiff,
        ExportFormatKind::Flac => ExportFormat::Flac,
        ExportFormatKind::Mp3 => ExportFormat::Mp3,
        ExportFormatKind::Ogg => ExportFormat::OggVorbis,
        ExportFormatKind::Aac => ExportFormat::Aac,
    }
}

impl MediaPort for CodecsMedia {
    fn import(
        &self,
        src: &Path,
        project_root: &Path,
        sample_rate: u32,
    ) -> Result<ImportedMedia, PortError> {
        let f = infra_codecs::import_into_project(src, project_root).map_err(port)?;
        // duração na taxa do projeto (mesma fórmula do reamostrador: ceil(n·to/from))
        let from = f.info.sample_rate.max(1) as u128;
        let frames = ((f.info.frames as u128 * sample_rate as u128).div_ceil(from)) as u64;
        Ok(ImportedMedia {
            content_hash: f.content_hash,
            rel_path: f.rel_path,
            original_name: f.original_name,
            channels: f.info.channels.min(2),
            frames,
        })
    }

    fn decode(&self, file: &Path, sample_rate: u32) -> Result<SourceBuffer, PortError> {
        let d = infra_codecs::decode_to_f32(file, sample_rate).map_err(port)?;
        Ok(SourceBuffer {
            channels: d.channels,
        })
    }

    fn hash_file(&self, file: &Path) -> Result<String, PortError> {
        infra_codecs::sha256_file(file).map_err(port)
    }

    fn encode(
        &self,
        file: &Path,
        s: &ExportSettings,
        sample_rate: u32,
        planes: &[Vec<f32>],
    ) -> Result<(), PortError> {
        let depth = match s.bit_depth {
            16 => SampleDepth::Int16,
            32 => SampleDepth::Float32,
            _ => SampleDepth::Int24,
        };
        let opts = EncodeOptions {
            depth,
            mp3: s.mp3_kbps.map_or(Mp3Mode::V0, Mp3Mode::Cbr),
            vorbis_quality: s.vorbis_quality,
            ..Default::default()
        };
        let mut enc = Encoder::new(
            file,
            to_codec_format(s.format),
            sample_rate,
            planes.len() as u16,
            &opts,
        )
        .map_err(port)?;
        let frames = planes.first().map_or(0, Vec::len);
        const BLOCK: usize = 8192;
        let mut at = 0;
        while at < frames {
            let end = (at + BLOCK).min(frames);
            let block: Vec<&[f32]> = planes.iter().map(|p| &p[at..end]).collect();
            enc.write(&block).map_err(port)?;
            at = end;
        }
        enc.finish().map_err(port)?;
        Ok(())
    }

    fn available_formats(&self) -> Vec<ExportFormatKind> {
        [
            ExportFormatKind::Wav,
            ExportFormatKind::Aiff,
            ExportFormatKind::Flac,
            ExportFormatKind::Mp3,
            ExportFormatKind::Ogg,
            ExportFormatKind::Aac,
        ]
        .into_iter()
        .filter(|k| to_codec_format(*k).is_available())
        .collect()
    }
}
