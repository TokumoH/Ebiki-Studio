//! Time-stretch offline (WSOLA) e pitch shift offline (stretch + reamostragem).
//!
//! WSOLA (Verhelst & Roelands, 1993): quadros Hann com salto de síntese fixo `N/2`; cada novo
//! quadro é buscado em ±`tolerance` amostras ao redor da posição nominal, maximizando a
//! correlação normalizada com a continuação natural do quadro anterior. A busca usa a soma
//! dos canais e o **mesmo deslocamento é aplicado a todos os canais** (coerência estéreo).
//! Áudio intercalado (`[L, R, L, R, …]`).

use std::f64::consts::PI;

/// Limites aceitos para a razão de duração (saída/entrada).
pub const STRETCH_RANGE: (f64, f64) = (0.25, 4.0);

/// Porta de time-stretch offline (AI-ready: outros algoritmos/modelos podem implementá-la).
pub trait TimeStretcher {
    /// Retorna áudio intercalado com `round(frames · ratio)` quadros e a mesma altura.
    fn stretch(&self, samples: &[f32], channels: usize, ratio: f64) -> Vec<f32>;
}

/// Parâmetros do WSOLA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wsola {
    /// Tamanho do quadro (amostras); salto de síntese = metade.
    pub frame_len: usize,
    /// Desvio máximo de busca (amostras).
    pub tolerance: usize,
}

impl Default for Wsola {
    fn default() -> Self {
        Self {
            frame_len: 2048,
            tolerance: 512,
        }
    }
}

/// Time-stretch com os parâmetros padrão do [`Wsola`].
pub fn time_stretch(samples: &[f32], channels: usize, ratio: f64) -> Vec<f32> {
    Wsola::default().stretch(samples, channels, ratio)
}

fn sanitize_ratio(ratio: f64) -> f64 {
    if ratio.is_finite() {
        ratio.clamp(STRETCH_RANGE.0, STRETCH_RANGE.1)
    } else {
        1.0
    }
}

impl TimeStretcher for Wsola {
    fn stretch(&self, samples: &[f32], channels: usize, ratio: f64) -> Vec<f32> {
        let ch = channels.max(1);
        let frames = samples.len() / ch;
        let ratio = sanitize_ratio(ratio);
        let out_frames = (frames as f64 * ratio).round() as usize;
        if frames == 0 || out_frames == 0 {
            return Vec::new();
        }
        let n = (self.frame_len.max(64) / 2) * 2;
        let hs = n / 2;
        let tol = self.tolerance.min(n) as i64;
        let ha = hs as f64 / ratio;
        let window: Vec<f32> = (0..n)
            .map(|i| (0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos()) as f32)
            .collect();
        let mono: Vec<f32> = (0..frames)
            .map(|f| {
                let v = samples[f * ch..f * ch + ch]
                    .iter()
                    .map(|s| if s.is_finite() { *s } else { 0.0 })
                    .sum::<f32>();
                v / ch as f32
            })
            .collect();
        let mono_at = |i: i64| -> f32 {
            if i < 0 || i >= frames as i64 {
                0.0
            } else {
                mono[i as usize]
            }
        };
        let sample_at = |i: i64, c: usize| -> f32 {
            if i < 0 || i >= frames as i64 {
                0.0
            } else {
                let v = samples[i as usize * ch + c];
                if v.is_finite() {
                    v
                } else {
                    0.0
                }
            }
        };

        let half = hs as i64;
        let mut out = vec![0.0f32; out_frames * ch];
        let mut wsum = vec![0.0f32; out_frames];
        let mut prev: Option<i64> = None;
        let mut k = 0i64;
        loop {
            let out_start = k * hs as i64 - half;
            if out_start >= out_frames as i64 {
                break;
            }
            let nominal = (k as f64 * ha).round() as i64 - half;
            let pos = match prev {
                None => nominal,
                Some(p) => {
                    let natural = p + hs as i64;
                    best_offset(&mono_at, natural, nominal, tol, hs)
                }
            };
            for (i, &w) in window.iter().enumerate() {
                let o = out_start + i as i64;
                if o < 0 || o >= out_frames as i64 {
                    continue;
                }
                let o = o as usize;
                for c in 0..ch {
                    out[o * ch + c] += w * sample_at(pos + i as i64, c);
                }
                wsum[o] += w;
            }
            prev = Some(pos);
            k += 1;
        }
        for (f, w) in wsum.iter().enumerate() {
            if *w > 1e-3 {
                for c in 0..ch {
                    out[f * ch + c] /= *w;
                }
            }
        }
        out
    }
}

/// Melhor início em `[nominal − tol, nominal + tol]` para continuar `natural` (busca grossa
/// de passo 4 com subamostragem 2, depois refinamento ±4 completo).
fn best_offset<F: Fn(i64) -> f32>(x: &F, natural: i64, nominal: i64, tol: i64, len: usize) -> i64 {
    let score = |cand: i64, stride: usize| -> f32 {
        let mut num = 0.0f32;
        let mut energy = 1e-9f32;
        let mut i = 0;
        while i < len {
            let a = x(cand + i as i64);
            num += a * x(natural + i as i64);
            energy += a * a;
            i += stride;
        }
        num / energy.sqrt()
    };
    // Leve preferência pela posição nominal (desempate entre períodos equivalentes).
    let biased = |raw: f32, cand: i64| {
        raw - raw.abs() * 1e-4 * (cand - nominal).abs() as f32 / tol.max(1) as f32
    };
    let mut best = nominal;
    let mut best_score = f32::NEG_INFINITY;
    let mut cand = nominal - tol;
    while cand <= nominal + tol {
        let s = biased(score(cand, 2), cand);
        if s > best_score {
            best_score = s;
            best = cand;
        }
        cand += 4;
    }
    let coarse = best;
    best_score = f32::NEG_INFINITY;
    for cand in (coarse - 4).max(nominal - tol)..=(coarse + 4).min(nominal + tol) {
        let s = biased(score(cand, 1), cand);
        if s > best_score {
            best_score = s;
            best = cand;
        }
    }
    best
}

/// Reamostragem com sinc janelado (Blackman) e filtro anti-aliasing quando reduz a taxa.
/// Retorna exatamente `out_frames` quadros intercalados.
pub fn resample(samples: &[f32], channels: usize, out_frames: usize) -> Vec<f32> {
    let ch = channels.max(1);
    let frames = samples.len() / ch;
    if frames == 0 || out_frames == 0 {
        return vec![0.0; out_frames * ch];
    }
    let step = frames as f64 / out_frames as f64;
    let cutoff = (1.0 / step).min(1.0);
    let zero_crossings = 16.0;
    let half = (zero_crossings / cutoff).ceil() as i64;
    let mut out = vec![0.0f32; out_frames * ch];
    for (o, frame) in out.chunks_mut(ch).enumerate() {
        let src = o as f64 * step;
        let center = src.floor() as i64;
        let mut norm = 0.0f64;
        for i in (center - half + 1)..=(center + half) {
            let t = i as f64 - src;
            let u = t / half as f64;
            if u.abs() >= 1.0 {
                continue;
            }
            let window = 0.42 + 0.5 * (PI * u).cos() + 0.08 * (2.0 * PI * u).cos();
            let arg = PI * cutoff * t;
            let sinc = if arg.abs() < 1e-9 {
                1.0
            } else {
                arg.sin() / arg
            };
            let w = cutoff * sinc * window;
            norm += w;
            if i < 0 || i >= frames as i64 {
                continue;
            }
            for (c, y) in frame.iter_mut().enumerate() {
                let v = samples[i as usize * ch + c];
                if v.is_finite() {
                    *y += (w * v as f64) as f32;
                }
            }
        }
        if norm.abs() > 1e-9 {
            for y in frame.iter_mut() {
                *y = (*y as f64 / norm) as f32;
            }
        }
    }
    out
}

/// Pitch shift offline preservando a duração: stretch por `2^(st/12)` e reamostragem de volta.
pub fn pitch_shift_offline(samples: &[f32], channels: usize, semitones: f64) -> Vec<f32> {
    let ch = channels.max(1);
    let frames = samples.len() / ch;
    let st = if semitones.is_finite() {
        semitones.clamp(-24.0, 24.0)
    } else {
        0.0
    };
    let ratio = 2f64.powf(st / 12.0);
    let stretched = time_stretch(&samples[..frames * ch], ch, ratio);
    resample(&stretched, ch, frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::{cents_between, PitchDetector, YinConfig, YinDetector};
    use crate::signal;

    const SR: f32 = 48_000.0;

    fn pitch_at(sig: &[f32], center: usize) -> f32 {
        let mut yin = YinDetector::new(YinConfig::default(), SR);
        let n = yin.frame_len();
        let s = center - n / 2;
        let e = yin.detect(&sig[s..s + n]);
        assert!(e.voiced);
        e.frequency_hz
    }

    #[test]
    fn stretch_1_5_changes_length_and_keeps_pitch() {
        let sig = signal::sine(220.0, SR, 48_000, 0.5);
        let out = time_stretch(&sig, 1, 1.5);
        let ratio = out.len() as f64 / sig.len() as f64;
        assert!((ratio - 1.5).abs() < 0.015, "razão {ratio}");
        for c in [out.len() / 4, out.len() / 2, 3 * out.len() / 4] {
            let f = pitch_at(&out, c);
            assert!(cents_between(220.0, f).abs() < 10.0, "{f} Hz em {c}");
        }
        assert!(out.iter().all(|v| v.is_finite() && v.abs() < 0.8));
    }

    #[test]
    fn compress_keeps_pitch_of_sawtooth() {
        let sig = signal::sawtooth(196.0, SR, 48_000, 0.4);
        let out = time_stretch(&sig, 1, 0.7);
        assert_eq!(out.len(), 33_600);
        let f = pitch_at(&out, out.len() / 2);
        assert!(cents_between(196.0, f).abs() < 10.0, "{f}");
    }

    #[test]
    fn stereo_channels_stay_phase_coherent() {
        let left = signal::sine(330.0, SR, 24_000, 0.5);
        let stereo: Vec<f32> = left.iter().flat_map(|l| [*l, 0.5 * *l]).collect();
        let out = time_stretch(&stereo, 2, 1.25);
        assert_eq!(out.len(), 30_000 * 2);
        for f in out.chunks(2) {
            assert!(
                (f[1] - 0.5 * f[0]).abs() < 1e-5,
                "mesmo deslocamento nos canais"
            );
        }
    }

    #[test]
    fn identity_and_invalid_inputs() {
        let sig = signal::sine(440.0, SR, 10_000, 0.5);
        let out = time_stretch(&sig, 1, 1.0);
        assert_eq!(out.len(), sig.len());
        let err = sig[3000..7000]
            .iter()
            .zip(&out[3000..7000])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(err < 1e-4, "razão 1 ≈ identidade ({err})");
        assert!(time_stretch(&[], 1, 2.0).is_empty());
        assert_eq!(time_stretch(&sig, 1, f64::NAN).len(), sig.len());
        assert_eq!(time_stretch(&sig, 0, 100.0).len(), sig.len() * 4);
    }

    #[test]
    fn resample_preserves_dc_and_frequency_scaling() {
        let dc = vec![0.5f32; 1000];
        let out = resample(&dc, 1, 500);
        assert_eq!(out.len(), 500);
        assert!(out[100..400].iter().all(|v| (v - 0.5).abs() < 1e-3));
        let sig = signal::sine(220.0, SR, 48_000, 0.5);
        let half = resample(&sig, 1, 24_000); // lido 2× mais rápido → 440 Hz a 48 kHz
        let f = pitch_at(&half, 12_000);
        assert!(cents_between(440.0, f).abs() < 5.0, "{f}");
    }

    #[test]
    fn offline_pitch_shift_up_an_octave_keeps_length() {
        let sig = signal::sine(220.0, SR, 48_000, 0.5);
        let out = pitch_shift_offline(&sig, 1, 12.0);
        assert_eq!(out.len(), sig.len());
        let f = pitch_at(&out, out.len() / 2);
        assert!(cents_between(440.0, f).abs() < 10.0, "{f}");
        let down = pitch_shift_offline(&sig, 1, -5.0);
        let f = pitch_at(&down, down.len() / 2);
        assert!(cents_between(220.0 * 2f32.powf(-5.0 / 12.0), f).abs() < 10.0);
    }
}
