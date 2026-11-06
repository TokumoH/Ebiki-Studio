//! `infra-codecs` — serviço técnico (GRASP Pure Fabrication) de formatos de áudio.
//!
//! Depende apenas do `shared-kernel` e de bibliotecas externas; nenhum bounded context depende
//! dele diretamente (a camada de aplicação faz a ligação).
//!
//! - [`decode`]: `probe` e `decode_to_f32` (symphonia) com downmix e conversão de taxa (rubato);
//! - [`import`]: cópia para `audio/imported/<sha256>.<ext>` com deduplicação e gravação atômica;
//! - [`peaks`]: picos de waveform e cache binário `cache/peaks/<hash>-<fpp>.peaks`;
//! - [`encode`]: exportação em fluxo WAV/AIFF/FLAC/MP3/OGG e stems;
//! - [`spectrogram`]: STFT (Hann) com eixo log e cache `cache/spectrogram/` (Fase 3, RF-3.5).
//!
//! Spec: `docs/specs/2026-11-05-fase-9-formatos.md`. ADRs: 0022 (codificadores e licenças),
//! 0023 (AAC).

pub mod decode;
mod downmix;
pub mod encode;
pub mod error;
pub mod format;
mod fsutil;
pub mod import;
pub mod peaks;
pub mod resample;
pub mod spectrogram;

pub use decode::{
    decode_to_f32, decode_to_f32_with, probe, probe_with, AudioInfo, DecodeLimits, DecodedAudio,
};
pub use encode::{
    export_stems, sanitize_file_stem, EncodeOptions, EncodedFile, Encoder, ExportFormat, Mp3Mode,
    SampleDepth, Stem,
};
pub use error::CodecError;
pub use format::{sanitize_extension, SourceFormat, IMPORT_EXTENSIONS};
pub use import::{
    import_into_project, import_into_project_with, sha256_file, ImportOptions, ImportedFile,
};
pub use peaks::{
    compute_multi_resolution, compute_peaks, load_or_compute, load_or_compute_multi,
    peaks_cache_path, ChannelPeaks, Peaks, DEFAULT_RESOLUTIONS,
};
pub use spectrogram::{
    compute_spectrogram, load_or_compute_spectrogram, spectrogram_cache_path, Spectrogram,
    SpectrogramParams,
};
