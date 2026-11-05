//! Codificação WAV PCM 16 bits **em memória** (audição na UI via WebAudio, sem tocar o disco).

/// Informações do cabeçalho de um WAV PCM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavInfo {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub frames: u32,
}

/// Codifica canais não intercalados (todos do mesmo tamanho; o menor manda) em WAV PCM 16 bits.
/// Amostras são limitadas a [−1, 1]; `NaN`/infinito viram silêncio.
pub fn encode_wav_pcm16(channels: &[Vec<f32>], sample_rate: u32) -> Vec<u8> {
    let ch = channels.len().clamp(1, u16::MAX as usize) as u16;
    let frames = channels.iter().map(Vec::len).min().unwrap_or(0);
    let data_len = frames as u64 * ch as u64 * 2;
    let data_len = data_len.min((u32::MAX - 44) as u64) as u32;
    let frames = (data_len / (ch as u32 * 2)) as usize;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&ch.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * ch as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(ch * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    if channels.is_empty() {
        return out;
    }
    for i in 0..frames {
        for c in channels {
            let x = c[i];
            let x = if x.is_finite() {
                x.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let s = (x * i16::MAX as f32).round() as i16;
            out.extend_from_slice(&s.to_le_bytes());
        }
    }
    out
}

/// Lê o cabeçalho canônico (44 bytes) produzido por [`encode_wav_pcm16`].
pub fn wav_info(bytes: &[u8]) -> Option<WavInfo> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..16] != b"WAVEfmt " {
        return None;
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    if u16_at(20) != 1 || &bytes[36..40] != b"data" {
        return None;
    }
    let channels = u16_at(22);
    let bits = u16_at(34);
    let data = u32_at(40);
    if channels == 0 || bits == 0 || u32_at(4) != 36 + data || bytes.len() != 44 + data as usize {
        return None;
    }
    Some(WavInfo {
        channels,
        sample_rate: u32_at(24),
        bits_per_sample: bits,
        frames: data / (channels as u32 * bits as u32 / 8),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_samples_are_canonical() {
        let wav = encode_wav_pcm16(
            &[vec![0.0, 1.0, -1.0, 2.0], vec![0.5, f32::NAN, 0.0, 0.0]],
            44_100,
        );
        let info = wav_info(&wav).unwrap();
        assert_eq!(
            info,
            WavInfo {
                channels: 2,
                sample_rate: 44_100,
                bits_per_sample: 16,
                frames: 4
            }
        );
        let s = |i: usize| i16::from_le_bytes([wav[44 + 2 * i], wav[45 + 2 * i]]);
        assert_eq!((s(0), s(1)), (0, 16_384));
        assert_eq!((s(2), s(3)), (i16::MAX, 0), "NaN vira silêncio");
        assert_eq!(s(4), -i16::MAX);
        assert_eq!(s(6), i16::MAX, "limitado a 1,0");
        // byte rate = sr × canais × 2
        assert_eq!(
            u32::from_le_bytes([wav[28], wav[29], wav[30], wav[31]]),
            176_400
        );
    }

    #[test]
    fn mono_and_empty() {
        let m = encode_wav_pcm16(&[vec![0.25; 10]], 48_000);
        assert_eq!(wav_info(&m).unwrap().frames, 10);
        assert_eq!(wav_info(&m).unwrap().channels, 1);
        let e = encode_wav_pcm16(&[], 48_000);
        assert_eq!(e.len(), 44);
        assert!(wav_info(b"RIFF").is_none());
    }
}
