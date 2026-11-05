//! Orçamento de CPU: 32 faixas com a cadeia completa de efeitos + master, bloco de 128 amostras
//! a 48 kHz (2,67 ms de áudio). Em release o custo médio por bloco deve ficar bem abaixo do
//! tempo real (≤ 50 % do orçamento, deixando folga para decodificação, mixagem e o SO).
//! Em debug o teste só executa (sem asserção de tempo).
//!
//! Rodar com medição: `cargo test -p dsp --release --test performance -- --nocapture`.

use dsp::*;
use std::time::Instant;

const SR: f32 = 48_000.0;
const BLOCK: usize = 128;
const TRACKS: usize = 32;

struct Track {
    chain: Vec<Box<dyn Processor + Send>>,
    l: Vec<f32>,
    r: Vec<f32>,
    src_l: Vec<f32>,
    src_r: Vec<f32>,
}

fn track_chain(i: usize) -> Vec<Box<dyn Processor + Send>> {
    let mut eq = Eq::new();
    eq.set_bands(&[
        EqBandParams {
            kind: FilterKind::HighPass,
            enabled: true,
            freq_hz: 80.0,
            gain_db: 0.0,
            q: 0.707,
        },
        EqBandParams {
            kind: FilterKind::LowShelf,
            enabled: true,
            freq_hz: 150.0,
            gain_db: 2.0,
            q: 0.707,
        },
        EqBandParams {
            kind: FilterKind::Peak,
            enabled: true,
            freq_hz: 1_000.0 + i as f32 * 50.0,
            gain_db: -3.0,
            q: 1.0,
        },
        EqBandParams {
            kind: FilterKind::HighShelf,
            enabled: true,
            freq_hz: 8_000.0,
            gain_db: 2.0,
            q: 0.707,
        },
    ]);
    let mut comp = Compressor::new();
    comp.set_params(CompressorParams::default());
    let mut chain: Vec<Box<dyn Processor + Send>> = vec![
        Box::new(Gate::new()),
        Box::new(eq),
        Box::new(comp),
        Box::new(DeEsser::new()),
        Box::new(Saturation::new()),
        Box::new(Chorus::new()),
        Box::new(Delay::new()),
        Box::new(Reverb::new()),
        Box::new(Panner::new()),
        Box::new(PeakMeter::default()),
        Box::new(RmsMeter::default()),
    ];
    for p in &mut chain {
        p.prepare(SR, BLOCK);
    }
    chain
}

#[test]
fn thirty_two_tracks_full_chain_fit_in_block_budget() {
    let mut tracks: Vec<Track> = (0..TRACKS)
        .map(|i| Track {
            chain: track_chain(i),
            l: vec![0.0; BLOCK],
            r: vec![0.0; BLOCK],
            src_l: signals::noise(BLOCK, 0.3, i as u32 + 1),
            src_r: signals::sine(110.0 * (i + 1) as f32, 0.3, SR, BLOCK, 0.0),
        })
        .collect();
    let mut master: Vec<Box<dyn Processor + Send>> = vec![
        Box::new(Eq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(LoudnessMeter::new()),
        Box::new(TruePeakMeter::new()),
        Box::new(CorrelationMeter::new()),
        Box::new(PeakMeter::default()),
        Box::new(ClipCounter::default()),
    ];
    for p in &mut master {
        p.prepare(SR, BLOCK);
    }
    let mut bus_l = vec![0.0f32; BLOCK];
    let mut bus_r = vec![0.0f32; BLOCK];

    let mut render_block = |tracks: &mut [Track], bus_l: &mut [f32], bus_r: &mut [f32]| {
        bus_l.iter_mut().for_each(|v| *v = 0.0);
        bus_r.iter_mut().for_each(|v| *v = 0.0);
        for t in tracks.iter_mut() {
            t.l.copy_from_slice(&t.src_l);
            t.r.copy_from_slice(&t.src_r);
            for p in t.chain.iter_mut() {
                p.process(&mut t.l, &mut t.r);
            }
            for i in 0..BLOCK {
                bus_l[i] += t.l[i];
                bus_r[i] += t.r[i];
            }
        }
        for p in master.iter_mut() {
            p.process(bus_l, bus_r);
        }
    };

    // Aquecimento.
    for _ in 0..50 {
        render_block(&mut tracks, &mut bus_l, &mut bus_r);
    }
    let blocks = 750; // 2 s de áudio
    let start = Instant::now();
    for _ in 0..blocks {
        render_block(&mut tracks, &mut bus_l, &mut bus_r);
    }
    let per_block = start.elapsed().as_secs_f64() / blocks as f64;
    let budget = BLOCK as f64 / SR as f64;
    let ratio = per_block / budget;
    println!(
        "32 faixas × cadeia completa: {:.1} µs/bloco de {:.0} µs → {:.1} % do orçamento",
        per_block * 1e6,
        budget * 1e6,
        ratio * 100.0
    );
    assert!(bus_l.iter().chain(bus_r.iter()).all(|v| v.is_finite()));
    if cfg!(debug_assertions) {
        return; // tempos de debug não são representativos
    }
    assert!(
        ratio < 0.5,
        "custo {:.1} % do orçamento de tempo real",
        ratio * 100.0
    );
}

/// Relatório (sem asserção de tempo) do custo de cada processador por bloco — orienta
/// otimizações e alimenta a tabela da spec da Fase 2.
#[test]
fn per_processor_cost_report() {
    let names = [
        "Gate",
        "Eq(4)",
        "Compressor",
        "DeEsser",
        "Saturation",
        "Chorus",
        "Delay",
        "Reverb",
        "Panner",
        "PeakMeter",
        "RmsMeter",
    ];
    let mut chain = track_chain(0);
    let src_l = signals::noise(BLOCK, 0.3, 1);
    let src_r = signals::sine(220.0, 0.3, SR, BLOCK, 0.0);
    let mut extra: Vec<(&str, Box<dyn Processor + Send>)> = vec![
        ("Limiter", Box::new(Limiter::new())),
        ("LoudnessMeter", Box::new(LoudnessMeter::new())),
        ("TruePeakMeter", Box::new(TruePeakMeter::new())),
    ];
    for (_, p) in &mut extra {
        p.prepare(SR, BLOCK);
    }
    let all = chain
        .iter_mut()
        .zip(names)
        .map(|(p, n)| (n, p))
        .chain(extra.iter_mut().map(|(n, p)| (*n, p)));
    for (name, p) in all {
        let (mut l, mut r) = (src_l.clone(), src_r.clone());
        let iters = 2_000;
        let start = Instant::now();
        for _ in 0..iters {
            l.copy_from_slice(&src_l);
            r.copy_from_slice(&src_r);
            p.process(&mut l, &mut r);
        }
        let us = start.elapsed().as_secs_f64() * 1e6 / iters as f64;
        println!("{name:>14}: {us:6.2} µs/bloco de {BLOCK}");
        assert!(l.iter().all(|v| v.is_finite()));
    }
}
