//! `MediaPort` sobre o crate `infra-codecs`.

use app_core::ports::{
    ExportFormatKind, ExportSettings, ImportedMedia, MediaPort, PortError, SpectrogramRequest,
};
use audio_engine::SourceBuffer;
use infra_codecs::{
    CodecError, EncodeOptions, Encoder, ExportFormat, Mp3Mode, SampleDepth, SpectrogramParams,
};
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

    fn spectrogram(
        &self,
        audio: &SourceBuffer,
        sample_rate: u32,
        req: SpectrogramRequest,
        cache: Option<(&Path, &str)>,
    ) -> Result<Vec<u8>, PortError> {
        let params = SpectrogramParams {
            fft_size: req.fft_size,
            hop: req.hop,
            bins: req.bins,
        };
        let s = match cache {
            Some((root, hash)) => infra_codecs::load_or_compute_spectrogram(
                root,
                hash,
                sample_rate,
                params,
                &audio.channels,
            ),
            None => infra_codecs::compute_spectrogram(&audio.channels, sample_rate, params),
        }
        .map_err(port)?;
        Ok(s.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrogram_is_computed_validated_and_cached() {
        let audio = SourceBuffer {
            channels: vec![(0..8000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect()],
        };
        let req = SpectrogramRequest::default();
        let bytes = CodecsMedia.spectrogram(&audio, 48_000, req, None).unwrap();
        assert_eq!(&bytes[0..4], b"AMSG");
        let parsed = infra_codecs::Spectrogram::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.columns(), 8000usize.div_ceil(512));
        let dir = tempfile::tempdir().unwrap();
        let hash = "d".repeat(64);
        let cached = CodecsMedia
            .spectrogram(&audio, 48_000, req, Some((dir.path(), &hash)))
            .unwrap();
        assert_eq!(cached, bytes);
        assert!(dir.path().join("cache/spectrogram").is_dir());
        let bad = SpectrogramRequest {
            fft_size: 1000,
            ..req
        };
        assert_eq!(
            CodecsMedia
                .spectrogram(&audio, 48_000, bad, None)
                .unwrap_err()
                .code,
            "codec.invalid_parameter"
        );
    }
}
