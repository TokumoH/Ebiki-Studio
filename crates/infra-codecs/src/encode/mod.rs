//! Exportação: escritor em fluxo para WAV, AIFF, FLAC, MP3 e OGG Vorbis.
//!
//! Todo arquivo é escrito num temporário oculto na pasta de destino e só é movido para o nome
//! final (rename atômico) em [`Encoder::finish`]. Se o `Encoder` for descartado sem `finish`, o
//! temporário é apagado e nenhum arquivo parcial fica visível.

mod aiff;
mod dither;
mod flac;
#[cfg(feature = "mp3")]
mod mp3;
#[cfg(feature = "vorbis")]
mod vorbis;

use crate::decode::DecodedAudio;
use crate::error::CodecError;
use crate::fsutil::{persist, temp_in};
use dither::{quantize, Tpdf};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[cfg(feature = "mp3")]
pub use mp3::MP3_CBR_KBPS;

/// Formato de exportação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Wav,
    Aiff,
    Flac,
    Mp3,
    OggVorbis,
    /// AAC/M4A: sem codificador viável nesta versão (ADR-0023) → `codec.unsupported_format`.
    Aac,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
            Self::OggVorbis => "ogg",
            Self::Aac => "m4a",
        }
    }

    pub fn is_lossless(self) -> bool {
        matches!(self, Self::Wav | Self::Aiff | Self::Flac)
    }

    /// `true` se este build consegue codificar o formato.
    pub fn is_available(self) -> bool {
        match self {
            Self::Wav | Self::Aiff | Self::Flac => true,
            Self::Mp3 => cfg!(feature = "mp3"),
            Self::OggVorbis => cfg!(feature = "vorbis"),
            Self::Aac => false,
        }
    }
}

/// Resolução de amostra para formatos PCM/FLAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleDepth {
    Int16,
    Int24,
    Float32,
}

impl SampleDepth {
    fn int_bits(self) -> Option<u32> {
        match self {
            Self::Int16 => Some(16),
            Self::Int24 => Some(24),
            Self::Float32 => None,
        }
    }
}

/// Modo do MP3: CBR (128–320 kbps) ou VBR V0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "kbps")]
pub enum Mp3Mode {
    Cbr(u16),
    V0,
}

/// Opções de codificação (os campos que não se aplicam ao formato são ignorados).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodeOptions {
    /// WAV: 16/24/32f; AIFF: 16/24/32f (AIFF-C `fl32`); FLAC: 16/24.
    pub depth: SampleDepth,
    /// Dither TPDF ao reduzir para 16/24 bits (padrão: ligado).
    pub dither: bool,
    /// Semente do dither e do número de série Ogg (saída reprodutível).
    pub seed: u64,
    pub mp3: Mp3Mode,
    /// Qualidade Vorbis em [-0,2; 1,0] (padrão 0,6).
    pub vorbis_quality: f32,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            depth: SampleDepth::Int24,
            dither: true,
            seed: 0x5EED_A11D_1050_0001,
            mp3: Mp3Mode::Cbr(320),
            vorbis_quality: 0.6,
        }
    }
}

enum Backend {
    Wav(hound::WavWriter<BufWriter<File>>),
    Aiff(aiff::AiffWriter<BufWriter<File>>),
    Flac(Box<flac::FlacWriter<BufWriter<File>>>),
    #[cfg(feature = "mp3")]
    Mp3(mp3::Mp3Writer<BufWriter<File>>),
    #[cfg(feature = "vorbis")]
    Vorbis(Box<vorbis::VorbisWriter<BufWriter<File>>>),
}

/// Resultado de uma exportação concluída.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedFile {
    pub path: PathBuf,
    pub frames: u64,
}

/// Codificador em fluxo: `new` → `write` (quantas vezes quiser) → `finish`.
pub struct Encoder {
    backend: Backend,
    temp: NamedTempFile,
    dest: PathBuf,
    channels: usize,
    int_bits: Option<u32>,
    dither: Option<Tpdf>,
    frames: u64,
    interleaved: Vec<i32>,
}

fn invalid(msg: impl Into<String>) -> CodecError {
    CodecError::InvalidParameter(msg.into())
}

fn enc_err(path: &Path) -> impl Fn(String) -> CodecError + '_ {
    move |e| CodecError::Encoder(format!("{}: {e}", path.display()))
}

fn hound_err(path: &Path) -> impl Fn(hound::Error) -> CodecError + '_ {
    move |e| match e {
        hound::Error::IoError(io) => CodecError::from_io(&io, path),
        other => CodecError::Encoder(format!("{}: {other}", path.display())),
    }
}

fn validate(
    format: ExportFormat,
    sample_rate: u32,
    channels: u16,
    options: &EncodeOptions,
) -> Result<(), CodecError> {
    if !(crate::decode::MIN_SAMPLE_RATE..=crate::decode::MAX_SAMPLE_RATE).contains(&sample_rate) {
        return Err(invalid(format!(
            "taxa de amostragem {sample_rate} Hz fora do intervalo suportado"
        )));
    }
    if !(1..=8).contains(&channels) {
        return Err(invalid(format!(
            "número de canais {channels} não suportado (1 a 8)"
        )));
    }
    match format {
        ExportFormat::Aac => Err(CodecError::UnsupportedFormat(
            "exportação AAC/M4A indisponível nesta versão: não há codificador AAC livre e \
             embutível com licença compatível (ADR-0023). Use MP3, OGG, FLAC ou WAV."
                .into(),
        )),
        ExportFormat::Flac if options.depth == SampleDepth::Float32 => Err(invalid(
            "FLAC suporta apenas 16 ou 24 bits; escolha uma dessas resoluções",
        )),
        ExportFormat::Mp3 => {
            if !cfg!(feature = "mp3") {
                return Err(CodecError::UnsupportedFormat(
                    "este build foi compilado sem o codificador MP3".into(),
                ));
            }
            if channels > 2 {
                return Err(invalid("MP3 suporta no máximo 2 canais"));
            }
            #[cfg(feature = "mp3")]
            if !mp3::MP3_RATES.contains(&sample_rate) {
                return Err(invalid(format!(
                    "MP3 não suporta {sample_rate} Hz; use até 48 kHz (44,1 ou 48 kHz recomendados)"
                )));
            }
            #[cfg(feature = "mp3")]
            if let Mp3Mode::Cbr(kbps) = options.mp3 {
                if !mp3::MP3_CBR_KBPS.contains(&kbps) {
                    return Err(invalid(format!(
                        "bitrate MP3 {kbps} kbps inválido; use 128, 160, 192, 224, 256 ou 320"
                    )));
                }
            }
            Ok(())
        }
        ExportFormat::OggVorbis => {
            if !cfg!(feature = "vorbis") {
                return Err(CodecError::UnsupportedFormat(
                    "este build foi compilado sem o codificador Ogg Vorbis".into(),
                ));
            }
            if !options.vorbis_quality.is_finite()
                || !(-0.2..=1.0).contains(&options.vorbis_quality)
            {
                return Err(invalid("qualidade Vorbis deve estar entre -0,2 e 1,0"));
            }
            if sample_rate > 192_000 {
                return Err(invalid("Ogg Vorbis suporta até 192 kHz"));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

impl Encoder {
    /// Prepara a exportação para `path` (o arquivo só aparece em `finish`).
    pub fn new(
        path: &Path,
        format: ExportFormat,
        sample_rate: u32,
        channels: u16,
        options: &EncodeOptions,
    ) -> Result<Self, CodecError> {
        validate(format, sample_rate, channels, options)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| CodecError::InvalidPath(format!("\"{}\" sem nome", path.display())))?;
        let parent = match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let parent = fs::canonicalize(&parent).map_err(|e| CodecError::from_io(&e, &parent))?;
        let dest = parent.join(file_name);
        if dest.is_dir() {
            return Err(CodecError::InvalidPath(format!(
                "\"{}\" é uma pasta",
                dest.display()
            )));
        }
        let temp = temp_in(&parent)?;
        let file = temp
            .as_file()
            .try_clone()
            .map_err(|e| CodecError::from_io(&e, &dest))?;
        let out = BufWriter::with_capacity(256 * 1024, file);
        let ch = usize::from(channels);
        // Formatos com perdas recebem `f32` direto (sem quantização/dither).
        let int_bits = match format {
            ExportFormat::Mp3 | ExportFormat::OggVorbis => None,
            _ => options.depth.int_bits(),
        };
        let backend = match format {
            ExportFormat::Wav => {
                let spec = hound::WavSpec {
                    channels,
                    sample_rate,
                    bits_per_sample: int_bits.map_or(32, |b| b as u16),
                    sample_format: if int_bits.is_some() {
                        hound::SampleFormat::Int
                    } else {
                        hound::SampleFormat::Float
                    },
                };
                Backend::Wav(hound::WavWriter::new(out, spec).map_err(hound_err(&dest))?)
            }
            ExportFormat::Aiff => {
                let bits = int_bits.map_or(32, |b| b as u16);
                Backend::Aiff(
                    aiff::AiffWriter::new(out, sample_rate, channels, bits, int_bits.is_none())
                        .map_err(|e| CodecError::from_io(&e, &dest))?,
                )
            }
            ExportFormat::Flac => {
                let bits = int_bits.unwrap_or(24) as usize;
                Backend::Flac(Box::new(
                    flac::FlacWriter::new(out, sample_rate, ch, bits).map_err(enc_err(&dest))?,
                ))
            }
            #[cfg(feature = "mp3")]
            ExportFormat::Mp3 => Backend::Mp3(
                mp3::Mp3Writer::new(out, sample_rate, ch, options.mp3).map_err(enc_err(&dest))?,
            ),
            #[cfg(feature = "vorbis")]
            ExportFormat::OggVorbis => Backend::Vorbis(Box::new(
                vorbis::VorbisWriter::new(
                    out,
                    sample_rate,
                    ch,
                    options.vorbis_quality,
                    options.seed as i32,
                )
                .map_err(enc_err(&dest))?,
            )),
            #[allow(unreachable_patterns)]
            _ => {
                return Err(CodecError::UnsupportedFormat(format!(
                    "exportação {format:?} indisponível"
                )))
            }
        };
        let dither = match int_bits {
            Some(_) if options.dither => Some(Tpdf::new(options.seed)),
            _ => None,
        };
        Ok(Self {
            backend,
            temp,
            dest,
            channels: ch,
            int_bits,
            dither,
            frames: 0,
            interleaved: Vec::new(),
        })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames_written(&self) -> u64 {
        self.frames
    }

    /// Escreve um bloco não intercalado: um slice por canal, todos do mesmo comprimento.
    pub fn write(&mut self, block: &[&[f32]]) -> Result<(), CodecError> {
        if block.len() != self.channels {
            return Err(invalid(format!(
                "bloco com {} canais; esperado {}",
                block.len(),
                self.channels
            )));
        }
        let frames = block[0].len();
        if block.iter().any(|c| c.len() != frames) {
            return Err(invalid("canais do bloco com comprimentos diferentes"));
        }
        if frames == 0 {
            return Ok(());
        }
        let dest = self.dest.clone();
        if let Some(bits) = self.int_bits {
            self.interleaved.clear();
            self.interleaved.reserve(frames * self.channels);
            for f in 0..frames {
                for c in block {
                    self.interleaved
                        .push(quantize(c[f], bits, self.dither.as_mut()));
                }
            }
        }
        let clean = |x: f32| if x.is_finite() { x } else { 0.0 };
        match &mut self.backend {
            Backend::Wav(w) => {
                if self.int_bits.is_some() {
                    for &s in &self.interleaved {
                        w.write_sample(s).map_err(hound_err(&dest))?;
                    }
                } else {
                    for f in 0..frames {
                        for c in block {
                            w.write_sample(clean(c[f])).map_err(hound_err(&dest))?;
                        }
                    }
                }
            }
            Backend::Aiff(w) => {
                if self.int_bits.is_some() {
                    w.write_int_interleaved(&self.interleaved)
                } else {
                    let inter: Vec<f32> = (0..frames)
                        .flat_map(|f| block.iter().map(move |c| clean(c[f])))
                        .collect();
                    w.write_float_interleaved(&inter)
                }
                .map_err(|e| CodecError::from_io(&e, &dest))?;
            }
            Backend::Flac(w) => w
                .write_interleaved(&self.interleaved)
                .map_err(enc_err(&dest))?,
            #[cfg(feature = "mp3")]
            Backend::Mp3(w) => {
                let cleaned: Vec<Vec<f32>> = block
                    .iter()
                    .map(|c| c.iter().map(|&x| clean(x)).collect())
                    .collect();
                let refs: Vec<&[f32]> = cleaned.iter().map(Vec::as_slice).collect();
                w.write(&refs).map_err(enc_err(&dest))?
            }
            #[cfg(feature = "vorbis")]
            Backend::Vorbis(w) => {
                let cleaned: Vec<Vec<f32>> = block
                    .iter()
                    .map(|c| c.iter().map(|&x| clean(x).clamp(-1.0, 1.0)).collect())
                    .collect();
                let refs: Vec<&[f32]> = cleaned.iter().map(Vec::as_slice).collect();
                w.write(&refs).map_err(enc_err(&dest))?
            }
        }
        self.frames += frames as u64;
        Ok(())
    }

    /// Finaliza o contêiner, sincroniza e move o temporário para o destino.
    pub fn finish(self) -> Result<EncodedFile, CodecError> {
        let dest = self.dest;
        let io = |e: std::io::Error| CodecError::from_io(&e, &dest);
        let flush = |w: BufWriter<File>| w.into_inner().map(drop).map_err(|e| io(e.into_error()));
        match self.backend {
            // `finalize` corrige o cabeçalho e faz flush do BufWriter.
            Backend::Wav(w) => w.finalize().map_err(hound_err(&dest))?,
            Backend::Aiff(w) => flush(w.finish().map_err(io)?)?,
            Backend::Flac(w) => flush(w.finish().map_err(enc_err(&dest))?)?,
            #[cfg(feature = "mp3")]
            Backend::Mp3(w) => flush(w.finish().map_err(enc_err(&dest))?)?,
            #[cfg(feature = "vorbis")]
            Backend::Vorbis(w) => flush(w.finish().map_err(enc_err(&dest))?)?,
        }
        persist(self.temp, &dest)?;
        Ok(EncodedFile {
            path: dest,
            frames: self.frames,
        })
    }
}

/// Um stem a exportar: nome (vira o nome do arquivo, saneado) e áudio.
#[derive(Debug, Clone, Copy)]
pub struct Stem<'a> {
    pub name: &'a str,
    pub audio: &'a DecodedAudio,
}

/// Saneia um nome para uso como arquivo: mantém letras/dígitos (inclusive acentuados), espaço,
/// `-` e `_`; troca o resto por `_`; limita a 80 caracteres; nunca vazio.
pub fn sanitize_file_stem(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let s = s.trim().to_string();
    if !s.chars().any(char::is_alphanumeric) {
        "stem".into()
    } else {
        s
    }
}

/// Exporta cada stem para `out_dir/<nome>.<ext>` com um codificador por stem.
/// Atômico no conjunto: se algum falhar, os já gravados nesta chamada são removidos.
pub fn export_stems(
    out_dir: &Path,
    stems: &[Stem<'_>],
    format: ExportFormat,
    options: &EncodeOptions,
) -> Result<Vec<EncodedFile>, CodecError> {
    const BLOCK: usize = 4096;
    let mut names = std::collections::HashSet::new();
    let mut plan = Vec::with_capacity(stems.len());
    for stem in stems {
        let file = format!("{}.{}", sanitize_file_stem(stem.name), format.extension());
        if !names.insert(file.to_lowercase()) {
            return Err(invalid(format!("stems com nomes repetidos: \"{file}\"")));
        }
        if stem.audio.channels.is_empty()
            || stem
                .audio
                .channels
                .iter()
                .any(|c| c.len() != stem.audio.frames())
        {
            return Err(invalid(format!(
                "stem \"{}\" sem canais válidos",
                stem.name
            )));
        }
        plan.push((out_dir.join(file), stem));
    }
    let mut done: Vec<EncodedFile> = Vec::with_capacity(plan.len());
    let result = (|| {
        for (path, stem) in &plan {
            let channels =
                u16::try_from(stem.audio.channel_count()).map_err(|_| invalid("canais demais"))?;
            let mut enc = Encoder::new(path, format, stem.audio.sample_rate, channels, options)?;
            let frames = stem.audio.frames();
            let mut start = 0;
            while start < frames {
                let end = (start + BLOCK).min(frames);
                let block: Vec<&[f32]> =
                    stem.audio.channels.iter().map(|c| &c[start..end]).collect();
                enc.write(&block)?;
                start = end;
            }
            done.push(enc.finish()?);
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(done),
        Err(e) => {
            for f in &done {
                let _ = fs::remove_file(&f.path);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_names_are_sanitized() {
        assert_eq!(sanitize_file_stem("Voz principal"), "Voz principal");
        assert_eq!(sanitize_file_stem("../../etc/passwd"), "______etc_passwd");
        assert_eq!(sanitize_file_stem("a/b\\c:d*?"), "a_b_c_d__");
        assert_eq!(sanitize_file_stem("..."), "stem");
        assert_eq!(sanitize_file_stem("Violão"), "Violão");
        assert_eq!(sanitize_file_stem(&"x".repeat(200)).len(), 80);
    }

    #[test]
    fn aac_is_reported_as_unsupported() {
        let e = validate(ExportFormat::Aac, 48_000, 2, &EncodeOptions::default()).unwrap_err();
        assert_eq!(e.code(), "codec.unsupported_format");
        assert!(e.to_string().contains("AAC"));
        assert!(!ExportFormat::Aac.is_available());
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        let o = EncodeOptions::default();
        assert!(validate(ExportFormat::Wav, 1000, 2, &o).is_err());
        assert!(validate(ExportFormat::Wav, 48_000, 0, &o).is_err());
        let f32o = EncodeOptions {
            depth: SampleDepth::Float32,
            ..o
        };
        assert!(validate(ExportFormat::Flac, 48_000, 2, &f32o).is_err());
        #[cfg(feature = "mp3")]
        {
            assert!(validate(ExportFormat::Mp3, 96_000, 2, &o).is_err());
            assert!(validate(ExportFormat::Mp3, 48_000, 3, &o).is_err());
            let bad = EncodeOptions {
                mp3: Mp3Mode::Cbr(100),
                ..o
            };
            assert!(validate(ExportFormat::Mp3, 48_000, 2, &bad).is_err());
        }
        let badq = EncodeOptions {
            vorbis_quality: f32::NAN,
            ..o
        };
        assert!(validate(ExportFormat::OggVorbis, 48_000, 2, &badq).is_err());
    }
}
