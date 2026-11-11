//! Simulação ponta a ponta do caso de uso principal (Fase 0 + gravação).
mod support;
use app_core::*;
use audio_engine::{create_engine, offline, EngineConfig};
use ctx_mixing::commands::EditStrip;
use ctx_mixing::{EffectSlot, StripEdit, StripTarget};
use ctx_project::commands::{MoveClip, SetTempo};
use ctx_recording::commands::SetTrackRecording;
use ctx_recording::{InputAssignment, MonitorMode, TrackRecording};
use shared_kernel::{AudioFormat, Db, Tick};
use support::*;

fn new_project(st: &mut Studio, dir: &std::path::Path) -> SessionView {
    st.create_project(dir, "Demo: música/1", AudioFormat::default())
        .unwrap()
}

/// Espera o dispositivo simulado gravar `frames` amostras (pela posição real do transporte),
/// em vez de um `sleep` fixo — runners de CI lentos não podem tornar o teste instável.
fn wait_recorded(st: &Studio, frames: u64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let t = st.transport().unwrap();
        if t.record_start.is_some_and(|rs| t.frame >= rs + frames) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "dispositivo simulado não avançou: {t:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Cria a faixa "Voz" armada na entrada 0 (mono), sem monitoração.
fn arm_new_track(st: &mut Studio) {
    let (tid, _) = st.create_track("Voz", None).unwrap();
    let rec = TrackRecording {
        armed: true,
        input: Some(InputAssignment {
            left: 0,
            right: None,
        }),
        monitor: MonitorMode::Off,
        ..Default::default()
    };
    st.execute(
        on_recording(SetTrackRecording::new(tid, rec, "Armar")),
        None,
    )
    .unwrap();
}

#[test]
fn create_import_play_is_sample_accurate() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let (h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 0).unwrap();
    let v = new_project(&mut st, dir.path());
    assert!(
        v.project.root.ends_with("Demo_ música_1.ebiki"),
        "nome de pasta saneado: {}",
        v.project.root
    );
    let a = impulse_wav(dir.path(), "a.wav", 0, 1000);
    let b = impulse_wav(dir.path(), "b.wav", 10, 1000);
    // importa em tick 960 (= 0,5 s = 24000 amostras a 120 BPM)
    let v = st.import_audio(&[a, b], None, Tick(960)).unwrap();
    assert_eq!(v.tracks.len(), 2);
    assert_eq!(v.tracks[0].name, "a");
    assert_eq!(v.tracks[0].clips[0].start_frame, 24_000);
    st.play().unwrap();
    let (l, _) = offline::render(&mut core, 30_000, 256);
    let hits: Vec<usize> = l
        .iter()
        .enumerate()
        .filter(|(_, x)| x.abs() > 0.5)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(hits, vec![24_000, 24_010]);
}

#[test]
fn mixer_commands_reach_the_engine_and_undo_restores() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let (h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 0).unwrap();
    new_project(&mut st, dir.path());
    let v = st
        .import_audio(&[dc_wav(dir.path(), "dc.wav", 0.5, 200_000)], None, Tick(0))
        .unwrap();
    let tid: shared_kernel::id::TrackId = v.tracks[0].id.parse().unwrap();
    st.play().unwrap();
    let level = |core: &mut audio_engine::EngineCore| offline::render(core, 2048, 512).0[2000];
    assert!((level(&mut core) - 0.5).abs() < 1e-6);
    // gesto de fader: 3 passos = 1 item de desfazer
    for db in [-1.0, -3.0, -6.0] {
        st.execute(
            on_mixer(EditStrip::new(
                StripTarget::Track(tid),
                StripEdit::Volume { value: Db(db) },
            )),
            Some(1),
        )
        .unwrap();
    }
    assert!((level(&mut core) - 0.5 * Db(-6.0).to_linear()).abs() < 1e-4);
    // efeito (dublê: ganho 0,5) no master
    st.execute(
        on_mixer(EditStrip::new(
            StripTarget::Master,
            StripEdit::AddEffect {
                slot: EffectSlot::new(ctx_mixing::Effect::Gate(Default::default())),
                index: None,
            },
        )),
        None,
    )
    .unwrap();
    assert!((level(&mut core) - 0.25 * Db(-6.0).to_linear()).abs() < 1e-4);
    st.undo().unwrap();
    st.undo().unwrap();
    assert!(
        (level(&mut core) - 0.5).abs() < 1e-6,
        "dois desfazer = estado original"
    );
    let v = st.redo().unwrap();
    assert_eq!(v.history.undo_label.as_deref(), Some("Volume"));
}

#[test]
fn every_operation_undoes_to_identical_session() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let base = st.session().unwrap().clone();
    let v = st
        .import_audio(&[impulse_wav(dir.path(), "x.wav", 0, 500)], None, Tick(0))
        .unwrap();
    let tid: shared_kernel::id::TrackId = v.tracks[0].id.parse().unwrap();
    let cid: ctx_project::ClipId = v.tracks[0].clips[0].id.parse().unwrap();
    let (t2, _) = st.create_track("Violão", None).unwrap();
    st.execute(on_project(MoveClip::new(cid, t2, Tick(1920))), None)
        .unwrap();
    st.execute(on_project(SetTempo::new(Tick(3840), 80.0)), None)
        .unwrap();
    st.delete_track(tid).unwrap();
    let full = st.session().unwrap().clone();
    for _ in 0..5 {
        st.undo().unwrap();
    }
    assert_eq!(st.session().unwrap(), &base);
    assert!(!st.view().unwrap().history.can_undo);
    for _ in 0..5 {
        st.redo().unwrap();
    }
    assert_eq!(st.session().unwrap(), &full);
}

#[test]
fn save_open_and_autosave_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = new_project(&mut st, dir.path());
    let root = std::path::PathBuf::from(&v.project.root);
    st.import_audio(&[dc_wav(dir.path(), "dc.wav", 0.1, 1000)], None, Tick(0))
        .unwrap();
    assert!(st.is_dirty());
    st.save().unwrap();
    assert!(!st.is_dirty());
    let saved = st.session().unwrap().clone();
    st.close().unwrap();
    let v = st.open_project(&root).unwrap();
    assert_eq!(st.session().unwrap(), &saved);
    assert_eq!(v.tracks[0].clips.len(), 1);
    // alteração não salva → autosave → "crash" → recuperação
    std::thread::sleep(std::time::Duration::from_millis(1100)); // mtime com resolução de segundos em alguns FS
    st.create_track("Nova", None).unwrap();
    assert!(st.autosave().unwrap());
    let changed = st.session().unwrap().clone();
    st.close().unwrap();
    assert!(Studio::recovery_candidate(&root).is_some());
    let v = st.recover(&root).unwrap();
    assert_eq!(st.session().unwrap(), &changed);
    assert!(v.history.dirty, "recuperado ainda não foi salvo");
    st.save().unwrap();
    Studio::discard_recovery(&root).unwrap();
    assert!(Studio::recovery_candidate(&root).is_none());
}

#[test]
fn autosave_rotates_backups() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = new_project(&mut st, dir.path());
    for i in 0..8 {
        st.create_track(&format!("T{i}"), None).unwrap();
        assert!(st.autosave().unwrap());
    }
    let n = std::fs::read_dir(std::path::Path::new(&v.project.root).join("backups"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".db")
        })
        .count();
    assert_eq!(n, AUTOSAVE_KEEP);
}

#[test]
fn errors_are_typed_and_state_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    assert_eq!(st.save().unwrap_err().code, "app.no_project");
    new_project(&mut st, dir.path());
    assert_eq!(
        st.create_project(dir.path(), "Demo: música/1", AudioFormat::default())
            .unwrap_err()
            .code,
        "app.exists"
    );
    assert_eq!(
        st.create_track("   ", None).unwrap_err().code,
        "project.invalid_name"
    );
    assert_eq!(
        st.start_recording(Tick(0), false).unwrap_err().code,
        "app.nothing_armed"
    );
    assert!(st
        .import_audio(&[dir.path().join("nao-existe.wav")], None, Tick(0))
        .is_err());
    assert!(
        st.session().unwrap().project.tracks().is_empty(),
        "falha na importação não deixa lixo"
    );
}

#[test]
fn folder_names_are_sanitized() {
    assert_eq!(
        sanitize_folder_name("a/b\\c:d*e?f\"g<h>i|j"),
        "a_b_c_d_e_f_g_h_i_j"
    );
    assert_eq!(sanitize_folder_name("  ..  "), "_");
    assert_eq!(sanitize_folder_name("CON"), "_CON");
    assert_eq!(sanitize_folder_name(&"x".repeat(300)).chars().count(), 100);
}

#[test]
fn records_from_simulated_device_into_take_and_clip() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let (h, core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    // entrada simulada: canal 0 = 0,25 constante
    let gen: audio_engine::null_device::InputGenerator = Box::new(|buf: &mut [f32]| {
        for (i, x) in buf.iter_mut().enumerate() {
            *x = if i % 2 == 0 { 0.25 } else { 0.0 };
        }
    });
    let dev = audio_engine::null_device::NullDevice::start(core, 256, 2, Some(gen));
    st.attach_engine(h, 2).unwrap();
    new_project(&mut st, dir.path());
    arm_new_track(&mut st);
    st.start_recording(Tick(960), false).unwrap();
    wait_recorded(&st, 19_200); // 0,4 s
    let v = st.stop_recording().unwrap();
    dev.stop();
    let t = &v.tracks[0];
    assert_eq!(t.recording.takes.len(), 1);
    assert_eq!(t.clips.len(), 1);
    let clip = &t.clips[0];
    assert!(
        clip.length_frames > 10_000,
        "≈0,4 s gravados: {}",
        clip.length_frames
    );
    assert!(
        clip.start_frame >= 24_000 && clip.start_frame - 24_000 < 30,
        "começa no ponto pedido"
    );
    let src = &v.sources[0];
    assert!(std::path::Path::new(&v.project.root)
        .join(src.rel_path.as_str())
        .exists());
    assert_eq!(v.history.undo_label.as_deref(), Some("Gravação"));
    let v = st.undo().unwrap();
    assert!(v.tracks[0].clips.is_empty() && v.tracks[0].recording.takes.is_empty());
}

#[test]
fn safe_join_rejects_escape() {
    let root = std::path::Path::new("/tmp/proj.ebiki");
    let ok = ctx_project::RelPath::new("audio/x.wav").unwrap();
    assert_eq!(safe_join(root, &ok).unwrap(), root.join("audio/x.wav"));
    assert!(
        ctx_project::RelPath::new("audio/../../x.wav").is_err(),
        "primeira barreira: RelPath"
    );
}

#[test]
fn failed_import_batch_leaves_no_orphan_copies() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = new_project(&mut st, dir.path());
    let imported = std::path::Path::new(&v.project.root).join("audio/imported");
    // um arquivo já importado continua (pode pertencer ao histórico)
    let keep = impulse_wav(dir.path(), "keep.wav", 0, 500);
    st.import_audio(&[keep], None, Tick(0)).unwrap();
    let before = std::fs::read_dir(&imported).unwrap().count();
    let good = impulse_wav(dir.path(), "good.wav", 3, 800);
    let bad = dir.path().join("bad.wav");
    std::fs::write(&bad, b"isto nao e um wav").unwrap();
    let err = st.import_audio(&[good, bad], None, Tick(0)).unwrap_err();
    assert_eq!(err.code, "codec.corrupt");
    assert_eq!(
        std::fs::read_dir(&imported).unwrap().count(),
        before,
        "cópias do lote com falha devem ser apagadas, as anteriores mantidas"
    );
    assert_eq!(st.view().unwrap().tracks.len(), 1, "estado intacto");
}

#[test]
fn discard_recovery_removes_wal_sidecars() {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let db = backups.join("autosave-1.db");
    for f in [
        db.clone(),
        backups.join("autosave-1.db-wal"),
        backups.join("autosave-1.db-shm"),
    ] {
        std::fs::write(f, b"x").unwrap();
    }
    Studio::discard_recovery(dir.path()).unwrap();
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
}

#[test]
fn swapping_engine_while_recording_keeps_the_take() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let start_dev = |st: &mut Studio| {
        let (h, core) = create_engine(EngineConfig {
            sample_rate: SR,
            ..Default::default()
        });
        let gen: audio_engine::null_device::InputGenerator = Box::new(|buf: &mut [f32]| {
            buf.iter_mut().for_each(|x| *x = 0.25);
        });
        let dev = audio_engine::null_device::NullDevice::start(core, 256, 2, Some(gen));
        st.attach_engine(h, 2).unwrap();
        dev
    };
    let dev1 = start_dev(&mut st);
    new_project(&mut st, dir.path());
    arm_new_track(&mut st);
    st.start_recording(Tick(0), false).unwrap();
    wait_recorded(&st, 14_400); // 0,3 s
                                // troca de dispositivo no meio do take
    let dev2 = start_dev(&mut st);
    dev1.stop();
    assert!(!st.is_recording());
    let v = st.view().unwrap();
    dev2.stop();
    assert_eq!(
        v.tracks[0].recording.takes.len(),
        1,
        "o take não pode ser perdido"
    );
    assert!(v.tracks[0].clips[0].length_frames > 5_000);
    let wavs = std::fs::read_dir(std::path::Path::new(&v.project.root).join("audio/recorded"))
        .unwrap()
        .count();
    assert_eq!(wavs, 1);
}

#[test]
fn opening_collects_unreferenced_media_but_keeps_backup_references() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = new_project(&mut st, dir.path());
    let root = std::path::PathBuf::from(&v.project.root);
    let kept = impulse_wav(dir.path(), "kept.wav", 0, 500);
    let v = st.import_audio(&[kept], None, Tick(0)).unwrap();
    let kept_file = root.join(v.sources[0].rel_path.as_str());
    let in_backup = impulse_wav(dir.path(), "bk.wav", 7, 600);
    let v = st.import_audio(&[in_backup], None, Tick(0)).unwrap();
    let backup_file = root.join(v.sources[1].rel_path.as_str());
    st.autosave().unwrap(); // o autosave referencia bk.wav
    st.undo().unwrap(); // o projeto salvo não referencia mais
    st.save().unwrap();
    st.close().unwrap();
    let orphan = root.join("audio/rendered/orfao.wav");
    std::fs::create_dir_all(orphan.parent().unwrap()).unwrap();
    std::fs::write(&orphan, b"x").unwrap();
    let orphan2 = root.join("audio/imported/sobra.wav");
    std::fs::write(&orphan2, b"x").unwrap();

    st.open_project(&root).unwrap();
    assert!(kept_file.exists(), "fonte do projeto mantida");
    assert!(
        backup_file.exists(),
        "fonte referenciada só pelo autosave mantida"
    );
    assert!(!orphan.exists() && !orphan2.exists(), "lixo apagado");
}

#[test]
fn opening_removes_caches_of_audio_no_longer_referenced() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = new_project(&mut st, dir.path());
    let root = std::path::PathBuf::from(&v.project.root);
    let kept = impulse_wav(dir.path(), "kept.wav", 0, 500);
    let v = st.import_audio(&[kept], None, Tick(0)).unwrap();
    let kept_hash = v.sources[0].content_hash.clone();
    let in_backup = impulse_wav(dir.path(), "bk.wav", 7, 600);
    let v = st.import_audio(&[in_backup], None, Tick(0)).unwrap();
    let backup_hash = v.sources[1].content_hash.clone();
    st.autosave().unwrap(); // o autosave ainda referencia bk.wav
    st.undo().unwrap();
    st.save().unwrap();
    st.close().unwrap();

    let orphan_hash = "f".repeat(64);
    let file = |sub: &str, name: String| {
        let p = root.join(sub).join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"x").unwrap();
        p
    };
    let kept_peaks = file("cache/peaks", format!("{kept_hash}-256.peaks"));
    let kept_spec = file(
        "cache/spectrogram",
        format!("{kept_hash}-2048-512-256.spec"),
    );
    let backup_peaks = file("cache/peaks", format!("{backup_hash}-256.peaks"));
    let orphan_peaks = file("cache/peaks", format!("{orphan_hash}-256.peaks"));
    let orphan_spec = file(
        "cache/spectrogram",
        format!("{orphan_hash}-2048-512-256.spec"),
    );
    let unknown = file("cache/peaks", "anotacoes.txt".into());

    st.open_project(&root).unwrap();
    assert!(
        kept_peaks.exists() && kept_spec.exists(),
        "cache de fonte do projeto mantido"
    );
    assert!(
        backup_peaks.exists(),
        "cache de fonte só do autosave mantido"
    );
    assert!(
        !orphan_peaks.exists() && !orphan_spec.exists(),
        "cache de áudio sem fonte apagado"
    );
    assert!(
        unknown.exists(),
        "arquivo de nome desconhecido não é tocado"
    );
}
