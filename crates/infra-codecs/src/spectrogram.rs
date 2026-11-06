//! Espectrograma (RF-3.5): STFT com janela de Hann, magnitude em dB quantizada para `u8` e
//! eixo de frequência logarítmico. Pure Fabrication: só cálculo e cache, sem regra de domínio.
//!
//! Formato binário (cache `cache/spectrogram/<hash>-<fft>-<hop>-<bins>.spec` e resposta do IPC
//! `clip_spectrogram`), little-endian:
//!
//! | Offset | Tamanho | Campo                                                    |
//! |--------|---------|----------------------------------------------------------|
//! | 0      | 4       | magic `AMSG`                                             |
//! | 4      | 2       | versão (`1`)                                             |
//! | 6      | 2       | faixas de frequência (`bins`, 16..=512)                  |
//! | 8      | 4       | tamanho da FFT (potência de 2, 256..=8192)               |
//! | 12     | 4       | salto entre colunas (`hop`, amostras)                    |
//! | 16     | 4       | taxa de amostragem                                       |
//! | 20     | 4       | colunas (`ceil(total_frames / hop)`)                     |
//! | 24     | 8       | quadros totais do áudio                                  |
//! | 32     | 8       | FNV-1a 64 do payload                                     |
//! | 40     | …       | `colunas × bins` bytes, por coluna, do grave para o agudo |
//!
//! Cada byte é `0` (≤ −100 dBFS) a `255` (0 dBFS). Cache inválido é ignorado e recalculado.

use crate::error::CodecError;
use crate::fsutil::{ensure_dir_within, persist, temp_in};
use crate::import::is_content_hash;
use crate::peaks::fnv1a64;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const SPECTROGRAM_MAGIC: [u8; 4] = *b"AMSG";
pub const SPECTROGRAM_VERSION: u16 = 1;
const HEADER_LEN: usize = 40;
/// Pasta (relativa à raiz do projeto) do cache.
pub const SPECTROGRAM_DIR: [&str; 2] = ["cache", "spectrogram"];
/// Faixa dinâmica representada: [−100, 0] dBFS → [0, 255].
pub const FLOOR_DB: f32 = -100.0;
/// Frequência mínima do eixo logarítmico.
pub const MIN_FREQ_HZ: f32 = 20.0;

/// Parâmetros da análise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpectrogramParams {
    pub fft_size: u32,
    pub hop: u32,
    pub bins: u16,
}

impl Default for SpectrogramParams {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop: 512,
            bins: 128,
        }
    }
}

impl SpectrogramParams {
    pub fn validate(&self) -> Result<(), CodecError> {
        let ok = self.fft_size.is_power_of_two()
            && (256..=8192).contains(&self.fft_size)
            && (64..=self.fft_size).contains(&self.hop)
            && (16..=512).contains(&self.bins);
        if ok {
            Ok(())
        } else {
            Err(CodecError::InvalidParameter(
                "espectrograma: FFT potência de 2 em 256..=8192, salto em 64..=FFT, 16..=512 faixas"
                    .into(),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spectrogram {
    pub params: SpectrogramParams,
    pub sample_rate: u32,
    pub total_frames: u64,
    /// `columns × bins`, por coluna (grave → agudo).
    pub data: Vec<u8>,
}

impl Spectrogram {
    pub fn columns(&self) -> usize {
        self.data.len() / usize::from(self.params.bins.max(1))
    }

    /// Valor (0..=255) da faixa `bin` na coluna `col`.
    pub fn at(&self, col: usize, bin: usize) -> u8 {
        self.data[col * usize::from(self.params.bins) + bin]
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.data.len());
        out.extend_from_slice(&SPECTROGRAM_MAGIC);
        out.extend_from_slice(&SPECTROGRAM_VERSION.to_le_bytes());
        out.extend_from_slice(&self.params.bins.to_le_bytes());
        out.extend_from_slice(&self.params.fft_size.to_le_bytes());
        out.extend_from_slice(&self.params.hop.to_le_bytes());
        out.extend_from_slice(&self.sample_rate.to_le_bytes());
        out.extend_from_slice(&(self.columns() as u32).to_le_bytes());
        out.extend_from_slice(&self.total_frames.to_le_bytes());
        out.extend_from_slice(&fnv1a64(&self.data).to_le_bytes());
        out.extend_from_slice(&self.data);
        out
    }

    /// `None` para qualquer inconsistência (magic, versão, parâmetros, tamanho, checksum).
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_LEN || bytes[0..4] != SPECTROGRAM_MAGIC {
            return None;
        }
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap_or([0; 4]));
        let u64_at = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap_or([0; 8]));
        if u16_at(4) != SPECTROGRAM_VERSION {
            return None;
        }
        let params = SpectrogramParams {
            bins: u16_at(6),
            fft_size: u32_at(8),
            hop: u32_at(12),
        };
        params.validate().ok()?;
        let sample_rate = u32_at(16);
        let columns = u64::from(u32_at(20));
        let total_frames = u64_at(24);
        if sample_rate == 0 || columns != total_frames.div_ceil(u64::from(params.hop)) {
            return None;
        }
        let payload = &bytes[HEADER_LEN..];
        if payload.len() as u64 != columns.checked_mul(u64::from(params.bins))?
            || fnv1a64(payload) != u64_at(32)
        {
            return None;
        }
        Some(Self {
            params,
            sample_rate,
            total_frames,
            data: payload.to_vec(),
        })
    }
}

/// Bordas (em índices de FFT) das faixas logarítmicas: `bins + 1` valores crescentes.
fn log_band_edges(params: &SpectrogramParams, sample_rate: u32) -> Vec<(usize, usize)> {
    let half = (params.fft_size / 2) as usize;
    let hz_per_bin = sample_rate as f32 / params.fft_size as f32;
    let nyquist = sample_rate as f32 / 2.0;
    let f_min = MIN_FREQ_HZ.max(hz_per_bin).min(nyquist / 2.0);
    let ratio = nyquist / f_min;
    let n = f32::from(params.bins);
    (0..params.bins)
        .map(|i| {
            let lo_hz = f_min * ratio.powf(f32::from(i) / n);
            let hi_hz = f_min * ratio.powf(f32::from(i + 1) / n);
            let lo = ((lo_hz / hz_per_bin).floor() as usize).min(half);
            let hi = ((hi_hz / hz_per_bin).ceil() as usize).clamp(lo + 1, half + 1);
            (lo, hi)
        })
        .collect()
}

fn quantize_db(mag_norm: f32) -> u8 {
    let db = 20.0 * mag_norm.max(1e-10).log10();
    let t = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
    (t * 255.0).round() as u8
}

/// Calcula o espectrograma da mistura mono de `channels` (não intercalados).
/// Colunas centradas em `c·hop` (preenchimento com zeros nas bordas). Amostras não finitas = 0.
pub fn compute_spectrogram(
    channels: &[Vec<f32>],
    sample_rate: u32,
    params: SpectrogramParams,
) -> Result<Spectrogram, CodecError> {
    params.validate()?;
    if sample_rate == 0 {
        return Err(CodecError::InvalidParameter(
            "taxa de amostragem zero".into(),
        ));
    }
    let total = channels.iter().map(Vec::len).min().unwrap_or(0);
    let gain = 1.0 / channels.len().max(1) as f32;
    // mistura mono calculada sob demanda (sem cópia do áudio inteiro)
    let mono = |i: usize| -> f32 {
        channels
            .iter()
            .map(|c| if c[i].is_finite() { c[i] } else { 0.0 })
            .sum::<f32>()
            * gain
    };
    let n = params.fft_size as usize;
    let hop = params.hop as usize;
    let window: Vec<f32> = (0..n)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos())
        .collect();
    // senoide de amplitude 1 com Hann: pico ≈ n/4
    let norm = 4.0 / n as f32;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
    let bands = log_band_edges(&params, sample_rate);
    let columns = total.div_ceil(hop);
    let mut data = Vec::with_capacity(columns * usize::from(params.bins));
    let mut buf = vec![Complex::new(0.0f32, 0.0); n];
    let mut mags = vec![0.0f32; n / 2 + 1];
    for c in 0..columns {
        let center = (c * hop) as isize;
        for (k, slot) in buf.iter_mut().enumerate() {
            let i = center - (n / 2) as isize + k as isize;
            let x = if i >= 0 && (i as usize) < total {
                mono(i as usize)
            } else {
                0.0
            };
            *slot = Complex::new(x * window[k], 0.0);
        }
        fft.process(&mut buf);
        for (m, z) in mags.iter_mut().zip(&buf) {
            *m = z.norm() * norm;
        }
        for &(lo, hi) in &bands {
            let peak = mags[lo..hi].iter().copied().fold(0.0f32, f32::max);
            data.push(quantize_db(peak));
        }
    }
    Ok(Spectrogram {
        params,
        sample_rate,
        total_frames: total as u64,
        data,
    })
}

/// Caminho do cache para um conteúdo; valida o hash (impede travessia de caminho).
pub fn spectrogram_cache_path(
    project_root: &Path,
    content_hash: &str,
    params: SpectrogramParams,
) -> Result<PathBuf, CodecError> {
    if !is_content_hash(content_hash) {
        return Err(CodecError::InvalidPath(
            "hash de conteúdo inválido para o cache de espectrograma".into(),
        ));
    }
    params.validate()?;
    let mut p = project_root.to_path_buf();
    for part in SPECTROGRAM_DIR {
        p.push(part);
    }
    p.push(file_name(content_hash, params));
    Ok(p)
}

fn file_name(content_hash: &str, p: SpectrogramParams) -> String {
    format!("{content_hash}-{}-{}-{}.spec", p.fft_size, p.hop, p.bins)
}

/// Lê do cache ou calcula e grava. Cache com parâmetros, taxa ou duração diferentes é
/// recalculado. Falha ao gravar o cache não é erro (é regenerável).
pub fn load_or_compute_spectrogram(
    project_root: &Path,
    content_hash: &str,
    sample_rate: u32,
    params: SpectrogramParams,
    channels: &[Vec<f32>],
) -> Result<Spectrogram, CodecError> {
    let path = spectrogram_cache_path(project_root, content_hash, params)?;
    let total = channels.iter().map(Vec::len).min().unwrap_or(0) as u64;
    if let Some(s) = fs::read(&path)
        .ok()
        .and_then(|b| Spectrogram::from_bytes(&b))
        .filter(|s| s.params == params && s.sample_rate == sample_rate && s.total_frames == total)
    {
        return Ok(s);
    }
    let s = compute_spectrogram(channels, sample_rate, params)?;
    let _ = write_cache(project_root, content_hash, &s);
    Ok(s)
}

fn write_cache(project_root: &Path, content_hash: &str, s: &Spectrogram) -> Result<(), CodecError> {
    let dir = ensure_dir_within(project_root, &SPECTROGRAM_DIR)?;
    let dest = dir.join(file_name(content_hash, s.params));
    let mut temp = temp_in(&dir)?;
    temp.as_file_mut()
        .write_all(&s.to_bytes())
        .map_err(|e| CodecError::from_io(&e, &dest))?;
    persist(temp, &dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn sine(freq: f32, amp: f32, frames: usize) -> Vec<Vec<f32>> {
        vec![(0..frames)
            .map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / SR as f32).sin())
            .collect()]
    }

    fn loudest_bin(s: &Spectrogram, col: usize) -> usize {
        (0..usize::from(s.params.bins))
            .max_by_key(|&b| s.at(col, b))
            .unwrap()
    }

    #[test]
    fn params_are_validated() {
        assert!(SpectrogramParams::default().validate().is_ok());
        for bad in [
            SpectrogramParams {
                fft_size: 1000,
                ..Default::default()
            },
            SpectrogramParams {
                fft_size: 128,
                hop: 64,
                ..Default::default()
            },
            SpectrogramParams {
                hop: 4096,
                ..Default::default()
            },
            SpectrogramParams {
                hop: 10,
                ..Default::default()
            },
            SpectrogramParams {
                bins: 8,
                ..Default::default()
            },
            SpectrogramParams {
                bins: 1024,
                ..Default::default()
            },
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn silence_is_zero_and_shape_is_columns_by_bins() {
        let s =
            compute_spectrogram(&[vec![0.0; 10_000]], SR, SpectrogramParams::default()).unwrap();
        assert_eq!(s.columns(), 10_000usize.div_ceil(512));
        assert_eq!(s.data.len(), s.columns() * 128);
        assert!(s.data.iter().all(|&v| v == 0));
        let empty = compute_spectrogram(&[], SR, SpectrogramParams::default()).unwrap();
        assert_eq!(empty.columns(), 0);
    }

    #[test]
    fn sine_peaks_in_the_right_log_band_and_higher_tone_is_higher() {
        let p = SpectrogramParams::default();
        let low = compute_spectrogram(&sine(440.0, 0.9, 20_000), SR, p).unwrap();
        let high = compute_spectrogram(&sine(5_000.0, 0.9, 20_000), SR, p).unwrap();
        let (bl, bh) = (loudest_bin(&low, 20), loudest_bin(&high, 20));
        assert!(bh > bl, "5 kHz acima de 440 Hz ({bl} vs {bh})");
        // a faixa do pico contém 440 Hz no eixo log 20 Hz..24 kHz
        let ratio: f32 = 24_000.0 / 20.0;
        let lo = 20.0 * ratio.powf(bl as f32 / 128.0);
        let hi = 20.0 * ratio.powf((bl + 1) as f32 / 128.0);
        let hz_per_bin = SR as f32 / 2048.0;
        assert!(
            lo - hz_per_bin <= 440.0 && 440.0 <= hi + hz_per_bin,
            "{lo}..{hi}"
        );
        assert!(
            low.at(20, bl) > 230,
            "seno quase em fundo de escala: {}",
            low.at(20, bl)
        );
        // mais baixo em amplitude → valor menor
        let quiet = compute_spectrogram(&sine(440.0, 0.01, 20_000), SR, p).unwrap();
        assert!(quiet.at(20, bl) < low.at(20, bl));
    }

    #[test]
    fn non_finite_samples_are_ignored() {
        let mut ch = sine(440.0, 0.5, 4096);
        ch[0][100] = f32::NAN;
        ch[0][200] = f32::INFINITY;
        let s = compute_spectrogram(&ch, SR, SpectrogramParams::default()).unwrap();
        assert!(s.data.iter().any(|&v| v > 0));
    }

    #[test]
    fn bytes_roundtrip_and_corruption_is_detected() {
        let s = compute_spectrogram(&sine(1000.0, 0.5, 5000), SR, SpectrogramParams::default())
            .unwrap();
        let b = s.to_bytes();
        assert_eq!(Spectrogram::from_bytes(&b), Some(s.clone()));
        let mut bad = b.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert_eq!(Spectrogram::from_bytes(&bad), None, "checksum");
        let mut bad = b.clone();
        bad[0] = b'X';
        assert_eq!(Spectrogram::from_bytes(&bad), None, "magic");
        assert_eq!(Spectrogram::from_bytes(&b[..b.len() - 1]), None, "tamanho");
        assert_eq!(Spectrogram::from_bytes(&[]), None);
    }

    #[test]
    fn cache_is_written_reused_and_regenerated_when_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let hash = "c".repeat(64);
        let p = SpectrogramParams::default();
        let audio = sine(440.0, 0.5, 6000);
        let first = load_or_compute_spectrogram(dir.path(), &hash, SR, p, &audio).unwrap();
        let path = spectrogram_cache_path(dir.path(), &hash, p).unwrap();
        assert!(path.exists());
        // reutiliza o cache mesmo que o áudio passado seja outro de mesma duração
        let cached =
            load_or_compute_spectrogram(dir.path(), &hash, SR, p, &[vec![0.0; 6000]]).unwrap();
        assert_eq!(cached, first);
        // corrompido → recalcula a partir do áudio
        fs::write(&path, b"lixo").unwrap();
        let again =
            load_or_compute_spectrogram(dir.path(), &hash, SR, p, &[vec![0.0; 6000]]).unwrap();
        assert!(again.data.iter().all(|&v| v == 0));
        assert!(spectrogram_cache_path(dir.path(), "../../x", p).is_err());
    }
}
