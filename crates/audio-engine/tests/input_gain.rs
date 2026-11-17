//! Ganho de entrada (antes de gravação, medidores, afinador e monitores) e tom de teste da saída.
mod common;
use audio_engine::recorder::*;
use audio_engine::*;
use common::*;

/// +12 dB sobre 0,1 = 0,398.
const GAINED: f32 = 0.398_107;

fn engine() -> (EngineHandle, EngineCore) {
    let (mut h, core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    h.swap_graph(RenderGraph::empty(SR)).unwrap();
    (h, core)
}

/// Aplica o ganho e deixa a rampa (~10 ms) terminar.
fn with_gain(h: &mut EngineHandle, core: &mut EngineCore, db: f32, in_ch: usize) {
    h.set_input_gain_db(db).unwrap();
    let input = vec![0.0f32; 2_048 * in_ch];
    let mut out = vec![0.0f32; 2_048 * 2];
    core.process(&mut out, 2, &input, in_ch);
    h.snapshot(); // zera os picos do assentamento
}

#[test]
fn gain_reaches_meters_tap_and_device_monitor() {
    let (mut h, mut core) = engine();
    with_gain(&mut h, &mut core, 12.0, 2);
    let mut tap = h.attach_input_tap(0, 4_096).unwrap();
    h.set_device_monitor(Some(DeviceMonitor {
        left: 0,
        right: Some(1),
        gain: 1.0,
    }))
    .unwrap();
    let input: Vec<f32> = (0..2_048).flat_map(|_| [0.1f32, -0.1]).collect();
    let mut out = vec![0.0f32; 2_048 * 2];
    for _ in 0..2 {
        core.process(&mut out, 2, &input, 2);
    }
    let peaks = h.snapshot().input_peaks;
    assert!(
        (peaks[0] - GAINED).abs() < 1e-4 && (peaks[1] - GAINED).abs() < 1e-4,
        "{peaks:?}"
    );
    let mut tapped = Vec::new();
    tap.drain(&mut tapped);
    assert!(
        (tapped[tapped.len() - 1] - GAINED).abs() < 1e-4,
        "afinador recebe o sinal amplificado"
    );
    let last = &out[out.len() - 2..];
    assert!(
        (last[0] - GAINED).abs() < 1e-4 && (last[1] + GAINED).abs() < 1e-4,
        "monitor: {last:?}"
    );
}

#[test]
fn gain_reaches_the_recorded_take() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("take.wav");
    let (mut h, mut core) = engine();
    with_gain(&mut h, &mut core, 12.0, 1);
    let (rt, session) = RecordingSession::start(
        &[TapConfig {
            key: 1,
            left: 0,
            right: None,
            path: path.clone(),
        }],
        SR,
        1,
    )
    .unwrap();
    h.attach_recorder(rt).unwrap();
    h.record(0, false).unwrap();
    offline::render_with_input(&mut core, &vec![0.1f32; 4_800], 1, 256);
    h.stop(None).unwrap();
    h.detach_recorder().unwrap();
    core.process(&mut [0.0; 2], 2, &[], 0);
    h.collect_garbage();
    let (files, errors) = session.finish();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(files[0].frames, 4_800);
    let (_, ch) = read_wav_f32(&path).unwrap();
    assert!(
        ch[0].iter().all(|x| (*x - GAINED).abs() < 1e-4),
        "gravado com +12 dB"
    );
}

#[test]
fn gain_reaches_track_input_monitoring() {
    let mut g = RenderGraph::empty(SR);
    let mut t = track(1, 1, vec![]);
    t.monitor = Some(InputMonitor {
        left: 0,
        right: None,
    });
    g.tracks.push(t);
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    h.swap_graph(g).unwrap();
    with_gain(&mut h, &mut core, 12.0, 1);
    let (l, r) = offline::render_with_input(&mut core, &vec![0.1f32; 2_048], 1, 512);
    assert!(
        (l[2_047] - GAINED).abs() < 1e-3 && (r[2_047] - GAINED).abs() < 1e-3,
        "{} {}",
        l[2_047],
        r[2_047]
    );
}

#[test]
fn zero_db_is_transparent_and_gain_changes_ramp_without_clicks() {
    let (mut h, mut core) = engine();
    let mut tap = h.attach_input_tap(0, 8_192).unwrap();
    let input = vec![0.1f32; 2_048];
    let mut out = vec![0.0f32; 2_048 * 2];
    core.process(&mut out, 2, &input, 1);
    let mut v = Vec::new();
    tap.drain(&mut v);
    assert!(v.iter().all(|x| *x == 0.1), "0 dB não altera a entrada");
    h.set_input_gain_db(24.0).unwrap();
    core.process(&mut out, 2, &input, 1);
    v.clear();
    tap.drain(&mut v);
    let jump = v
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max);
    assert!(jump < 0.005, "rampa por amostra, salto {jump}");
    assert!(
        (v[v.len() - 1] - 1.5849).abs() < 1e-3,
        "+24 dB: {}",
        v[v.len() - 1]
    );
    // fora da faixa e NaN: limitado / 0 dB
    h.set_input_gain_db(60.0).unwrap();
    for _ in 0..2 {
        core.process(&mut out, 2, &input, 1);
    }
    v.clear();
    tap.drain(&mut v);
    assert!((v[v.len() - 1] - 1.5849).abs() < 1e-3, "limitado a +24 dB");
    h.set_input_gain_db(f32::NAN).unwrap();
    for _ in 0..2 {
        core.process(&mut out, 2, &input, 1);
    }
    v.clear();
    tap.drain(&mut v);
    assert!((v[v.len() - 1] - 0.1).abs() < 1e-6, "NaN = 0 dB");
}

#[test]
fn many_input_channels_are_processed_in_smaller_blocks() {
    // 80 canais > 64: blocos de 819 quadros; o último canal também recebe o ganho
    let (mut h, mut core) = engine();
    with_gain(&mut h, &mut core, 12.0, 80);
    let mut tap = h.attach_input_tap(79, 8_192).unwrap();
    let input = vec![0.1f32; 3_000 * 80];
    let mut out = vec![0.0f32; 3_000 * 2];
    core.process(&mut out, 2, &input, 80);
    let mut v = Vec::new();
    assert_eq!(tap.drain(&mut v), 3_000);
    assert!(v.iter().all(|x| (*x - GAINED).abs() < 1e-4));
}

#[test]
fn test_tone_plays_at_the_requested_level_and_duration() {
    let (mut h, mut core) = engine();
    h.play_test_tone(0.5, -12.0).unwrap();
    let (l, r) = offline::render(&mut core, 48_000, 512);
    let peak = l.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    let expected = 10f32.powf(-12.0 / 20.0); // 0,251
    assert!(
        (peak - expected).abs() < 0.005,
        "pico {peak}, esperado {expected}"
    );
    assert_eq!(l, r);
    let last = l.iter().rposition(|v| *v != 0.0).unwrap();
    assert!((23_900..24_000).contains(&last), "dura 0,5 s: {last}");
    assert!(
        l[0].abs() < 0.01 && l[23_990].abs() < 0.01,
        "rampas sem estalo"
    );
    assert_eq!(
        h.snapshot().peaks[0],
        [0.0, 0.0],
        "o tom não passa pelo master"
    );
}

#[test]
fn test_tone_is_not_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("take.wav");
    let (mut h, mut core) = engine();
    let (rt, session) = RecordingSession::start(
        &[TapConfig {
            key: 1,
            left: 0,
            right: None,
            path: path.clone(),
        }],
        SR,
        1,
    )
    .unwrap();
    h.attach_recorder(rt).unwrap();
    h.record(0, false).unwrap();
    h.play_test_tone(1.0, -12.0).unwrap();
    let (l, _) = offline::render_with_input(&mut core, &vec![0.0f32; 4_800], 1, 256);
    assert!(l.iter().any(|v| v.abs() > 0.1), "o tom sai no dispositivo");
    h.stop(None).unwrap();
    h.detach_recorder().unwrap();
    core.process(&mut [0.0; 2], 2, &[], 0);
    h.collect_garbage();
    let (files, _) = session.finish();
    assert_eq!(files[0].frames, 4_800);
    let (_, ch) = read_wav_f32(&path).unwrap();
    assert!(
        ch[0].iter().all(|x| *x == 0.0),
        "a gravação lê só a entrada"
    );
}

#[test]
fn test_tone_parameters_are_clamped() {
    let (mut h, mut core) = engine();
    h.play_test_tone(f32::NAN, 20.0).unwrap(); // NaN = 1 s; +20 dBFS → 0 dBFS
    let (l, _) = offline::render(&mut core, 60_000, 1_024);
    let peak = l.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    assert!(peak <= 1.0 && peak > 0.99, "pico {peak}");
    let last = l.iter().rposition(|v| *v != 0.0).unwrap();
    assert!((47_900..48_000).contains(&last), "{last}");
}
