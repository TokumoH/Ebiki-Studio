//! S-1: a engine roda na taxa do projeto e o conversor entrega na taxa do dispositivo sem mudar a
//! altura (tom de teste de 440 Hz continua 440 Hz num fone a 44,1 kHz), e a entrada convertida chega
//! ao núcleo na taxa do projeto.
mod common;
use audio_engine::resampler::{InputRateAdapter, OutputRateAdapter};
use audio_engine::*;
use common::*;

/// Frequência por cruzamentos de zero (canal 0 de um sinal estéreo intercalado).
fn freq_of(x: &[f32], rate: u32) -> f64 {
    let frames = x.len() / 2;
    let crossings = (1..frames)
        .filter(|j| (x[(j - 1) * 2] < 0.0) != (x[j * 2] < 0.0))
        .count();
    crossings as f64 / 2.0 / (frames as f64 / rate as f64)
}

#[test]
fn engine_at_project_rate_keeps_pitch_on_a_44k1_device() {
    for device_rate in [44_100u32, 16_000, 96_000] {
        let (mut h, mut core) = create_engine(EngineConfig {
            sample_rate: SR,
            ..Default::default()
        });
        h.play_test_tone(5.0, -6.0).unwrap();
        let mut a = OutputRateAdapter::new(2, SR, device_rate, 4096);
        let mut out = Vec::new();
        let mut rendered = 0usize;
        // callbacks irregulares do driver (taxa do dispositivo), ~3 s (o tom dura 5 s)
        let calls = device_rate as usize * 3 / 4032 * 5;
        for (k, n) in [512usize, 441, 1024, 7, 2048]
            .iter()
            .cycle()
            .take(calls)
            .enumerate()
        {
            let mut dev = vec![f32::NAN; n * 2];
            a.fill(&mut dev, |eng: &mut [f32]| {
                rendered += eng.len() / 2;
                core.process(eng, 2, &[], 0);
            });
            assert!(dev.iter().all(|x| x.is_finite()), "callback {k} preenchido");
            out.extend_from_slice(&dev);
        }
        // fora das rampas do tom (~10 ms), a frequência continua 440 Hz
        let skip = device_rate as usize / 20;
        let mid = &out[skip * 2..out.len() - skip * 2];
        let f = freq_of(mid, device_rate);
        assert!((f - 440.0).abs() < 1.5, "{device_rate} Hz: {f:.2} Hz");
        // a engine andou na taxa do projeto: quadros do dispositivo × razão + a antecedência do filtro
        let dev_frames = out.len() / 2;
        let expected = ((dev_frames as u64 - 1) * SR as u64 / device_rate as u64) as usize
            + a.latency_engine_frames()
            + 1;
        assert_eq!(rendered, expected, "{device_rate} Hz");
    }
}

#[test]
fn converted_microphone_reaches_the_core_at_the_project_rate() {
    // microfone a 44,1 kHz, projeto a 48 kHz: 1 s de entrada vira ~48 000 quadros para a fila
    let mut a = InputRateAdapter::new(1, 44_100, SR, 4096);
    let mut got = 0usize;
    let mic: Vec<f32> = (0..44_100)
        .map(|i| (std::f32::consts::TAU * 1000.0 * i as f32 / 44_100.0).sin())
        .collect();
    let mut converted = Vec::new();
    for c in mic.chunks(441) {
        a.process(c, |x| {
            got += x.len();
            converted.extend_from_slice(x);
        });
    }
    assert!((got as i64 - 48_000).abs() <= 30, "{got}");
    let stereo: Vec<f32> = converted.iter().flat_map(|x| [*x, *x]).collect();
    let f = freq_of(&stereo[200..stereo.len() - 200], SR);
    assert!((f - 1000.0).abs() < 2.0, "{f:.2} Hz");
}
