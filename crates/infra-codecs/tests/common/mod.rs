//! Geradores de sinal e utilitários para os testes de integração (fixtures só em `tempfile`).
#![allow(dead_code)]

use infra_codecs::{DecodedAudio, EncodeOptions, Encoder, ExportFormat};
use std::path::{Path, PathBuf};

pub fn sine(freq: f32, sr: u32, frames: usize, amp: f32) -> Vec<f32> {
    (0..frames)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin() * amp)
        .collect()
}

/// Ruído branco determinístico (xorshift) em ±amp.
pub fn noise(seed: u64, frames: usize, amp: f32) -> Vec<f32> {
    let mut s = seed | 1;
    (0..frames)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            ((s >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) as f32 * amp
        })
        .collect()
}

pub fn rms(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>() / x.len() as f64).sqrt() as f32
}

/// Frequência estimada por cruzamentos de zero ascendentes (com interpolação linear).
pub fn estimate_freq(x: &[f32], sr: u32) -> f64 {
    let mut first = None;
    let mut last = 0.0;
    let mut count = 0usize;
    for i in 1..x.len() {
        if x[i - 1] < 0.0 && x[i] >= 0.0 {
            let frac = f64::from(-x[i - 1]) / f64::from(x[i] - x[i - 1]);
            let t = (i - 1) as f64 + frac;
            if first.is_none() {
                first = Some(t);
            } else {
                count += 1;
            }
            last = t;
        }
    }
    let first = first.expect("sem cruzamentos de zero");
    count as f64 * f64::from(sr) / (last - first)
}

/// Escreve `audio` com o `Encoder` em blocos irregulares (exercita o fluxo).
pub fn encode(
    path: &Path,
    format: ExportFormat,
    audio: &DecodedAudio,
    options: &EncodeOptions,
) -> PathBuf {
    let mut enc = Encoder::new(
        path,
        format,
        audio.sample_rate,
        audio.channels.len() as u16,
        options,
    )
    .expect("Encoder::new");
    let frames = audio.frames();
    let mut start = 0;
    let mut step = 1000;
    while start < frames {
        let end = (start + step).min(frames);
        let block: Vec<&[f32]> = audio.channels.iter().map(|c| &c[start..end]).collect();
        enc.write(&block).expect("write");
        start = end;
        step = if step == 1000 { 3071 } else { 1000 };
    }
    enc.finish().expect("finish").path
}

pub fn stereo(l: Vec<f32>, r: Vec<f32>, sr: u32) -> DecodedAudio {
    DecodedAudio {
        channels: vec![l, r],
        sample_rate: sr,
    }
}

pub fn mono(x: Vec<f32>, sr: u32) -> DecodedAudio {
    DecodedAudio {
        channels: vec![x],
        sample_rate: sr,
    }
}

/// Melhor correlação normalizada entre `a` e `b` deslocando `b` em ±`max_lag` amostras.
pub fn best_correlation(a: &[f32], b: &[f32], max_lag: isize) -> (isize, f64) {
    let mut best = (0, f64::MIN);
    for lag in -max_lag..=max_lag {
        let (mut num, mut ea, mut eb) = (0.0f64, 0.0f64, 0.0f64);
        for (i, &x) in a.iter().enumerate() {
            let j = i as isize + lag;
            if j < 0 || j as usize >= b.len() {
                continue;
            }
            let y = b[j as usize];
            num += f64::from(x) * f64::from(y);
            ea += f64::from(x) * f64::from(x);
            eb += f64::from(y) * f64::from(y);
        }
        let c = if ea > 0.0 && eb > 0.0 {
            num / (ea.sqrt() * eb.sqrt())
        } else {
            0.0
        };
        if c > best.1 {
            best = (lag, c);
        }
    }
    best
}
