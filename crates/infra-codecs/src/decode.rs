//! Probe e decodificação para `f32` não intercalado (symphonia), com downmix e conversão de taxa.
//!
//! A decodificação é em fluxo: o arquivo comprimido é lido pacote a pacote (buffer de 64 KiB do
//! `MediaSourceStream`), cada pacote é decodificado, reduzido de canais e reamostrado antes do
//! próximo. Só o resultado final (PCM na taxa do projeto) fica em memória.

use crate::downmix::{mix_into, stereo_gains_discrete, stereo_gains_for_mask};
use crate::error::CodecError;
use crate::format::{display_name, SourceFormat};
use crate::resample::StreamResampler;
use serde::Serialize;
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use symphonia::core::audio::Channels;
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

/// Menor e maior taxa aceitas como destino de conversão.
pub const MIN_SAMPLE_RATE: u32 = 8_000;
pub const MAX_SAMPLE_RATE: u32 = 384_000;

/// Erros de pacote consecutivos tolerados antes de considerar o arquivo corrompido.
const MAX_CONSECUTIVE_ERRORS: u32 = 64;

/// Informações básicas de um arquivo de áudio.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInfo {
    pub format: SourceFormat,
    /// Nome curto do codec (ex.: `"pcm_s16le"`, `"mp3"`, `"flac"`).
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    /// Quadros reproduzíveis (sem atraso/preenchimento do codificador).
    pub frames: u64,
    pub duration: Duration,
}

/// Áudio decodificado: um `Vec<f32>` por canal (1 = mono, 2 = estéreo), mesmo comprimento.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DecodedAudio {
    pub channels: Vec<Vec<f32>>,
    pub sample_rate: u32,
}

impl DecodedAudio {
    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub fn duration(&self) -> Duration {
        if self.sample_rate == 0 {
            return Duration::ZERO;
        }
        Duration::from_secs_f64(self.frames() as f64 / f64::from(self.sample_rate))
    }
}

/// Limites de proteção contra arquivos enormes ou maliciosos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeLimits {
    /// Tamanho máximo do arquivo de origem.
    pub max_file_bytes: u64,
    /// Máximo de quadros decodificados por canal (na taxa de destino).
    pub max_frames: u64,
}

impl Default for DecodeLimits {
    /// 2 GiB de arquivo e 6 h de áudio a 96 kHz.
    fn default() -> Self {
        Self {
            max_file_bytes: 2 * 1024 * 1024 * 1024,
            max_frames: 6 * 3600 * 96_000,
        }
    }
}

struct Opened {
    reader: Box<dyn FormatReader>,
    track_id: u32,
    params: AudioCodecParameters,
    format: SourceFormat,
    frames: Option<u64>,
}

fn map_sym(err: SymError, path: &Path) -> CodecError {
    let name = display_name(path);
    match err {
        SymError::IoError(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            CodecError::Corrupt(format!("\"{name}\" terminou antes do esperado"))
        }
        SymError::IoError(e) => CodecError::from_io(&e, path),
        SymError::Unsupported(what) => {
            CodecError::UnsupportedFormat(format!("\"{name}\" não é um áudio reconhecido ({what})"))
        }
        SymError::LimitError(what) => {
            CodecError::Corrupt(format!("\"{name}\" excede limites internos ({what})"))
        }
        other => CodecError::Corrupt(format!("\"{name}\": {other}")),
    }
}

fn open(path: &Path, limits: &DecodeLimits) -> Result<Opened, CodecError> {
    let meta = std::fs::metadata(path).map_err(|e| CodecError::from_io(&e, path))?;
    if !meta.is_file() {
        return Err(CodecError::InvalidPath(format!(
            "\"{}\" não é um arquivo",
            path.display()
        )));
    }
    if meta.len() > limits.max_file_bytes {
        return Err(CodecError::TooLarge {
            size: meta.len(),
            limit: limits.max_file_bytes,
        });
    }
    if meta.len() == 0 {
        return Err(CodecError::Corrupt(format!(
            "\"{}\" está vazio",
            display_name(path)
        )));
    }
    let file = File::open(path).map_err(|e| CodecError::from_io(&e, path))?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let reader = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| map_sym(e, path))?;
    let format = SourceFormat::from_demuxer_name(reader.format_info().short_name);
    let track = reader.default_track(TrackType::Audio).ok_or_else(|| {
        CodecError::UnsupportedFormat(format!(
            "\"{}\" não contém faixa de áudio",
            display_name(path)
        ))
    })?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .cloned()
        .ok_or_else(|| {
            CodecError::UnsupportedFormat(format!(
                "codec de áudio desconhecido em \"{}\"",
                display_name(path)
            ))
        })?;
    let track_id = track.id;
    let frames = track.num_frames;
    Ok(Opened {
        reader,
        track_id,
        params,
        format,
        frames,
    })
}

/// Lê cabeçalhos e devolve formato, taxa, canais e duração, sem decodificar o áudio quando o
/// contêiner informa o número de quadros (caso contrário, decodifica em fluxo para contar).
pub fn probe(path: &Path) -> Result<AudioInfo, CodecError> {
    probe_with(path, &DecodeLimits::default())
}

pub fn probe_with(path: &Path, limits: &DecodeLimits) -> Result<AudioInfo, CodecError> {
    let mut opened = open(path, limits)?;
    let decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&opened.params, &AudioDecoderOptions::default())
        .map_err(|e| map_sym(e, path))?;
    let codec = decoder.codec_info().short_name.to_string();
    let mut sample_rate = opened.params.sample_rate.unwrap_or(0);
    let mut channels = opened.params.channels.as_ref().map_or(0, Channels::count);
    let frames = match opened.frames {
        Some(f) if sample_rate > 0 && channels > 0 => f,
        _ => {
            // Contagem por decodificação (ex.: MP3 sem cabeçalho Xing, ADTS).
            let mut counter = FrameCounter::default();
            run_decode(&mut opened, path, limits.max_frames, &mut counter)?;
            if sample_rate == 0 {
                sample_rate = counter.sample_rate;
            }
            if channels == 0 {
                channels = counter.channels;
            }
            counter.frames
        }
    };
    if sample_rate == 0 || channels == 0 {
        return Err(CodecError::Corrupt(format!(
            "\"{}\" não informa taxa de amostragem ou canais",
            display_name(path)
        )));
    }
    let channels = u16::try_from(channels).map_err(|_| {
        CodecError::UnsupportedFormat(format!("\"{}\" tem canais demais", display_name(path)))
    })?;
    Ok(AudioInfo {
        format: opened.format,
        codec,
        sample_rate,
        channels,
        frames,
        duration: Duration::from_secs_f64(frames as f64 / f64::from(sample_rate)),
    })
}

/// Decodifica `path` para `f32` não intercalado na taxa `target_sample_rate`.
///
/// Mono continua mono; estéreo continua estéreo; mais de dois canais são reduzidos a estéreo
/// (coeficientes em [`crate::downmix`]). Se a taxa de origem difere, usa sinc de alta qualidade.
pub fn decode_to_f32(path: &Path, target_sample_rate: u32) -> Result<DecodedAudio, CodecError> {
    decode_to_f32_with(path, target_sample_rate, &DecodeLimits::default())
}

pub fn decode_to_f32_with(
    path: &Path,
    target_sample_rate: u32,
    limits: &DecodeLimits,
) -> Result<DecodedAudio, CodecError> {
    if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&target_sample_rate) {
        return Err(CodecError::InvalidParameter(format!(
            "taxa de destino {target_sample_rate} Hz fora do intervalo {MIN_SAMPLE_RATE}–{MAX_SAMPLE_RATE} Hz"
        )));
    }
    let mut opened = open(path, limits)?;
    let mut sink = PcmSink::new(target_sample_rate, limits.max_frames);
    run_decode(&mut opened, path, u64::MAX, &mut sink)?;
    sink.finish(path)
}

/// Recebe os blocos decodificados (planos na taxa de origem).
trait BlockSink {
    fn accept(
        &mut self,
        planes: &[Vec<f32>],
        frames: usize,
        channels: &Channels,
        rate: u32,
    ) -> Result<(), CodecError>;
}

#[derive(Default)]
struct FrameCounter {
    frames: u64,
    sample_rate: u32,
    channels: usize,
}

impl BlockSink for FrameCounter {
    fn accept(
        &mut self,
        _planes: &[Vec<f32>],
        frames: usize,
        channels: &Channels,
        rate: u32,
    ) -> Result<(), CodecError> {
        self.frames += frames as u64;
        self.sample_rate = rate;
        self.channels = channels.count();
        Ok(())
    }
}

struct PcmSink {
    target_rate: u32,
    max_frames: u64,
    out: Vec<Vec<f32>>,
    source_rate: u32,
    resampler: Option<StreamResampler>,
    mixed: Vec<Vec<f32>>,
}

impl PcmSink {
    fn new(target_rate: u32, max_frames: u64) -> Self {
        Self {
            target_rate,
            max_frames,
            out: Vec::new(),
            source_rate: 0,
            resampler: None,
            mixed: Vec::new(),
        }
    }

    fn finish(mut self, path: &Path) -> Result<DecodedAudio, CodecError> {
        if self.out.is_empty() {
            return Err(CodecError::Corrupt(format!(
                "\"{}\" não contém áudio decodificável",
                display_name(path)
            )));
        }
        if let Some(rs) = self.resampler.take() {
            rs.finish(&mut self.out)?;
        }
        self.check_limit()?;
        Ok(DecodedAudio {
            channels: self.out,
            sample_rate: self.target_rate,
        })
    }

    fn check_limit(&self) -> Result<(), CodecError> {
        let frames = self.out.first().map_or(0, Vec::len) as u64;
        if frames > self.max_frames {
            return Err(CodecError::TooLarge {
                size: frames,
                limit: self.max_frames,
            });
        }
        Ok(())
    }
}

impl BlockSink for PcmSink {
    fn accept(
        &mut self,
        planes: &[Vec<f32>],
        frames: usize,
        channels: &Channels,
        rate: u32,
    ) -> Result<(), CodecError> {
        if self.out.is_empty() {
            let n = channels.count().clamp(1, 2);
            self.out = vec![Vec::new(); n];
            self.mixed = vec![Vec::new(); n];
            self.source_rate = rate;
            if rate != self.target_rate {
                self.resampler = Some(StreamResampler::new(rate, self.target_rate, n)?);
            }
        } else if rate != self.source_rate {
            return Err(CodecError::UnsupportedFormat(
                "a taxa de amostragem muda no meio do arquivo".into(),
            ));
        }
        let gains = match channels {
            Channels::Positioned(p) if planes.len() > 2 => stereo_gains_for_mask(p.bits()),
            _ if planes.len() > 2 => stereo_gains_discrete(planes.len()),
            _ => Vec::new(),
        };
        match self.resampler.as_mut() {
            None => mix_into(planes, frames, &gains, &mut self.out),
            Some(rs) => {
                for m in &mut self.mixed {
                    m.clear();
                }
                mix_into(planes, frames, &gains, &mut self.mixed);
                rs.push(&self.mixed, frames, &mut self.out)?;
            }
        }
        self.check_limit()
    }
}

fn run_decode(
    opened: &mut Opened,
    path: &Path,
    max_source_frames: u64,
    sink: &mut dyn BlockSink,
) -> Result<(), CodecError> {
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&opened.params, &AudioDecoderOptions::default())
        .map_err(|e| map_sym(e, path))?;
    let mut scratch: Vec<Vec<f32>> = Vec::new();
    let mut consecutive_errors = 0u32;
    let mut decoded_any = false;
    let mut source_frames = 0u64;
    loop {
        let packet = match opened.reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::DecodeError(_)) => {
                consecutive_errors += 1;
                if consecutive_errors > MAX_CONSECUTIVE_ERRORS {
                    return Err(CodecError::Corrupt(format!(
                        "\"{}\" tem dados corrompidos demais",
                        display_name(path)
                    )));
                }
                continue;
            }
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(map_sym(e, path)),
        };
        if packet.track_id != opened.track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                consecutive_errors = 0;
                let frames = buf.frames();
                if frames == 0 {
                    continue;
                }
                let spec = buf.spec().clone();
                if spec.channels().count() == 0 || spec.rate() == 0 {
                    continue;
                }
                buf.copy_to_vecs_planar::<f32>(&mut scratch);
                decoded_any = true;
                source_frames += frames as u64;
                if source_frames > max_source_frames {
                    return Err(CodecError::TooLarge {
                        size: source_frames,
                        limit: max_source_frames,
                    });
                }
                sink.accept(&scratch, frames, spec.channels(), spec.rate())?;
            }
            Err(SymError::DecodeError(_)) => {
                consecutive_errors += 1;
                if consecutive_errors > MAX_CONSECUTIVE_ERRORS {
                    return Err(CodecError::Corrupt(format!(
                        "\"{}\" tem dados corrompidos demais",
                        display_name(path)
                    )));
                }
            }
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(map_sym(e, path)),
        }
    }
    if !decoded_any {
        return Err(CodecError::Corrupt(format!(
            "\"{}\" não contém áudio decodificável",
            display_name(path)
        )));
    }
    Ok(())
}
