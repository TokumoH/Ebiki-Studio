//! Simulação com as implementações reais: codecs + DSP + SQLite + engine.
use app_core::ports::{ExportFormatKind, ExportSettings, SystemClock};
use app_core::*;
use ctx_mixing::commands::EditStrip;
use ctx_mixing::presets::InstrumentPreset;
use ctx_mixing::{StripEdit, StripTarget};
use infra_adapters::{CodecsMedia, DspProcessorFactory};
use infra_codecs::{EncodeOptions, Encoder, ExportFormat, SampleDepth};
use shared_kernel::{AudioFormat, Tick};
use std::sync::Arc;

fn studio() -> Studio {
    Studio::new(StudioDeps {
        repo: Arc::new(infra_persistence::SqliteSessionRepository),
        media: Arc::new(CodecsMedia),
        factory: Arc::new(DspProcessorFactory),
        clock: Arc::new(SystemClock),
        synth: Arc::new(infra_adapters::SynthRenderer),
        notation: Arc::new(infra_adapters::NotationIoFormats),
    })
}

fn tone_file(
    dir: &std::path::Path,
    name: &str,
    sr: u32,
    seconds: f32,
    freq: f32,
    amp: f32,
) -> std::path::PathBuf {
    let n = (sr as f32 * seconds) as usize;
    let s: Vec<f32> = (0..n)
        .map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin())
        .collect();
    let path = dir.join(name);
    let opts = EncodeOptions {
        depth: SampleDepth::Float32,
        dither: false,
        ..Default::default()
    };
    let mut enc = Encoder::new(&path, ExportFormat::Wav, sr, 2, &opts).unwrap();
    enc.write(&[&s, &s]).unwrap();
    enc.finish().unwrap();
    path
}

#[test]
fn import_resampled_mix_export_and_analyze() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = st
        .create_project(dir.path(), "Integração", AudioFormat::default())
        .unwrap();
    // arquivo a 44,1 kHz: deve ser convertido para 48 kHz do projeto
    let f = tone_file(dir.path(), "voz.wav", 44_100, 2.0, 440.0, 0.5);
    let v2 = st.import_audio(&[f.clone(), f], None, Tick(0)).unwrap();
    assert_eq!(
        v2.sources.len(),
        1,
        "mesmo conteúdo importado duas vezes = uma fonte (dedupe)"
    );
    assert_eq!(v2.tracks.len(), 2);
    assert_eq!(v2.sources[0].length_frames, 96_000, "2 s a 48 kHz");
    let tid: shared_kernel::id::TrackId = v2.tracks[0].id.parse().unwrap();
    for slot in InstrumentPreset::LeadVocal.chain() {
        st.execute(
            on_mixer(EditStrip::new(
                StripTarget::Track(tid),
                StripEdit::AddEffect { slot, index: None },
            )),
            None,
        )
        .unwrap();
    }
    let wav = dir.path().join("mix.wav");
    st.export_mix(&wav, &ExportSettings::default(), None)
        .unwrap();
    let decoded = infra_codecs::decode_to_f32(&wav, 48_000).unwrap();
    assert_eq!(
        decoded.channels[0].len(),
        96_000 + 96_000,
        "conteúdo + 2 s de cauda"
    );
    assert!(decoded.channels[0].iter().all(|x| x.is_finite()));
    let peak = decoded.channels[0].iter().fold(0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.05, "a mixagem tem sinal");

    let mp3 = dir.path().join("mix.mp3");
    let s = ExportSettings {
        format: ExportFormatKind::Mp3,
        mp3_kbps: Some(192),
        normalize: true,
        ..Default::default()
    };
    st.export_mix(&mp3, &s, Some((Tick(0), Tick(1920))))
        .unwrap();
    let d = infra_codecs::decode_to_f32(&mp3, 48_000).unwrap();
    let expected = 48_000 + 96_000; // 1 s de intervalo + 2 s de cauda
    assert!(
        (d.channels[0].len() as i64 - expected as i64).abs() < 2_000,
        "{}",
        d.channels[0].len()
    );
    let p = d.channels[0].iter().fold(0f32, |m, x| m.max(x.abs()));
    assert!((p - 0.891).abs() < 0.08, "normalizado para −1 dBFS: {p}");

    let stems = st
        .export_stems(&dir.path().join("stems"), &ExportSettings::default())
        .unwrap();
    assert_eq!(stems.len(), 2);
    assert!(stems.iter().all(|p| p.exists()));

    let (analysis, advice) = st.analyze_mix(-14.0).unwrap();
    assert_eq!(analysis.channels.len(), 2);
    assert!(analysis.master.integrated_lufs.is_finite());
    assert!(analysis.master.correlation > 0.9, "sinais idênticos L/R");
    // duas faixas somando ~0,5 de pico cada podem passar de 0 dBFS → alerta de clipping/true peak
    let codes: Vec<_> = advice.iter().map(|a| a.code.as_str()).collect();
    assert!(
        !codes.is_empty(),
        "uma mix quente gera ao menos um conselho: {:?}",
        analysis.master
    );
    let _ = v;
}

#[test]
fn reopen_project_decodes_sources_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = st
        .create_project(dir.path(), "Reabrir", AudioFormat::default())
        .unwrap();
    let f = tone_file(dir.path(), "a.wav", 48_000, 0.5, 220.0, 0.3);
    st.import_audio(&[f], None, Tick(0)).unwrap();
    st.save().unwrap();
    st.close().unwrap();
    let v2 = st
        .open_project(std::path::Path::new(&v.project.root))
        .unwrap();
    let id: shared_kernel::id::AudioSourceId = v2.sources[0].id;
    let peaks = st.waveform(id, 256).unwrap();
    assert_eq!(peaks.len(), (24_000usize).div_ceil(256) * 2);
    assert!(peaks.iter().any(|x| *x > 0.25));
}
