//! Entrada ao vivo: medidores por canal e "Ouvir entrada" (monitor direto do dispositivo).
mod common;
use audio_engine::*;
use common::*;

fn engine() -> (EngineHandle, EngineCore) {
    let (mut h, core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    h.swap_graph(RenderGraph::empty(SR)).unwrap();
    (h, core)
}

/// Bloco intercalado estéreo com valores constantes por canal.
fn stereo(l: f32, r: f32, frames: usize) -> Vec<f32> {
    (0..frames).flat_map(|_| [l, r]).collect()
}

/// Roda callbacks suficientes para a rampa de ganho terminar e devolve a última saída.
fn settle(core: &mut EngineCore, input: &[f32], in_ch: usize) -> Vec<f32> {
    let frames = input.len() / in_ch;
    let mut out = vec![0.0f32; frames * 2];
    for _ in 0..4 {
        core.process(&mut out, 2, input, in_ch);
    }
    out
}

#[test]
fn input_peaks_are_reported_per_channel() {
    let (h, mut core) = engine();
    let input = stereo(0.5, -0.25, 256);
    let mut out = vec![0.0f32; 512];
    core.process(&mut out, 2, &input, 2);
    let snap = h.snapshot();
    assert_eq!(snap.input_peaks, vec![0.5, 0.25]);
    // lidos e zerados
    assert_eq!(h.snapshot().input_peaks, vec![0.0, 0.0]);
}

#[test]
fn without_input_device_there_are_no_input_meters() {
    let (h, mut core) = engine();
    let mut out = vec![0.0f32; 512];
    core.process(&mut out, 2, &[], 0);
    assert!(h.snapshot().input_peaks.is_empty());
}

#[test]
fn monitor_is_off_by_default() {
    let (_h, mut core) = engine();
    let out = settle(&mut core, &stereo(0.5, 0.5, 256), 2);
    assert!(out.iter().all(|x| *x == 0.0));
}

#[test]
fn mono_monitor_sends_the_channel_to_both_sides() {
    let (mut h, mut core) = engine();
    h.set_device_monitor(Some(DeviceMonitor {
        left: 1,
        right: None,
        gain: 0.5,
    }))
    .unwrap();
    let out = settle(&mut core, &stereo(0.9, 0.4, 256), 2);
    let last = &out[out.len() - 2..];
    assert!(
        (last[0] - 0.2).abs() < 1e-6 && (last[1] - 0.2).abs() < 1e-6,
        "{last:?}"
    );
}

#[test]
fn stereo_monitor_keeps_left_and_right() {
    let (mut h, mut core) = engine();
    h.set_device_monitor(Some(DeviceMonitor {
        left: 0,
        right: Some(1),
        gain: 1.0,
    }))
    .unwrap();
    let out = settle(&mut core, &stereo(0.3, -0.6, 256), 2);
    let last = &out[out.len() - 2..];
    assert!(
        (last[0] - 0.3).abs() < 1e-6 && (last[1] + 0.6).abs() < 1e-6,
        "{last:?}"
    );
}

#[test]
fn gain_changes_are_ramped_without_jumps() {
    let (mut h, mut core) = engine();
    h.set_device_monitor(Some(DeviceMonitor {
        left: 0,
        right: None,
        gain: 1.0,
    }))
    .unwrap();
    let input = stereo(1.0, 0.0, 256);
    let mut out = vec![0.0f32; 512];
    core.process(&mut out, 2, &input, 2);
    // primeiro callback: sobe aos poucos a partir de zero (sem estalo)
    assert!(out[0] < 0.05, "começa perto de zero: {}", out[0]);
    let steps: Vec<f32> = out.chunks(2).map(|f| f[0]).collect();
    assert!(steps.windows(2).all(|w| w[1] >= w[0] && w[1] - w[0] < 0.01));
}

#[test]
fn monitor_stays_out_of_the_mix_and_meters() {
    let (mut h, mut core) = engine();
    h.set_device_monitor(Some(DeviceMonitor {
        left: 0,
        right: Some(1),
        gain: 1.0,
    }))
    .unwrap();
    settle(&mut core, &stereo(0.7, 0.7, 256), 2);
    // o master (o que vai para gravação de mixagem e exportação) não vê o monitor
    assert_eq!(
        h.snapshot().peaks.first().copied().unwrap_or([0.0; 2]),
        [0.0, 0.0]
    );
}

#[test]
fn turning_off_fades_out_and_bad_channels_are_silent() {
    let (mut h, mut core) = engine();
    h.set_device_monitor(Some(DeviceMonitor {
        left: 0,
        right: None,
        gain: 1.0,
    }))
    .unwrap();
    settle(&mut core, &stereo(0.5, 0.5, 256), 2);
    h.set_device_monitor(None).unwrap();
    let out = settle(&mut core, &stereo(0.5, 0.5, 256), 2);
    assert!(out.iter().all(|x| *x == 0.0));
    // canal inexistente no dispositivo: silêncio, sem pânico
    h.set_device_monitor(Some(DeviceMonitor {
        left: 7,
        right: Some(9),
        gain: 1.0,
    }))
    .unwrap();
    let out = settle(&mut core, &stereo(0.5, 0.5, 256), 2);
    assert!(out.iter().all(|x| *x == 0.0));
}
