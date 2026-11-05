mod common;
use audio_engine::*;
use common::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[test]
fn two_tracks_are_sample_accurate() {
    // R2: impulsos em faixas diferentes, posicionados em 1000 e 1000+480 amostras
    let mut g = RenderGraph::empty(SR);
    g.tracks.push(track(1, 1, vec![clip(impulse(0, 10), 1000)]));
    g.tracks.push(track(2, 2, vec![clip(impulse(0, 10), 1480)]));
    let (_h, mut core) = playing(g);
    let (l, r) = offline::render(&mut core, 4000, 333); // bloco ímpar força divisões
    let hits: Vec<usize> = l
        .iter()
        .enumerate()
        .filter(|(_, x)| x.abs() > 0.5)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(hits, vec![1000, 1480]);
    assert_eq!(l, r);
}

#[test]
fn fader_and_balance_values() {
    let mut g = RenderGraph::empty(SR);
    let mut t = track(1, 1, vec![clip(dc(1.0, 20_000), 0)]);
    t.strip.volume = shared_kernel::Db(-6.0).to_linear();
    t.strip.pan = -1.0;
    g.tracks.push(t);
    let (_h, mut core) = playing(g);
    let (l, r) = offline::render(&mut core, 4096, 256);
    assert!(
        (l[2000] - 0.501_187).abs() < 1e-3,
        "−6 dB = 0,501, obtido {}",
        l[2000]
    );
    assert!(r[2000].abs() < 1e-6, "pan à esquerda zera a direita");
}

#[test]
fn inaudible_strip_is_silent_and_buses_sum() {
    let mut g = RenderGraph::empty(SR);
    let mut muted = track(1, 1, vec![clip(dc(1.0, 4096), 0)]);
    muted.strip.audible = false;
    let mut to_bus = track(2, 2, vec![clip(dc(0.25, 4096), 0)]);
    to_bus.strip.dest = Destination::Bus(0);
    to_bus.strip.sends.push(SendRender {
        bus: 1,
        gain: 0.5,
        pre_fader: false,
    });
    g.tracks.push(muted);
    g.tracks.push(to_bus);
    g.buses.push(StripRender::new(10, 3));
    g.buses.push(StripRender::new(11, 4));
    let (h, mut core) = playing(g);
    let (l, _) = offline::render(&mut core, 2048, 512);
    // 0,25 pelo bus 0 + 0,25*0,5 pelo envio ao bus 1 = 0,375
    assert!((l[1000] - 0.375).abs() < 1e-6, "obtido {}", l[1000]);
    let snap = h.snapshot();
    assert_eq!(
        snap.peaks[1],
        [0.0, 0.0],
        "faixa muda não pontua no medidor"
    );
    assert!((snap.peaks[0][0] - 0.375).abs() < 1e-6, "master");
}

#[test]
fn loop_wraps_playback() {
    let mut g = RenderGraph::empty(SR);
    g.tracks
        .push(track(1, 1, vec![clip(impulse(100, 1000), 0)]));
    g.loop_range = Some((0, 500));
    let (_h, mut core) = playing(g);
    let (l, _) = offline::render(&mut core, 1600, 128);
    let hits: Vec<usize> = l
        .iter()
        .enumerate()
        .filter(|(_, x)| x.abs() > 0.5)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        hits,
        vec![100, 600, 1100, 1600]
            .into_iter()
            .filter(|i| *i < 1600)
            .collect::<Vec<_>>()
    );
}

#[test]
fn processor_state_is_inherited_across_graph_swaps() {
    let processed = Arc::new(AtomicU64::new(0));
    let build = |gain: f32, with_processor: bool| {
        let mut g = RenderGraph::empty(SR);
        let mut t = track(7, 1, vec![clip(dc(1.0, 100_000), 0)]);
        t.strip.inserts.push(InsertRender {
            id: 42,
            type_tag: 1,
            bypass: false,
            params: vec![gain],
            processor: with_processor.then(|| {
                Box::new(Gain {
                    g: 1.0,
                    processed: processed.clone(),
                }) as Box<dyn AudioProcessor>
            }),
        });
        g.tracks.push(t);
        g
    };
    let (mut h, mut core) = playing(build(0.5, true));
    let (l, _) = offline::render(&mut core, 1024, 256);
    assert!((l[512] - 0.5).abs() < 1e-6);
    h.swap_graph(build(0.25, false)).unwrap(); // sem processador: deve herdar
    let (l, _) = offline::render(&mut core, 1024, 256);
    assert!(
        (l[512] - 0.25).abs() < 1e-6,
        "parâmetro novo aplicado no processador herdado"
    );
    assert_eq!(
        processed.load(Ordering::Relaxed),
        2048,
        "mesmo processador (estado contínuo)"
    );
    assert!(
        h.collect_garbage() >= 1,
        "grafo antigo devolvido para liberar fora do tempo real"
    );
}

#[test]
fn nan_from_effect_is_sanitized() {
    struct Bad;
    impl AudioProcessor for Bad {
        fn set_params(&mut self, _: &[f32]) {}
        fn process(&mut self, l: &mut [f32], _: &mut [f32]) {
            l.fill(f32::NAN);
        }
        fn reset(&mut self) {}
    }
    let mut g = RenderGraph::empty(SR);
    let mut t = track(1, 1, vec![clip(dc(1.0, 4096), 0)]);
    t.strip.inserts.push(InsertRender {
        id: 1,
        type_tag: 9,
        bypass: false,
        params: vec![],
        processor: Some(Box::new(Bad)),
    });
    g.tracks.push(t);
    let (_h, mut core) = playing(g);
    let (l, r) = offline::render(&mut core, 1024, 256);
    assert!(l.iter().chain(&r).all(|x| x.is_finite()));
}

#[test]
fn invalid_graph_is_rejected() {
    let (mut h, _core) = create_engine(EngineConfig::default());
    let mut g = RenderGraph::empty(SR);
    let mut t = track(1, 1, vec![]);
    t.strip.dest = Destination::Bus(3);
    g.tracks.push(t);
    assert!(h.swap_graph(g).is_err());
    let mut g = RenderGraph::empty(SR);
    let src = dc(1.0, 10);
    g.tracks.push(track(
        1,
        1,
        vec![ClipRender {
            source: src,
            start: 0,
            offset: 5,
            length: 10,
            gain: 1.0,
        }],
    ));
    assert!(h.swap_graph(g).is_err(), "clipe além do fim da fonte");
}

#[test]
fn metronome_clicks_on_beats() {
    let mut g = RenderGraph::empty(SR); // 120 BPM: 1 tempo = 24000 amostras
    g.metronome = MetronomeSettings {
        enabled: true,
        volume: 1.0,
        ..Default::default()
    };
    let (_h, mut core) = playing(g);
    let (l, _) = offline::render(&mut core, 24_000 * 4 + 100, 500);
    // início = primeira amostra não nula após ≥ 200 amostras de silêncio
    let mut onsets = Vec::new();
    let mut last_sound: Option<usize> = None;
    for (i, x) in l.iter().enumerate() {
        if x.abs() > 1e-6 {
            if last_sound.is_none_or(|p| i - p > 200) {
                onsets.push(i);
            }
            last_sound = Some(i);
        }
    }
    assert_eq!(
        onsets.len(),
        5,
        "tempos 0,1,2,3 e o 1º do compasso seguinte: {onsets:?}"
    );
    for (k, o) in onsets.iter().enumerate() {
        assert!(
            (*o as i64 - (k as i64 * 24_000)).abs() <= 2,
            "clique {k} em {o}"
        );
    }
}

#[test]
fn stop_returns_position_and_seek_moves() {
    let (mut h, mut core) = playing(RenderGraph::empty(SR));
    offline::render(&mut core, 1000, 100);
    assert_eq!(core.position(), 1000);
    h.stop(Some(0)).unwrap();
    core.process(&mut [0.0; 2], 2, &[], 0);
    assert_eq!(core.position(), 0);
    assert_eq!(core.state(), TransportState::Stopped);
    h.seek(5000).unwrap();
    core.process(&mut [0.0; 2], 2, &[], 0);
    assert_eq!(core.position(), 5000);
    assert_eq!(h.telemetry().position(), 5000);
}
