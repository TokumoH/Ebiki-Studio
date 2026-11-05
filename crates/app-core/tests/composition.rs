//! Integração dos casos de uso de composição (Fase 5 Harmonia + Fase 4 Bases) com o `Studio`.
mod support;
use app_core::composition::ctx_harmony::{Genre, Key, Mode};
use app_core::composition::ctx_library::{BaseKey, BaseQuery, KeyMode, StylePattern};
use app_core::composition::*;
use app_core::*;
use shared_kernel::music::PitchClass;
use shared_kernel::{AudioFormat, Tick, PPQ};
use std::path::Path;
use support::*;

fn key(tonic: u8, mode: Mode) -> Key {
    Key::new(PitchClass::new(tonic).unwrap(), mode)
}

fn composer_in(dir: &Path) -> Composer {
    Composer::open(
        Box::new(MemoryLibraryStore::default()),
        BaseRoots {
            user: dir.join("bases"),
            bundled: dir.join("bundled"),
        },
    )
    .unwrap()
}

fn open_project(st: &mut Studio, dir: &Path) -> SessionView {
    st.create_project(dir, "Composição", AudioFormat::default())
        .unwrap()
}

fn pop_axis() -> RealizeRequest {
    RealizeRequest {
        progression: ProgressionRef::Builtin {
            slug: "pop-axis".into(),
        },
        key: key(0, Mode::Ionian),
        bpm: None,
        bars: None,
        voicing: VoicingRequest::default(),
    }
}

#[test]
fn keys_field_and_suggestions_are_returned() {
    let dir = tempfile::tempdir().unwrap();
    let c = composer_in(dir.path());
    let cat = keys_catalog();
    assert_eq!(
        (cat.tonics.len(), cat.modes.len(), cat.genres.len()),
        (12, 21, 15)
    );
    let f = field_view(key(0, Mode::Ionian));
    let triads: Vec<_> = f.degrees.iter().map(|d| d.triad.symbol.as_str()).collect();
    assert_eq!(triads, ["C", "Dm", "Em", "F", "G", "Am", "Bdim"]);
    assert_eq!(f.degrees[4].seventh.symbol, "G7");
    assert_eq!(f.degrees[4].seventh.roman, "V7");

    let s = c.suggest_progressions(key(2, Mode::Ionian), Some(Genre::Samba), 5);
    assert!(!s.is_empty() && s.len() <= 5);
    assert!(s.windows(2).all(|w| w[0].score >= w[1].score));
    for p in &s {
        assert_eq!(p.cifra.len(), p.progression.roman.len());
    }
    let next = c
        .suggest_next(key(0, Mode::Ionian), &["Dm7".into(), "G7".into()], 5)
        .unwrap();
    assert_eq!(next[0].symbol, "C");
    assert_eq!(next[0].score, 1.0);
    assert!(!next[0].reason_label.is_empty());
    assert!(c
        .suggest_next(key(0, Mode::Ionian), &["H#".into()], 5)
        .is_err());
}

#[test]
fn realization_timing_is_exact() {
    let dir = tempfile::tempdir().unwrap();
    let c = composer_in(dir.path());
    let r = c.realize(&pop_axis(), 120.0).unwrap();
    assert_eq!(r.bpm, 120.0);
    assert_eq!(r.total_ticks, 16 * PPQ);
    let starts: Vec<i64> = r.chords.iter().map(|c| c.start_tick).collect();
    assert_eq!(starts, [0, 4 * PPQ, 8 * PPQ, 12 * PPQ]);
    assert!(r.chords.iter().all(|c| c.duration_ticks == 4 * PPQ));
    let symbols: Vec<_> = r.chords.iter().map(|c| c.symbol.as_str()).collect();
    assert_eq!(symbols, ["C", "G", "Am", "F"]);
    // vozes no canal 0 dentro da faixa, baixo no canal 1
    for e in &r.events {
        match e.channel {
            0 => assert!((52..=76).contains(&e.key)),
            1 => assert!((36..=52).contains(&e.key)),
            _ => panic!("canal inesperado"),
        }
    }
    // 6 compassos: repete a progressão e corta no fim
    let mut req = pop_axis();
    req.bars = Some(6);
    req.bpm = Some(90.0);
    let r6 = c.realize(&req, 120.0).unwrap();
    assert_eq!(r6.bpm, 90.0);
    assert_eq!(r6.total_ticks, 24 * PPQ);
    assert_eq!(r6.chords.len(), 6);
    assert_eq!(r6.chords[5].symbol, "G");
    let end = r6
        .events
        .iter()
        .map(|e| e.start_tick + e.duration_ticks)
        .max();
    assert_eq!(end, Some(24 * PPQ));
    // cifra avulsa e erros tipados
    let mut cifra = pop_axis();
    cifra.progression = ProgressionRef::Cifra {
        chords: "Dm7:2 G7:2 Cmaj7".into(),
        beats_per_bar: 4,
    };
    assert_eq!(c.realize(&cifra, 100.0).unwrap().total_ticks, 8 * PPQ);
    let mut bad = pop_axis();
    bad.bpm = Some(f64::NAN);
    assert_eq!(
        c.realize(&bad, 120.0).unwrap_err().code,
        "composition.invalid_bpm"
    );
    bad.bpm = None;
    bad.bars = Some(0);
    assert_eq!(
        c.realize(&bad, 120.0).unwrap_err().code,
        "composition.invalid_bars"
    );
}

#[test]
fn audition_wav_header_is_valid_and_duration_correct() {
    let dir = tempfile::tempdir().unwrap();
    let c = composer_in(dir.path());
    let st = studio(); // sem projeto: 48 kHz
    let r = c.realize(&pop_axis(), 120.0).unwrap();
    let wav = st.audition_realization(&r).unwrap();
    let info = wav_info(&wav).unwrap();
    assert_eq!(info.channels, 2);
    assert_eq!(info.sample_rate, 48_000);
    assert_eq!(info.bits_per_sample, 16);
    // 16 tempos a 120 BPM = 8 s + 1 s de cauda
    assert_eq!(info.frames as f64, (8.0 + AUDITION_TAIL_SECONDS) * 48_000.0);
    // limite de duração: 256 compassos a 20 BPM é cortado em MAX_AUDITION_SECONDS
    let mut long = pop_axis();
    long.bars = Some(256);
    long.bpm = Some(20.0);
    let r = c.realize(&long, 120.0).unwrap();
    let info = wav_info(&st.audition_realization(&r).unwrap()).unwrap();
    assert_eq!(info.frames as f64, MAX_AUDITION_SECONDS * 48_000.0);
}

#[test]
fn insert_progression_is_one_undoable_step() {
    let dir = tempfile::tempdir().unwrap();
    let c = composer_in(dir.path());
    let mut st = studio();
    open_project(&mut st, dir.path());
    let before = st.view().unwrap();
    // tick 1920 = 1 s a 120 BPM
    let ins = st
        .insert_progression(&c, &pop_axis(), None, Tick(1920))
        .unwrap();
    let v = &ins.view;
    assert_eq!(v.tracks.len(), 1);
    assert_eq!(v.tracks[0].clips.len(), 1);
    assert_eq!(v.tracks[0].clips[0].start_frame, 48_000);
    assert!(v.tracks[0].name.starts_with("Progressão"));
    assert!(ins.rel_path.starts_with("audio/generated/"));
    let root = std::path::PathBuf::from(&v.project.root);
    assert!(root.join(&ins.rel_path).is_file());
    // 8 s de música + 1 s de cauda na taxa do projeto
    assert_eq!(ins.frames as f64, (8.0 + INSERT_TAIL_SECONDS) * SR as f64);
    let session = st.session().unwrap();
    assert_eq!(session.project.sources().len(), 1);
    assert_eq!(v.history.undo_label.as_deref(), Some("Inserir progressão"));

    let undone = st.undo().unwrap();
    assert_eq!(undone.tracks, before.tracks);
    assert!(st.session().unwrap().project.sources().is_empty());
    let redone = st.redo().unwrap();
    assert_eq!(redone.tracks.len(), 1);
    assert_eq!(st.session().unwrap().project.sources().len(), 1);

    // numa faixa existente: não cria faixa nova
    let tid: shared_kernel::id::TrackId = redone.tracks[0].id.parse().unwrap();
    let again = st
        .insert_progression(&c, &pop_axis(), Some(tid), Tick(0))
        .unwrap();
    assert_eq!(again.view.tracks.len(), 1);
    assert_eq!(again.view.tracks[0].clips.len(), 2);
    st.undo().unwrap();
    assert_eq!(st.view().unwrap().tracks[0].clips.len(), 1);
}

#[test]
fn insert_progression_follows_project_tempo_changes() {
    let dir = tempfile::tempdir().unwrap();
    let c = composer_in(dir.path());
    let mut st = studio();
    open_project(&mut st, dir.path());
    // 60 BPM a partir do compasso 3: a progressão (16 tempos) começa no tick 2·3840
    st.execute(
        on_project(ctx_project::commands::SetTempo::new(Tick(2 * 3840), 60.0)),
        None,
    )
    .unwrap();
    let ins = st
        .insert_progression(&c, &pop_axis(), None, Tick(2 * 3840))
        .unwrap();
    assert_eq!(ins.frames as f64, (16.0 + INSERT_TAIL_SECONDS) * SR as f64);
}

#[test]
fn user_progressions_crud_with_undo_and_suggestions() {
    let dir = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(MemoryLibraryStore::default());
    let mut c = Composer::open(
        Box::new(store.clone()),
        BaseRoots {
            user: dir.path().join("b"),
            bundled: dir.path().join("c"),
        },
    )
    .unwrap();
    let input = UserProgressionInput {
        name: "Minha samba".into(),
        mode: Mode::Ionian,
        genres: [Genre::Samba].into(),
        beats_per_bar: 4,
        steps: "I VI7 ii7 V7".into(),
    };
    let p = c.add_user_progression(&input).unwrap();
    assert_eq!(p.origin, "user");
    assert_eq!(store.snapshot().harmony.user_progressions().len(), 1);
    // aparece nas sugestões do gênero
    let s = c.suggest_progressions(key(0, Mode::Ionian), Some(Genre::Samba), 100);
    assert!(s.iter().any(|x| x.progression.name == "Minha samba"));
    // realizar pela referência do usuário
    let mut req = pop_axis();
    req.progression = p.reference.clone();
    let r = c.realize(&req, 120.0).unwrap();
    assert_eq!(r.chords[1].symbol, "A7");
    let ProgressionRef::User { id } = &p.reference else {
        panic!("referência do usuário")
    };
    let mut edited = input.clone();
    edited.name = "Editada".into();
    assert_eq!(
        c.edit_user_progression(id, &edited).unwrap().name,
        "Editada"
    );
    assert_eq!(c.user_progressions()[0].name, "Editada");
    c.undo().unwrap();
    assert_eq!(c.user_progressions()[0].name, "Minha samba");
    assert_eq!(
        store.snapshot().harmony.user_progressions()[0].name,
        "Minha samba"
    );
    c.remove_user_progression(id).unwrap();
    assert!(c.user_progressions().is_empty());
    let h = c.undo().unwrap();
    assert_eq!(c.user_progressions().len(), 1);
    assert!(h.can_redo);
    // nome duplicado é recusado; falha de gravação desfaz a alteração
    assert!(c.add_user_progression(&input).is_err());
    store.set_fail_saves(true);
    let mut other = input.clone();
    other.name = "Outra".into();
    assert!(c.add_user_progression(&other).is_err());
    assert_eq!(c.user_progressions().len(), 1);
}

#[test]
fn composer_persists_in_real_library_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("library.db");
    let roots = BaseRoots {
        user: dir.path().join("bases"),
        bundled: dir.path().join("bundled"),
    };
    {
        let db = infra_persistence::library::LibraryDb::open(&db_path).unwrap();
        db.seed_starter_bases(&starter_bases().unwrap(), STARTER_BASES_VERSION)
            .unwrap();
        let mut c = Composer::open(Box::new(db), roots.clone()).unwrap();
        assert!(c.search_bases(&BaseQuery::default()).unwrap().len() >= 6);
        c.add_user_progression(&UserProgressionInput {
            name: "x'); DROP TABLE progression; --".into(),
            mode: Mode::Dorian,
            genres: Default::default(),
            beats_per_bar: 4,
            steps: "i7 IV7".into(),
        })
        .unwrap();
    }
    let db = infra_persistence::library::LibraryDb::open(&db_path).unwrap();
    let c = Composer::open(Box::new(db), roots).unwrap();
    assert_eq!(
        c.user_progressions()[0].name,
        "x'); DROP TABLE progression; --"
    );
    let found = c
        .search_bases(&BaseQuery {
            text: Some("bossa".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(found.len(), 1);
}

fn sine_wav(dir: &Path, name: &str, frames: usize) -> std::path::PathBuf {
    let s: Vec<f32> = (0..frames)
        .map(|i| (i as f32 * 220.0 * std::f32::consts::TAU / SR as f32).sin() * 0.4)
        .collect();
    let p = dir.join(name);
    write_wav(&p, vec![s.clone(), s]);
    p
}

#[test]
fn audio_base_mismatch_requires_decision_and_adapt_stretches() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = composer_in(dir.path());
    let src = sine_wav(dir.path(), "loop.wav", SR as usize);
    let base = c
        .add_user_base(
            &src,
            &NewUserBase {
                name: "Loop em Lá menor".into(),
                bpm: 100.0,
                key: Some(BaseKey::new(PitchClass::new(9).unwrap(), KeyMode::Minor)),
                meter: None,
                genre: Some("pop".into()),
                instruments: Default::default(),
                tags: vec!["Loop".into()],
                progression: Some("Am F C G".into()),
            },
        )
        .unwrap();
    assert!(dir
        .path()
        .join("bases")
        .join(format!("{}.wav", base.id))
        .is_file());
    // audição do arquivo da base
    let st0 = studio();
    let info = wav_info(&st0.audition_base(&c, base.id).unwrap()).unwrap();
    assert_eq!(info.frames, SR);

    let mut st = studio();
    open_project(&mut st, dir.path());
    let mut req = InsertBaseRequest {
        base_id: base.id,
        at: Tick(0),
        track: None,
        project_key: Some(BaseKey::new(PitchClass::new(11).unwrap(), KeyMode::Minor)),
        decision: None,
    };
    // projeto a 120 BPM em Si menor: precisa decidir e nada muda até lá
    let out = st.insert_base(&c, &req).unwrap();
    let InsertBaseOutcome::NeedsDecision { report } = out else {
        panic!("deveria pedir decisão")
    };
    assert!(report.needs_decision);
    assert!((report.bpm_ratio - 1.2).abs() < 1e-9);
    assert_eq!(report.semitone_shift, Some(2));
    assert!(st.view().unwrap().tracks.is_empty());
    assert!(!st.view().unwrap().history.can_undo);

    req.decision = Some(BaseDecision::Adapt);
    let InsertBaseOutcome::Inserted { clip, decision, .. } = st.insert_base(&c, &req).unwrap()
    else {
        panic!("inserida")
    };
    assert_eq!(decision, BaseDecision::Adapt);
    let expected = SR as f64 * 100.0 / 120.0;
    assert!(
        ((clip.frames as f64) - expected).abs() / expected <= 0.01,
        "{} vs {expected}",
        clip.frames
    );
    assert!(clip.rel_path.starts_with("audio/generated/"));

    req.decision = Some(BaseDecision::Keep);
    let InsertBaseOutcome::Inserted { clip, .. } = st.insert_base(&c, &req).unwrap() else {
        panic!("inserida")
    };
    assert_eq!(clip.frames, SR as u64, "Manter = duração original");
    assert!(clip.rel_path.starts_with("audio/imported/"));
    assert_eq!(st.view().unwrap().tracks.len(), 2);
    st.undo().unwrap();
    st.undo().unwrap();
    assert!(st.view().unwrap().tracks.is_empty());
    assert!(st.session().unwrap().project.sources().is_empty());

    // sem divergência: insere direto (Manter)
    let mut same = req.clone();
    same.decision = None;
    same.project_key = Some(BaseKey::new(PitchClass::new(9).unwrap(), KeyMode::Minor));
    st.execute(
        on_project(ctx_project::commands::SetTempo::new(Tick(0), 100.0)),
        None,
    )
    .unwrap();
    assert!(matches!(
        st.insert_base(&c, &same).unwrap(),
        InsertBaseOutcome::Inserted {
            decision: BaseDecision::Keep,
            ..
        }
    ));
    // formato não suportado é recusado
    let txt = dir.path().join("x.txt");
    std::fs::write(&txt, "x").unwrap();
    assert!(c
        .add_user_base(
            &txt,
            &NewUserBase {
                name: "X".into(),
                bpm: 100.0,
                key: None,
                meter: None,
                genre: None,
                instruments: Default::default(),
                tags: vec![],
                progression: None,
            }
        )
        .is_err());
}

#[test]
fn generated_base_save_audition_and_adapt_by_bpm_ratio() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = composer_in(dir.path());
    let gen = GenerateBaseRequest {
        progression: ProgressionRef::Builtin {
            slug: "ii-v-i-major".into(),
        },
        key: key(2, Mode::Ionian),
        bpm: 90.0,
        bars: 4,
        style: StylePattern::BossaComping,
        drums: None,
        bass: None,
    };
    let g = c.generate_base(&gen).unwrap();
    assert_eq!(g.total_ticks, 4 * 4 * PPQ);
    assert_eq!(g.recipe_progression, "ii7:4 V7:4 Imaj7:8");
    assert!(g.events.iter().any(|e| e.channel == 9), "bateria");
    assert!(g.events.iter().any(|e| e.channel == 1), "baixo");
    let base = c
        .save_generated_base(&SaveGeneratedBaseRequest {
            name: "Bossa em Ré".into(),
            generation: gen,
            genre: Some("bossa_nova".into()),
            tags: vec!["suave".into()],
        })
        .unwrap();
    let st0 = studio();
    let info = wav_info(&st0.audition_base(&c, base.id).unwrap()).unwrap();
    // 16 tempos a 90 BPM = 10,666… s + cauda
    let secs = 16.0 * 60.0 / 90.0 + AUDITION_TAIL_SECONDS;
    assert!((info.frames as f64 - secs * 48_000.0).abs() <= 1.0);

    let mut st = studio();
    open_project(&mut st, dir.path()); // 120 BPM
    let mut req = InsertBaseRequest {
        base_id: base.id,
        at: Tick(0),
        track: None,
        project_key: None,
        decision: Some(BaseDecision::Keep),
    };
    let InsertBaseOutcome::Inserted {
        clip: keep, report, ..
    } = st.insert_base(&c, &req).unwrap()
    else {
        panic!()
    };
    assert_eq!(report.semitone_shift, None, "tom do projeto desconhecido");
    req.decision = Some(BaseDecision::Adapt);
    let InsertBaseOutcome::Inserted { clip: adapt, .. } = st.insert_base(&c, &req).unwrap() else {
        panic!()
    };
    let tail = INSERT_TAIL_SECONDS * SR as f64;
    let ratio = (adapt.frames as f64 - tail) / (keep.frames as f64 - tail);
    assert!((ratio - 90.0 / 120.0).abs() < 0.01, "{ratio}");
    // biblioteca: desfazer remove a base salva
    c.undo().unwrap();
    assert!(c.base(base.id).is_err());
}
