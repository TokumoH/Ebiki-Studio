mod common;
use audio_engine::recorder::*;
use audio_engine::*;
use common::*;

fn input_sine(frames: usize) -> Vec<f32> {
    // 2 canais de entrada intercalados: canal 0 = seno, canal 1 = 0,5 constante
    let mut v = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        v.push((std::f32::consts::TAU * 440.0 * i as f32 / SR as f32).sin() * 0.5);
        v.push(0.5);
    }
    v
}

#[test]
fn records_punch_range_to_wav_with_start_frame() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audio/recorded/take1.wav");
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    let mut g = RenderGraph::empty(SR);
    g.punch = Some((1_000, 3_000));
    h.swap_graph(g).unwrap();
    let (rt, session) = RecordingSession::start(
        &[TapConfig {
            key: 1,
            left: 0,
            right: Some(1),
            path: path.clone(),
        }],
        SR,
        2,
    )
    .unwrap();
    h.attach_recorder(rt).unwrap();
    h.record(0, false).unwrap();
    let input = input_sine(5_000);
    offline::render_with_input(&mut core, &input, 2, 256);
    assert_eq!(h.snapshot().record_start, Some(1_000));
    h.stop(None).unwrap();
    h.detach_recorder().unwrap();
    core.process(&mut [0.0; 2], 2, &[], 0);
    h.collect_garbage();
    let files = session.finish().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].frames, 2_000, "somente a janela de punch");
    let (sr, ch) = read_wav_f32(&path).unwrap();
    assert_eq!(sr, SR);
    assert_eq!(ch.len(), 2);
    // primeira amostra gravada = amostra 1000 da entrada
    assert!((ch[0][0] - input[2_000]).abs() < 1e-6);
    assert!(ch[1].iter().all(|x| (*x - 0.5).abs() < 1e-6));
}

#[test]
fn count_in_delays_recording_and_holds_position() {
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    let mut g = RenderGraph::empty(SR); // 120 BPM 4/4 → 1 compasso = 96000 amostras
    g.metronome = MetronomeSettings {
        count_in_bars: 1,
        ..Default::default()
    };
    h.swap_graph(g).unwrap();
    h.record(48_000, true).unwrap();
    offline::render(&mut core, 50_000, 512);
    assert_eq!(core.state(), TransportState::CountIn);
    assert_eq!(core.position(), 48_000, "timeline parada na contagem");
    offline::render(&mut core, 50_000, 512);
    assert_eq!(core.state(), TransportState::Recording);
    assert_eq!(
        core.position(),
        48_000 + 4_000,
        "avança só o que passou da contagem"
    );
    assert_eq!(h.snapshot().record_start, Some(48_000));
}

#[test]
fn input_monitoring_passes_through_strip() {
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    let mut g = RenderGraph::empty(SR);
    let mut t = track(1, 1, vec![]);
    t.monitor = Some(InputMonitor {
        left: 1,
        right: None,
    });
    t.strip.volume = 0.5;
    g.tracks.push(t);
    h.swap_graph(g).unwrap();
    let input = input_sine(1_000);
    let (l, r) = offline::render_with_input(&mut core, &input, 2, 128);
    assert!(
        (l[500] - 0.25).abs() < 1e-6 && (r[500] - 0.25).abs() < 1e-6,
        "monitoração mesmo parado"
    );
}
