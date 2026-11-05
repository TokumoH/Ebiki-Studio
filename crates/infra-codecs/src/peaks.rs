//! Picos de waveform (mín./máx. por canal e por bloco) e cache binário regenerável.
//!
//! Formato do arquivo `cache/peaks/<hash>-<fpp>.peaks` (little-endian):
//!
//! | Offset | Tamanho | Campo                                         |
//! |--------|---------|-----------------------------------------------|
//! | 0      | 4       | magic `AMPK`                                  |
//! | 4      | 2       | versão (`1`)                                  |
//! | 6      | 2       | canais (1..=32)                               |
//! | 8      | 4       | quadros por pico (`fpp`)                      |
//! | 12     | 4       | reservado (0)                                 |
//! | 16     | 8       | número de blocos (`buckets`)                  |
//! | 24     | 8       | quadros totais do áudio                       |
//! | 32     | 8       | FNV-1a 64 do payload                          |
//! | 40     | …       | por canal: `buckets` × `f32` mín., depois máx. |
//!
//! Cache inválido (magic, versão, tamanho, checksum, valores não finitos) é ignorado e
//! recalculado — nunca é erro para o usuário.

use crate::decode::DecodedAudio;
use crate::error::CodecError;
use crate::fsutil::{ensure_dir_within, persist, temp_in};
use crate::import::is_content_hash;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PEAKS_MAGIC: [u8; 4] = *b"AMPK";
pub const PEAKS_VERSION: u16 = 1;
const HEADER_LEN: usize = 40;
const MAX_CHANNELS: u16 = 32;

/// Pasta (relativa à raiz do projeto) do cache de picos.
pub const PEAKS_DIR: [&str; 2] = ["cache", "peaks"];

/// Resoluções padrão (quadros por pico) para zoom.
pub const DEFAULT_RESOLUTIONS: [u32; 3] = [256, 1024, 4096];

/// Mínimos e máximos de um canal; `min[i] <= max[i]`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChannelPeaks {
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Peaks {
    pub frames_per_peak: u32,
    pub total_frames: u64,
    pub channels: Vec<ChannelPeaks>,
}

impl Peaks {
    pub fn buckets(&self) -> usize {
        self.channels.first().map_or(0, |c| c.min.len())
    }

    /// Pares `[min, max]` intercalados de um canal (formato do IPC `waveform_peaks`).
    pub fn interleaved_min_max(&self, channel: usize) -> Vec<f32> {
        self.channels.get(channel).map_or_else(Vec::new, |c| {
            c.min
                .iter()
                .zip(&c.max)
                .flat_map(|(&a, &b)| [a, b])
                .collect()
        })
    }

    /// Serializa no formato binário do cache.
    pub fn to_bytes(&self) -> Vec<u8> {
        let buckets = self.buckets();
        let mut payload = Vec::with_capacity(self.channels.len() * buckets * 8);
        for c in &self.channels {
            for v in c.min.iter().chain(&c.max) {
                payload.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
        out.extend_from_slice(&PEAKS_MAGIC);
        out.extend_from_slice(&PEAKS_VERSION.to_le_bytes());
        out.extend_from_slice(&(self.channels.len() as u16).to_le_bytes());
        out.extend_from_slice(&self.frames_per_peak.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(buckets as u64).to_le_bytes());
        out.extend_from_slice(&self.total_frames.to_le_bytes());
        out.extend_from_slice(&fnv1a64(&payload).to_le_bytes());
        out.extend_from_slice(&payload);
        out
    }

    /// Lê o formato binário; `None` para qualquer inconsistência.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_LEN || bytes[0..4] != PEAKS_MAGIC {
            return None;
        }
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap_or([0; 4]));
        let u64_at = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap_or([0; 8]));
        if u16_at(4) != PEAKS_VERSION {
            return None;
        }
        let channels = u16_at(6);
        let fpp = u32_at(8);
        let buckets = u64_at(16);
        let total_frames = u64_at(24);
        let checksum = u64_at(32);
        if channels == 0 || channels > MAX_CHANNELS || fpp == 0 {
            return None;
        }
        let expected_buckets = total_frames.div_ceil(u64::from(fpp));
        if buckets != expected_buckets {
            return None;
        }
        let payload_len = u64::from(channels).checked_mul(buckets)?.checked_mul(8)?;
        if (bytes.len() - HEADER_LEN) as u64 != payload_len {
            return None;
        }
        let payload = &bytes[HEADER_LEN..];
        if fnv1a64(payload) != checksum {
            return None;
        }
        let buckets = buckets as usize;
        let mut floats = payload
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c));
        let mut out = Vec::with_capacity(usize::from(channels));
        for _ in 0..channels {
            let min: Vec<f32> = floats.by_ref().take(buckets).collect();
            let max: Vec<f32> = floats.by_ref().take(buckets).collect();
            if min
                .iter()
                .zip(&max)
                .any(|(a, b)| !a.is_finite() || !b.is_finite() || a > b)
            {
                return None;
            }
            out.push(ChannelPeaks { min, max });
        }
        Some(Self {
            frames_per_peak: fpp,
            total_frames,
            channels: out,
        })
    }
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn check_fpp(frames_per_peak: u32) -> Result<(), CodecError> {
    if frames_per_peak == 0 {
        return Err(CodecError::InvalidParameter(
            "quadros por pico deve ser maior que zero".into(),
        ));
    }
    Ok(())
}

/// Calcula mín./máx. por canal em blocos de `frames_per_peak` quadros (o último pode ser menor).
/// Amostras não finitas são tratadas como 0.
pub fn compute_peaks(audio: &DecodedAudio, frames_per_peak: u32) -> Result<Peaks, CodecError> {
    check_fpp(frames_per_peak)?;
    let fpp = frames_per_peak as usize;
    let frames = audio.frames();
    let channels = audio
        .channels
        .iter()
        .map(|c| {
            let (min, max) = c[..frames.min(c.len())]
                .chunks(fpp)
                .map(|chunk| {
                    chunk
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &x| {
                            let x = if x.is_finite() { x } else { 0.0 };
                            (lo.min(x), hi.max(x))
                        })
                })
                .unzip();
            ChannelPeaks { min, max }
        })
        .collect();
    Ok(Peaks {
        frames_per_peak,
        total_frames: frames as u64,
        channels,
    })
}

/// Reduz picos finos para uma resolução múltipla (`factor` blocos viram 1).
fn coarsen(fine: &Peaks, factor: usize) -> Peaks {
    let channels = fine
        .channels
        .iter()
        .map(|c| {
            let min = c
                .min
                .chunks(factor)
                .map(|s| s.iter().copied().fold(f32::INFINITY, f32::min))
                .collect();
            let max = c
                .max
                .chunks(factor)
                .map(|s| s.iter().copied().fold(f32::NEG_INFINITY, f32::max))
                .collect();
            ChannelPeaks { min, max }
        })
        .collect();
    Peaks {
        frames_per_peak: fine.frames_per_peak * factor as u32,
        total_frames: fine.total_frames,
        channels,
    }
}

/// Calcula várias resoluções; quando uma é múltipla da menor, deriva dela (sem reler o áudio).
pub fn compute_multi_resolution(
    audio: &DecodedAudio,
    resolutions: &[u32],
) -> Result<Vec<Peaks>, CodecError> {
    let Some(&finest) = resolutions.iter().min() else {
        return Ok(Vec::new());
    };
    check_fpp(finest)?;
    let base = compute_peaks(audio, finest)?;
    resolutions
        .iter()
        .map(|&r| {
            if r == finest {
                Ok(base.clone())
            } else if r % finest == 0 {
                Ok(coarsen(&base, (r / finest) as usize))
            } else {
                compute_peaks(audio, r)
            }
        })
        .collect()
}

/// Caminho do cache de picos de um conteúdo; valida o hash (impede travessia de caminho).
pub fn peaks_cache_path(
    project_root: &Path,
    content_hash: &str,
    frames_per_peak: u32,
) -> Result<PathBuf, CodecError> {
    if !is_content_hash(content_hash) {
        return Err(CodecError::InvalidPath(
            "hash de conteúdo inválido para o cache de picos".into(),
        ));
    }
    check_fpp(frames_per_peak)?;
    let mut p = project_root.to_path_buf();
    for part in PEAKS_DIR {
        p.push(part);
    }
    p.push(format!("{content_hash}-{frames_per_peak}.peaks"));
    Ok(p)
}

fn read_cache(path: &Path, fpp: u32) -> Option<Peaks> {
    let bytes = fs::read(path).ok()?;
    Peaks::from_bytes(&bytes).filter(|p| p.frames_per_peak == fpp)
}

fn write_cache(project_root: &Path, content_hash: &str, peaks: &Peaks) -> Result<(), CodecError> {
    let dir = ensure_dir_within(project_root, &PEAKS_DIR)?;
    let dest = dir.join(format!("{content_hash}-{}.peaks", peaks.frames_per_peak));
    let mut temp = temp_in(&dir)?;
    temp.as_file_mut()
        .write_all(&peaks.to_bytes())
        .map_err(|e| CodecError::from_io(&e, &dest))?;
    persist(temp, &dest)
}

/// Lê os picos do cache ou, se ausente/corrompido, decodifica (`source`), calcula e grava.
/// Falha ao gravar o cache não é erro (o cache é regenerável).
pub fn load_or_compute(
    project_root: &Path,
    content_hash: &str,
    frames_per_peak: u32,
    source: impl FnOnce() -> Result<DecodedAudio, CodecError>,
) -> Result<Peaks, CodecError> {
    let mut v = load_or_compute_multi(project_root, content_hash, &[frames_per_peak], source)?;
    Ok(v.remove(0))
}

/// Versão multi-resolução de [`load_or_compute`]: decodifica no máximo uma vez.
pub fn load_or_compute_multi(
    project_root: &Path,
    content_hash: &str,
    resolutions: &[u32],
    source: impl FnOnce() -> Result<DecodedAudio, CodecError>,
) -> Result<Vec<Peaks>, CodecError> {
    let paths = resolutions
        .iter()
        .map(|&r| peaks_cache_path(project_root, content_hash, r))
        .collect::<Result<Vec<_>, _>>()?;
    let cached: Vec<Option<Peaks>> = paths
        .iter()
        .zip(resolutions)
        .map(|(p, &r)| read_cache(p, r))
        .collect();
    if cached.iter().all(Option::is_some) {
        return Ok(cached.into_iter().flatten().collect());
    }
    let audio = source()?;
    let missing: Vec<u32> = resolutions
        .iter()
        .zip(&cached)
        .filter(|(_, c)| c.is_none())
        .map(|(&r, _)| r)
        .collect();
    let mut computed = compute_multi_resolution(&audio, &missing)?.into_iter();
    let mut out = Vec::with_capacity(resolutions.len());
    for c in cached {
        match c {
            Some(p) => out.push(p),
            None => {
                let p = computed.next().ok_or_else(|| {
                    CodecError::InvalidParameter("resolução de picos inconsistente".into())
                })?;
                let _ = write_cache(project_root, content_hash, &p);
                out.push(p);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio(ch: Vec<Vec<f32>>) -> DecodedAudio {
        DecodedAudio {
            channels: ch,
            sample_rate: 48_000,
        }
    }

    #[test]
    fn peaks_of_known_signal() {
        let a = audio(vec![vec![0.0, 0.5, -0.25, 1.0, -1.0, 0.1, 0.2]]);
        let p = compute_peaks(&a, 3).unwrap();
        assert_eq!(p.buckets(), 3);
        assert_eq!(p.channels[0].min, vec![-0.25, -1.0, 0.2]);
        assert_eq!(p.channels[0].max, vec![0.5, 1.0, 0.2]);
        assert_eq!(
            p.interleaved_min_max(0),
            vec![-0.25, 0.5, -1.0, 1.0, 0.2, 0.2]
        );
    }

    #[test]
    fn zero_fpp_is_rejected() {
        assert!(compute_peaks(&audio(vec![vec![0.0]]), 0).is_err());
    }

    #[test]
    fn binary_round_trip_and_corruption_detection() {
        let a = audio(vec![
            (0..1000).map(|i| (i as f32 * 0.01).sin()).collect(),
            (0..1000).map(|i| (i as f32 * 0.02).cos()).collect(),
        ]);
        let p = compute_peaks(&a, 64).unwrap();
        let bytes = p.to_bytes();
        assert_eq!(Peaks::from_bytes(&bytes).unwrap(), p);
        for i in [0usize, 5, 20, HEADER_LEN + 3, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[i] ^= 0x55;
            assert!(Peaks::from_bytes(&bad).is_none(), "byte {i}");
        }
        assert!(Peaks::from_bytes(&bytes[..bytes.len() - 1]).is_none());
        assert!(Peaks::from_bytes(&[]).is_none());
    }

    #[test]
    fn multi_resolution_matches_direct_computation() {
        let a = audio(vec![(0..10_000)
            .map(|i| ((i * 7919) % 2000) as f32 / 1000.0 - 1.0)
            .collect()]);
        let multi = compute_multi_resolution(&a, &DEFAULT_RESOLUTIONS).unwrap();
        for (p, &r) in multi.iter().zip(&DEFAULT_RESOLUTIONS) {
            assert_eq!(*p, compute_peaks(&a, r).unwrap());
        }
    }

    #[test]
    fn cache_path_rejects_bad_hash() {
        let root = Path::new("/tmp/proj");
        assert!(peaks_cache_path(root, "../../etc/passwd", 256).is_err());
        let ok = peaks_cache_path(root, &"0".repeat(64), 256).unwrap();
        assert!(ok.ends_with(format!("cache/peaks/{}-256.peaks", "0".repeat(64))));
    }
}
