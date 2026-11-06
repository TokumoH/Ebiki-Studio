//! Fase 3: processamento não destrutivo do clipe na engine (ganho, fades, polaridade, reverso).
//! Render offline com o mesmo núcleo do tempo real; compara amostra a amostra.
mod common;
use audio_engine::*;
use common::*;
use std::sync::Arc;

fn render_clips(clips: Vec<ClipRender>, frames: usize) -> Vec<f32> {
    let mut g = RenderGraph::empty(SR);
    g.tracks.push(track(1, 1, clips));
    let (_h, mut core) = playing(g);
    let (l, r) = offline::render(&mut core, frames, 333); // bloco ímpar: fades cruzam blocos
    assert_eq!(l, r, "fonte mono duplicada");
    l
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-4 * b.abs().max(1.0)
}

#[test]
fn source_buffer_is_never_modified() {
    let src = ramp(1000);
    let before = (*src).clone();
    let mut c = ClipRender::new(src.clone(), 0, 0, 1000, -0.5);
    c.reverse = true;
    c.fade_in = 100;
    c.invert = true;
    render_clips(vec![c], 1200);
    assert_eq!(*src, before, "RNF-07: a fonte permanece intacta");
}

#[test]
fn clip_gain_scales_samples() {
    let out = render_clips(
        vec![ClipRender::new(dc(0.5, 1000), 10, 0, 1000, 0.25)],
        1100,
    );
    assert_eq!(out[9], 0.0);
    assert!(close(out[10], 0.125));
    assert!(close(out[1009], 0.125));
    assert_eq!(out[1010], 0.0);
}

#[test]
fn inverted_polarity_negates_every_sample() {
    let src = sine(440.0, 0.5, 2000);
    let normal = render_clips(vec![ClipRender::new(src.clone(), 0, 0, 2000, 1.0)], 2000);
    let mut c = ClipRender::new(src, 0, 0, 2000, 1.0);
    c.invert = true;
    let inv = render_clips(vec![c], 2000);
    for (a, b) in normal.iter().zip(&inv) {
        assert_eq!(*a, -*b);
    }
    assert!(normal.iter().any(|x| x.abs() > 0.4));
}

#[test]
fn reversed_reads_region_backwards() {
    // região [100, 600) da rampa, tocada ao contrário a partir da amostra 50
    let mut c = ClipRender::new(ramp(1000), 50, 100, 500, 1.0);
    c.reverse = true;
    let out = render_clips(vec![c], 700);
    assert_eq!(out[49], 0.0);
    assert_eq!(out[50], 599.0, "começa no fim da região");
    assert_eq!(out[51], 598.0);
    assert_eq!(out[549], 100.0, "termina no início da região");
    assert_eq!(out[550], 0.0);
}

#[test]
fn linear_fades_ramp_from_and_to_silence() {
    let mut c = ClipRender::new(dc(1.0, 1000), 0, 0, 1000, 1.0);
    c.fade_in = 100;
    c.fade_out = 200;
    let out = render_clips(vec![c], 1000);
    assert_eq!(out[0], 0.0, "fade de entrada começa em silêncio");
    assert!(close(out[50], 0.5));
    assert!(close(out[100], 1.0));
    assert!(close(out[500], 1.0));
    assert!(close(out[800], 1.0), "início do fade de saída");
    assert!(close(out[900], 0.5));
    assert!(
        close(out[999], 1.0 / 200.0),
        "última amostra quase em silêncio"
    );
    // monotônico nas rampas
    assert!(out[..100].windows(2).all(|w| w[0] <= w[1]));
    assert!(out[800..].windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn equal_power_fade_follows_sine_curve() {
    let mut c = ClipRender::new(dc(1.0, 1000), 0, 0, 1000, 1.0);
    c.fade_in = 100;
    c.fade_in_shape = FadeShape::EqualPower;
    let out = render_clips(vec![c], 1000);
    assert!(close(out[50], std::f32::consts::FRAC_1_SQRT_2));
    assert!(out[25] > 0.25, "equal power sobe mais rápido que o linear");
}

#[test]
fn linear_crossfade_keeps_constant_amplitude() {
    let mut a = ClipRender::new(dc(1.0, 1000), 0, 0, 1000, 1.0);
    let mut b = ClipRender::new(dc(1.0, 1000), 800, 0, 1000, 1.0);
    a.fade_out = 200;
    b.fade_in = 200;
    let out = render_clips(vec![a, b], 1800);
    for (i, x) in out.iter().enumerate() {
        assert!(close(*x, 1.0), "amostra {i}: {x}");
    }
}

#[test]
fn fades_combine_with_reverse_and_gain() {
    let mut c = ClipRender::new(ramp(1000), 0, 0, 1000, 0.5);
    c.reverse = true;
    c.fade_in = 10;
    let out = render_clips(vec![c], 1000);
    assert_eq!(out[0], 0.0);
    assert!(close(out[5], (999.0 - 5.0) * 0.5 * 0.5));
    assert!(close(out[500], (999.0 - 500.0) * 0.5));
}

#[test]
fn split_clips_render_identically_to_the_original() {
    let src = sine(330.0, 0.7, 5000);
    let whole = render_clips(
        vec![ClipRender::new(src.clone(), 100, 200, 4000, 0.8)],
        4300,
    );
    // dividido em 1777: [200, 1977) + [1977, 4200)
    let a = ClipRender::new(src.clone(), 100, 200, 1777, 0.8);
    let b = ClipRender::new(src.clone(), 1877, 1977, 4000 - 1777, 0.8);
    let split = render_clips(vec![a, b], 4300);
    assert_eq!(whole, split, "dividir não altera o áudio");

    // o mesmo vale para um clipe reverso: a primeira parte toca o fim da região
    let mut w = ClipRender::new(src.clone(), 0, 200, 4000, 1.0);
    w.reverse = true;
    let whole = render_clips(vec![w], 4100);
    let mut a = ClipRender::new(src.clone(), 0, 200 + 4000 - 1000, 1000, 1.0);
    let mut b = ClipRender::new(src, 1000, 200, 3000, 1.0);
    a.reverse = true;
    b.reverse = true;
    assert_eq!(whole, render_clips(vec![a, b], 4100));
}

#[test]
fn huge_start_does_not_overflow() {
    let c = ClipRender::new(
        Arc::new(SourceBuffer {
            channels: vec![vec![1.0; 10]],
        }),
        u64::MAX - 5,
        0,
        10,
        1.0,
    );
    assert_eq!(c.end(), u64::MAX);
    let mut g = RenderGraph::empty(SR);
    g.tracks.push(track(1, 1, vec![c]));
    assert_eq!(g.content_end(), u64::MAX);
    let (_h, mut core) = playing(g);
    let (l, _) = offline::render(&mut core, 64, 64);
    assert!(l.iter().all(|x| *x == 0.0));
}
