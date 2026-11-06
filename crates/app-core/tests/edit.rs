//! Fase 3 — edição ponta a ponta: casos de uso do `Studio` → engine (render offline).
mod support;
use app_core::edit_service::FadeRequest;
use app_core::*;
use audio_engine::{create_engine, offline, EngineConfig, EngineCore};
use ctx_project::{ClipId, FadeShape};
use shared_kernel::{AudioFormat, Tick};
use support::*;

/// Projeto com um clipe de seno (amplitude 0,5) em tick 0, tocando.
fn setup(dir: &std::path::Path) -> (Studio, EngineCore, ClipId) {
    let mut st = studio();
    let (h, core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 0).unwrap();
    st.create_project(dir, "Edição", AudioFormat::default())
        .unwrap();
    let v = st
        .import_audio(&[sine_wav(dir, "s.wav", 220.0, 48_000)], None, Tick(0))
        .unwrap();
    let id = v.tracks[0].clips[0].id.parse().unwrap();
    (st, core, id)
}

fn render(st: &mut Studio, core: &mut EngineCore, frames: usize) -> Vec<f32> {
    st.seek(Tick(0)).unwrap();
    st.play().unwrap();
    let out = offline::render(core, frames, 500).0;
    st.pause().unwrap();
    core.process(&mut [], 2, &[], 0);
    out
}

#[test]
fn split_renders_identically_and_undo_restores_one_clip() {
    let dir = tempfile::tempdir().unwrap();
    let (mut st, mut core, id) = setup(dir.path());
    let before = render(&mut st, &mut core, 40_000);
    // 960 ticks = 0,5 s = 24 000 amostras
    let v = st.clip_split(id, Tick(960)).unwrap();
    let clips = &v.tracks[0].clips;
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].length_frames, 24_000);
    assert_eq!(clips[1].start_frame, 24_000);
    assert_eq!(clips[1].offset_frames, 24_000);
    assert_eq!(
        render(&mut st, &mut core, 40_000),
        before,
        "dividir não altera o áudio"
    );
    let v = st.undo().unwrap();
    assert_eq!(v.tracks[0].clips.len(), 1);
    assert_eq!(v.history.undo_label.as_deref(), Some("Importar áudio"));
    let v = st.redo().unwrap();
    assert_eq!(v.tracks[0].clips.len(), 2);
}

#[test]
fn fades_polarity_reverse_and_trim_reach_the_engine() {
    let dir = tempfile::tempdir().unwrap();
    let (mut st, mut core, id) = setup(dir.path());
    let plain = render(&mut st, &mut core, 48_000);

    st.clip_set_polarity(id, true).unwrap();
    let inv = render(&mut st, &mut core, 48_000);
    assert!(plain.iter().zip(&inv).all(|(a, b)| *a == -*b));
    st.undo().unwrap();

    let v = st
        .clip_set_fades(
            id,
            FadeRequest {
                fade_in_frames: 4800,
                fade_out_frames: 4800,
                fade_in_shape: FadeShape::Linear,
                fade_out_shape: FadeShape::EqualPower,
            },
            None,
        )
        .unwrap();
    assert_eq!(v.tracks[0].clips[0].edit.fade_in_frames, 4800);
    let faded = render(&mut st, &mut core, 48_000);
    assert_eq!(faded[0], 0.0);
    let early: f32 = faded[..480].iter().map(|x| x.abs()).fold(0.0, f32::max);
    assert!(
        early < 0.06,
        "início do fade de entrada quase em silêncio: {early}"
    );
    assert_eq!(faded[24_000], plain[24_000], "meio do clipe intacto");

    st.clip_set_reversed(id, true).unwrap();
    st.undo().unwrap();
    st.undo().unwrap();
    assert_eq!(
        render(&mut st, &mut core, 48_000),
        plain,
        "desfazer restaura tudo"
    );

    // corta a borda direita pela metade
    let v = st.clip_trim(id, Tick(0), 24_000, Some(7)).unwrap();
    assert_eq!(v.tracks[0].clips[0].length_frames, 24_000);
    assert_eq!(v.tracks[0].clips[0].source_frames, 48_000);
    let trimmed = render(&mut st, &mut core, 30_000);
    assert!(trimmed[24_000..].iter().all(|x| *x == 0.0));
    let err = st.clip_trim(id, Tick(0), 48_001, None).unwrap_err();
    assert_eq!(err.code, "project.clip_out_of_bounds");
}

#[test]
fn normalize_sets_clip_gain_from_the_source_peak() {
    let dir = tempfile::tempdir().unwrap();
    let (mut st, _core, id) = setup(dir.path());
    let v = st.clip_normalize(id, -1.0).unwrap();
    let g = v.tracks[0].clips[0].gain.0;
    // seno de amplitude 0,5 (−6,02 dBFS) → +5,02 dB
    assert!((g - 5.0206).abs() < 0.05, "ganho {g}");
    assert_eq!(v.history.undo_label.as_deref(), Some("Ganho do clipe"));
    assert_eq!(
        st.clip_normalize(id, 3.0).unwrap_err().code,
        "api.invalid_argument"
    );
    // o arquivo da fonte não foi alterado (RNF-07)
    let src_dir = std::path::Path::new(&v.project.root).join("audio/imported");
    let file = std::fs::read_dir(src_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let (_, ch) = audio_engine::recorder::read_wav_f32(&file).unwrap();
    let peak = ch[0].iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!((peak - 0.5).abs() < 1e-3);
}

#[test]
fn crossfade_of_overlapping_clips_is_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let (mut st, _core, id) = setup(dir.path());
    let v = st.clip_split(id, Tick(960)).unwrap();
    let second: ClipId = v.tracks[0].clips[1].id.parse().unwrap();
    // estende a segunda parte para a esquerda: 96 ticks = 2 400 amostras de sobreposição
    let v = st.clip_trim(second, Tick(864), 26_400, None).unwrap();
    assert_eq!(v.tracks[0].clips[1].offset_frames, 21_600);
    let v = st
        .clips_crossfade(id, second, FadeShape::EqualPower)
        .unwrap();
    let c = &v.tracks[0].clips;
    assert_eq!(c[0].edit.fade_out_frames, 2_400);
    assert_eq!(c[1].edit.fade_in_frames, 2_400);
    assert_eq!(v.history.undo_label.as_deref(), Some("Crossfade"));
    let v = st.undo().unwrap();
    assert_eq!(v.tracks[0].clips[0].edit.fade_out_frames, 0);
}

#[test]
fn spectrogram_without_codec_support_reports_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let (st, _core, _) = setup(dir.path());
    let v = st.view().unwrap();
    let src = v.tracks[0].clips[0].source_id.parse().unwrap();
    let err = st
        .spectrogram(src, app_core::ports::SpectrogramRequest::default())
        .unwrap_err();
    assert_eq!(err.code, "media.unsupported");
}
