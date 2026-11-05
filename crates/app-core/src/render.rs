//! Renderização offline (exportação, stems, análise) e medições.

use crate::compile::compile;
use crate::ports::ProcessorFactory;
use crate::session::Session;
use crate::studio::{AppError, AppResult};
use audio_engine::{create_engine, offline, EngineConfig, SourceBuffer};
use ctx_mixing::advisor::{ChannelAnalysis, MasterAnalysis};
use ctx_mixing::{StripEdit, StripTarget};
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::{Db, Tick};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Limite de duração renderizada (proteção contra projetos com clipes absurdamente distantes).
pub const MAX_RENDER_SECONDS: f64 = 60.0 * 60.0;

pub fn render_session(
    session: &Session,
    sources: &HashMap<AudioSourceId, Arc<SourceBuffer>>,
    factory: &dyn ProcessorFactory,
    range: Option<(Tick, Tick)>,
    tail_seconds: f64,
) -> AppResult<(Vec<f32>, Vec<f32>)> {
    let p = &session.project;
    let sr = p.format.sample_rate.hz();
    let (start, end) = match range {
        Some((a, b)) if b > a => (
            p.tempo().tick_to_frame(a, sr),
            p.tempo().tick_to_frame(b, sr),
        ),
        Some(_) => {
            return Err(AppError::new(
                "app.invalid_range",
                "Intervalo de exportação inválido.",
            ))
        }
        None => (0, p.content_end_frame()),
    };
    if end <= start {
        return Err(AppError::new(
            "app.empty_mix",
            "Não há áudio para renderizar.",
        ));
    }
    let total = end - start + (tail_seconds.max(0.0) * sr as f64) as u64;
    if total as f64 / sr as f64 > MAX_RENDER_SECONDS {
        return Err(AppError::new(
            "app.too_long",
            "A renderização excede 1 hora.",
        ));
    }
    let mut session = session.clone();
    // exportação não usa monitoração de entrada, loop nem metrônomo
    session.recording = Default::default();
    let c = compile(&session, sources, factory, &HashSet::new(), 0);
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: sr,
        ..Default::default()
    });
    h.swap_graph(c.graph)
        .map_err(|e| AppError::new("app.engine", e.to_string()))?;
    h.seek(start)
        .map_err(|e| AppError::new("app.engine", e.to_string()))?;
    h.play()
        .map_err(|e| AppError::new("app.engine", e.to_string()))?;
    let out = offline::render(&mut core, total as usize, 1024);
    h.collect_garbage();
    Ok(out)
}

/// Cópia da sessão com apenas `track` audível (para stems e análise por canal).
pub fn solo_session(session: &Session, track: TrackId) -> Session {
    let mut s = session.clone();
    let ids: Vec<TrackId> = s.mixer.strips().keys().copied().collect();
    for id in ids {
        let _ = s.mixer.edit(
            StripTarget::Track(id),
            &StripEdit::Solo { value: id == track },
        );
        if id == track {
            let _ = s
                .mixer
                .edit(StripTarget::Track(id), &StripEdit::Mute { value: false });
        }
    }
    s
}

pub fn normalize(l: &mut [f32], r: &mut [f32], target_peak: f32) {
    let peak = l.iter().chain(r.iter()).fold(0f32, |m, x| m.max(x.abs()));
    if peak > 1e-9 {
        let g = target_peak / peak;
        for x in l.iter_mut().chain(r.iter_mut()) {
            *x *= g;
        }
    }
}

fn db(x: f32) -> f32 {
    Db::from_linear(x).0
}

pub fn analyze_channel(name: &str, l: &[f32], r: &[f32]) -> ChannelAnalysis {
    let peak = l.iter().chain(r).fold(0f32, |m, x| m.max(x.abs()));
    let n = (l.len() + r.len()).max(1);
    let rms = (l
        .iter()
        .chain(r)
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        / n as f64)
        .sqrt() as f32;
    let clipped = l.iter().chain(r).filter(|x| x.abs() >= 1.0).count() as u64;
    ChannelAnalysis {
        name: name.to_owned(),
        peak_db: db(peak),
        rms_db: db(rms),
        clipped_samples: clipped,
    }
}

pub fn analyze_master(l: &[f32], r: &[f32], sr: u32) -> MasterAnalysis {
    use dsp::Processor;
    let ch = analyze_channel("master", l, r);
    let mut loud = dsp::LoudnessMeter::new();
    let mut tp = dsp::TruePeakMeter::new();
    let mut corr = dsp::CorrelationMeter::new();
    loud.prepare(sr as f32, 4096);
    tp.prepare(sr as f32, 4096);
    corr.prepare(sr as f32, 4096);
    for (a, b) in l.chunks(4096).zip(r.chunks(4096)) {
        loud.analyze(a, b);
        tp.analyze(a, b);
        corr.analyze(a, b);
    }
    MasterAnalysis {
        peak_db: ch.peak_db,
        true_peak_db: tp.max_db(),
        rms_db: ch.rms_db,
        integrated_lufs: loud.integrated_lufs().unwrap_or(f32::NEG_INFINITY),
        correlation: corr.correlation(),
        clipped_samples: ch.clipped_samples,
    }
}

/// Mín./máx. por bloco de `fpp` amostras (canais combinados), intercalados.
pub fn peaks(buf: &SourceBuffer, fpp: usize) -> Vec<f32> {
    let frames = buf.frames() as usize;
    let buckets = frames.div_ceil(fpp.max(1));
    let mut out = Vec::with_capacity(buckets * 2);
    for b in 0..buckets {
        let (a, e) = (b * fpp, ((b + 1) * fpp).min(frames));
        let mut mn = f32::INFINITY;
        let mut mx = f32::NEG_INFINITY;
        for ch in &buf.channels {
            for &x in &ch[a..e] {
                mn = mn.min(x);
                mx = mx.max(x);
            }
        }
        out.push(if mn.is_finite() { mn } else { 0.0 });
        out.push(if mx.is_finite() { mx } else { 0.0 });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_cover_all_frames() {
        let buf = SourceBuffer {
            channels: vec![
                vec![0.1, -0.5, 0.2, 0.9, -0.1],
                vec![0.0, 0.0, -0.7, 0.0, 0.0],
            ],
        };
        assert_eq!(
            peaks(&buf, 2),
            vec![-0.5, 0.1, -0.7, 0.9, -0.1, 0.0],
            "último bloco: L=-0,1 e R=0,0"
        );
    }

    #[test]
    fn normalize_hits_target() {
        let mut l = vec![0.25, -0.5];
        let mut r = vec![0.1, 0.0];
        normalize(&mut l, &mut r, 0.9);
        assert!((l[1] + 0.9).abs() < 1e-6);
        let (mut z1, mut z2) = (vec![0.0; 4], vec![0.0; 4]);
        normalize(&mut z1, &mut z2, 0.9);
        assert!(z1.iter().all(|x| *x == 0.0), "silêncio não explode");
    }

    #[test]
    fn master_analysis_of_reference_sine() {
        // seno de 997 Hz, 0 dBFS de pico nos dois canais: −3,01 LKFS por canal → +3 dB somando 2 canais ≈ 0 LUFS
        let n = 48_000 * 3;
        let s: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * 997.0 * i as f32 / 48_000.0).sin() * 0.5)
            .collect();
        let a = analyze_master(&s, &s, 48_000);
        assert!((a.peak_db + 6.02).abs() < 0.05);
        assert!((a.correlation - 1.0).abs() < 1e-3);
        assert!(
            (a.integrated_lufs - (-6.02)).abs() < 0.3,
            "obtido {}",
            a.integrated_lufs
        );
    }
}
