//! Audição pelo motor (S-3): Harmonia, Bases e Partitura tocam no fone escolhido no app, e não na saída
//! padrão do sistema pela WebAudio. Os serviços continuam gerando um WAV em memória; aqui ele é lido e
//! mandado à engine como prévia (fora do master: não é gravado nem exportado).

use crate::studio::Studio;
use crate::{AppError, AppResult};
use audio_engine::preview::PreviewClip;

/// Lê um WAV canônico (cabeçalho de 44 bytes) PCM 16 bits ou float 32 — os dois formatos que os
/// serviços de audição geram. Devolve canais separados e a taxa.
pub fn decode_wav(bytes: &[u8]) -> Option<(Vec<Vec<f32>>, u32)> {
    if bytes.len() < 44
        || &bytes[0..4] != b"RIFF"
        || &bytes[8..16] != b"WAVEfmt "
        || &bytes[36..40] != b"data"
    {
        return None;
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let (format, channels, rate, bits) = (u16_at(20), u16_at(22) as usize, u32_at(24), u16_at(34));
    let bytes_per = match (format, bits) {
        (1, 16) => 2,
        (3, 32) => 4,
        _ => return None,
    };
    if channels == 0 || rate == 0 {
        return None;
    }
    let data = &bytes[44..(44 + u32_at(40) as usize).min(bytes.len())];
    let frames = data.len() / (bytes_per * channels);
    let mut out = vec![Vec::with_capacity(frames); channels];
    for f in 0..frames {
        for (c, plane) in out.iter_mut().enumerate() {
            let i = (f * channels + c) * bytes_per;
            let v = if bytes_per == 2 {
                i16::from_le_bytes([data[i], data[i + 1]]) as f32 / i16::MAX as f32
            } else {
                f32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]])
            };
            plane.push(if v.is_finite() { v } else { 0.0 });
        }
    }
    Some((out, rate))
}

/// Converte a taxa por interpolação linear (prévia: as audições já saem na taxa do projeto; isto só
/// cobre o caso raro de a engine estar noutra taxa).
pub fn resample_linear(planes: Vec<Vec<f32>>, from: u32, to: u32) -> Vec<Vec<f32>> {
    if from == to || from == 0 || to == 0 {
        return planes;
    }
    let ratio = from as f64 / to as f64;
    planes
        .into_iter()
        .map(|p| {
            if p.is_empty() {
                return p;
            }
            let n = ((p.len() as f64) / ratio).floor() as usize;
            (0..n)
                .map(|i| {
                    let x = i as f64 * ratio;
                    let k = x.floor() as usize;
                    let t = (x - k as f64) as f32;
                    let a = p[k.min(p.len() - 1)];
                    let b = p[(k + 1).min(p.len() - 1)];
                    a + (b - a) * t
                })
                .collect()
        })
        .collect()
}

impl Studio {
    /// Toca o WAV de uma audição pela engine. Devolve a duração em segundos (a UI apaga o "tocando").
    pub fn play_preview_wav(&mut self, wav: &[u8]) -> AppResult<f64> {
        let (planes, rate) = decode_wav(wav)
            .ok_or_else(|| AppError::new("app.preview", "Áudio da audição inválido."))?;
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| AppError::new("app.no_engine", "Engine de áudio indisponível."))?;
        let target = engine.sample_rate();
        let clip = PreviewClip::from_planar(&resample_linear(planes, rate, target));
        let seconds = clip.frames() as f64 / target.max(1) as f64;
        engine
            .play_preview(clip)
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        Ok(seconds)
    }

    /// Para a audição em andamento.
    pub fn stop_preview(&mut self) -> AppResult<()> {
        if let Some(engine) = self.engine.as_mut() {
            engine
                .stop_preview()
                .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::encode_wav_pcm16;
    use crate::music_support::wav_bytes;

    #[test]
    fn decodes_the_two_formats_the_auditions_produce() {
        let (p, sr) = decode_wav(&encode_wav_pcm16(
            &[vec![0.5, -0.5, 0.0], vec![0.25, 0.25, 0.25]],
            44_100,
        ))
        .unwrap();
        assert_eq!(sr, 44_100);
        assert_eq!(p.len(), 2);
        assert!(
            (p[0][0] - 0.5).abs() < 1e-3
                && (p[0][1] + 0.5).abs() < 1e-3
                && (p[1][2] - 0.25).abs() < 1e-3
        );
        let (p, sr) = decode_wav(&wav_bytes(&[vec![0.125, -1.0]], 48_000)).unwrap();
        assert_eq!((sr, p.len(), p[0].clone()), (48_000, 1, vec![0.125, -1.0]));
        assert!(decode_wav(b"nada").is_none());
    }

    #[test]
    fn linear_resample_keeps_duration_and_level() {
        let p = resample_linear(vec![vec![0.5; 44_100]], 44_100, 48_000);
        assert!((p[0].len() as i64 - 48_000).abs() <= 1);
        assert!(p[0].iter().all(|x| (x - 0.5).abs() < 1e-6));
        assert_eq!(
            resample_linear(vec![vec![1.0, 2.0]], 48_000, 48_000),
            vec![vec![1.0, 2.0]]
        );
    }
}
