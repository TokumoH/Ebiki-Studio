//! R1: o núcleo não pode alocar nem liberar memória no callback.
mod common;
use assert_no_alloc::*;
use audio_engine::*;
use common::*;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

/// Metade das faixas com fades/reverso/polaridade (Fase 3): o caminho por amostra também não aloca.
fn edited_clip(k: u128, src: Arc<SourceBuffer>, start: u64) -> ClipRender {
    let mut c = clip(src, start);
    if k % 2 == 0 {
        c.fade_in = 5_000;
        c.fade_out = 5_000;
        c.fade_in_shape = FadeShape::EqualPower;
        c.reverse = k % 4 == 0;
        c.invert = true;
    }
    c
}

#[test]
fn process_does_not_allocate() {
    let mut g = RenderGraph::empty(SR);
    for k in 0..32u128 {
        let mut t = track(
            k + 1,
            k as usize + 1,
            vec![edited_clip(k, sine(220.0, 0.1, 200_000), (k as u64) * 100)],
        );
        t.strip.inserts.push(InsertRender {
            id: k,
            type_tag: 1,
            bypass: false,
            params: vec![0.8],
            processor: Some(Box::new(Gain {
                g: 1.0,
                processed: Arc::new(AtomicU64::new(0)),
            })),
        });
        t.strip.sends.push(SendRender {
            bus: 0,
            gain: 0.2,
            pre_fader: false,
        });
        g.tracks.push(t);
    }
    g.buses.push(StripRender::new(1000, 40));
    g.metronome.enabled = true;
    g.loop_range = Some((0, 30_000));
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    h.swap_graph(g).unwrap();
    h.play().unwrap();
    // derivação de entrada do afinador ligada (fila menor que o total: exercita o descarte)
    let mut tap = h.attach_input_tap(1, 48_000).unwrap();
    let mut out = vec![0.0f32; 2048 * 2];
    let input = vec![0.1f32; 2048 * 2];
    // a primeira chamada troca o grafo: o grafo antigo vai para a fila de lixo, sem liberar
    reset_violation_count();
    assert_no_alloc(|| {
        for _ in 0..200 {
            core.process(&mut out, 2, &input, 2);
        }
    });
    assert_eq!(violation_count(), 0, "o núcleo alocou memória no callback");
    let mut tapped = Vec::new();
    assert_eq!(
        tap.drain(&mut tapped),
        48_000,
        "derivação cheia, excedente descartado"
    );
    assert!(tapped.iter().all(|x| (*x - 0.1).abs() < 1e-9));
    // troca/desliga a derivação também sem alocar no núcleo
    h.detach_input_tap().unwrap();
    reset_violation_count();
    assert_no_alloc(|| {
        core.process(&mut out, 2, &input, 2);
    });
    assert_eq!(
        violation_count(),
        0,
        "desligar a derivação não libera memória no núcleo"
    );
    h.collect_garbage();
}

#[test]
fn guard_detects_allocation() {
    reset_violation_count();
    assert_no_alloc(|| {
        let v = std::hint::black_box(vec![1u8; 16]);
        drop(v);
    });
    assert!(violation_count() > 0, "o detector precisa acusar alocações");
    reset_violation_count();
}
