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
    if k.is_multiple_of(2) {
        c.fade_in = 5_000;
        c.fade_out = 5_000;
        c.fade_in_shape = FadeShape::EqualPower;
        c.reverse = k.is_multiple_of(4);
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
    // "Ouvir entrada" ligado e medidores de entrada: também sem alocar
    h.set_device_monitor(Some(DeviceMonitor {
        left: 0,
        right: Some(1),
        gain: 0.5,
    }))
    .unwrap();
    // ganho de entrada (rampa no rascunho pré-alocado) e tom de teste: também sem alocar
    h.set_input_gain_db(12.0).unwrap();
    h.play_test_tone(5.0, -12.0).unwrap();
    // prévia (audição) longa: tocando durante todos os blocos medidos (S-3)
    h.play_preview(audio_engine::preview::PreviewClip::from_planar(&[
        vec![0.1; 48_000 * 10],
    ]))
    .unwrap();
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
    // a derivação recebe a entrada já amplificada (+12 dB, depois da rampa de ~10 ms)
    let gained = 0.1 * audio_engine::input_gain_linear(12.0);
    assert!((tapped[tapped.len() - 1] - gained).abs() < 1e-5);
    assert!(tapped
        .iter()
        .all(|x| *x >= 0.1 - 1e-7 && *x <= gained + 1e-5));
    // troca/desliga a derivação também sem alocar no núcleo
    h.detach_input_tap().unwrap();
    h.set_device_monitor(None).unwrap();
    h.set_input_gain_db(0.0).unwrap();
    h.play_test_tone(0.1, -6.0).unwrap();
    // troca e parada da prévia: o trecho antigo vai para o lixo sem alocar no callback
    h.play_preview(audio_engine::preview::PreviewClip::from_planar(&[vec![
        0.2;
        64
    ]]))
    .unwrap();
    h.stop_preview().unwrap();
    reset_violation_count();
    assert_no_alloc(|| {
        core.process(&mut out, 2, &input, 2);
    });
    assert_eq!(
        violation_count(),
        0,
        "desligar a derivação e mudar o ganho/tom não aloca nem libera memória no núcleo"
    );
    h.collect_garbage();
}

/// S-1: o conversor de taxa (fone/microfone em outra taxa) também roda no callback sem alocar.
#[test]
fn rate_conversion_does_not_allocate() {
    use audio_engine::resampler::{InputRateAdapter, OutputRateAdapter};
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    h.play().unwrap();
    h.play_test_tone(5.0, -12.0).unwrap();
    let mut out_a = OutputRateAdapter::new(2, SR, 44_100, 4096);
    let mut in_a = InputRateAdapter::new(2, 16_000, SR, 4096);
    let mut dev = vec![0.0f32; 4096 * 2];
    let mic = vec![0.1f32; 4096 * 2];
    let mut received = 0usize;
    reset_violation_count();
    assert_no_alloc(|| {
        for n in [4096usize, 1, 441, 512, 7, 2048, 4096, 333]
            .iter()
            .cycle()
            .take(400)
        {
            out_a.fill(&mut dev[..n * 2], |eng: &mut [f32]| {
                core.process(eng, 2, &[], 0)
            });
            in_a.process(&mic[..n * 2], |x| received += x.len());
        }
    });
    assert_eq!(
        violation_count(),
        0,
        "o conversor alocou memória no callback"
    );
    assert!(received > 0);
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
