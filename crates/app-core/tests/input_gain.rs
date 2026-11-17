//! Ganho de entrada e tom de teste ("Testar som") pela camada de aplicação.
mod support;
use app_core::device_monitor::DeviceMonitorView;
use app_core::ports::ExportSettings;
use audio_engine::{create_engine, offline, EngineConfig, EngineCore, EngineHandle};
use shared_kernel::{AudioFormat, Tick};
use support::*;

fn engine() -> (EngineHandle, EngineCore) {
    create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    })
}

#[test]
fn gain_is_validated_kept_without_engine_and_applied_on_attach() {
    let mut st = studio();
    assert_eq!(st.input_gain_db(), 0.0, "começa em 0 dB");
    assert_eq!(
        st.set_input_gain_db(f64::NAN).unwrap_err().code,
        "app.invalid"
    );
    assert_eq!(
        st.set_input_gain_db(30.0).unwrap(),
        24.0,
        "limitado a +24 dB"
    );
    assert_eq!(st.set_input_gain_db(-3.0).unwrap(), 0.0, "sem atenuação");
    // sem engine: guardado e aplicado quando ela chegar (ex.: valor salvo lido ao abrir o app)
    assert_eq!(st.set_input_gain_db(12.0).unwrap(), 12.0);
    let (h, mut core) = engine();
    st.attach_engine(h, 1).unwrap();
    st.set_device_monitor(Some(DeviceMonitorView {
        left: 0,
        right: None,
        gain_db: 0.0,
    }))
    .unwrap();
    offline::render_with_input(&mut core, &vec![0.0f32; 2_048], 1, 512); // rampa
    st.meters();
    let (l, _) = offline::render_with_input(&mut core, &vec![0.1f32; 2_048], 1, 512);
    let m = st.meters();
    assert!(
        (m.inputs[0] - 0.398).abs() < 1e-3,
        "medidor com +12 dB: {:?}",
        m.inputs
    );
    assert!(
        (l[2_047] - 0.398).abs() < 1e-3,
        "ouvir o microfone com +12 dB: {}",
        l[2_047]
    );
    // troca de dispositivo: o ganho continua valendo na engine nova
    let (h2, mut core2) = engine();
    st.attach_engine(h2, 1).unwrap();
    assert_eq!(st.input_gain_db(), 12.0);
    offline::render_with_input(&mut core2, &vec![0.1f32; 2_048], 1, 512);
    st.meters();
    offline::render_with_input(&mut core2, &vec![0.1f32; 2_048], 1, 512);
    assert!((st.meters().inputs[0] - 0.398).abs() < 1e-3);
}

#[test]
fn test_tone_needs_an_engine_and_is_not_exported() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    assert_eq!(st.play_test_tone().unwrap_err().code, "app.no_engine");
    st.create_project(dir.path(), "Tom", AudioFormat::default())
        .unwrap();
    let (h, mut core) = engine();
    st.attach_engine(h, 0).unwrap();
    st.play_test_tone().unwrap();
    // sai no dispositivo: ~1 s a −12 dBFS
    let (l, _) = offline::render(&mut core, 60_000, 512);
    let peak = l.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    assert!((peak - 0.251).abs() < 0.005, "pico {peak}");
    let last = l.iter().rposition(|v| *v != 0.0).unwrap();
    assert!((47_900..48_000).contains(&last), "{last}");
    // a exportação usa outro núcleo, offline: o tom não aparece
    st.play_test_tone().unwrap();
    let file = dir.path().join("mix.wav");
    st.export_mix(
        &file,
        &ExportSettings::default(),
        Some((Tick(0), Tick(3_840))),
    )
    .unwrap();
    let mut rd = hound::WavReader::open(&file).unwrap();
    let samples: Vec<f32> = rd.samples::<f32>().map(Result::unwrap).collect();
    assert!(!samples.is_empty());
    assert!(samples.iter().all(|v| *v == 0.0), "exportação sem o tom");
}
