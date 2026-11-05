//! R4: salvar → carregar devolve exatamente o mesmo estado.
use app_core::Session;
use ctx_mixing::*;
use ctx_project::*;
use ctx_recording::*;
use infra_persistence::SqliteSessionRepository;
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::{AudioFormat, Db, Pan, Tick};

fn rich_session() -> Session {
    let mut p = Project::new("Minha música", AudioFormat::default()).unwrap();
    p.tempo_mut().set_tempo(Tick(3840), 90.0).unwrap();
    p.tempo_mut().set_meter(Tick(3840 * 2), 6, 8).unwrap();
    let hash = "a".repeat(64);
    let src = AudioSource {
        id: AudioSourceId::new(),
        rel_path: RelPath::new(format!("audio/imported/{hash}.wav")).unwrap(),
        content_hash: hash,
        original_name: "voz'; DROP TABLE track; --.wav".into(),
        channels: 2,
        length_frames: 480_000,
    };
    p.add_source(src.clone()).unwrap();
    let t1 = Track::new_audio("Voz").unwrap();
    let t2 = Track::new_audio("Violão").unwrap();
    let (id1, id2) = (t1.id, t2.id);
    p.insert_track(t1, 0).unwrap();
    p.insert_track(t2, 1).unwrap();
    for (i, start) in [(0, 0i64), (1, 960)] {
        let _ = i;
        p.insert_clip(
            id1,
            Clip {
                id: ClipId::new(),
                source_id: src.id,
                start: Tick(start),
                offset_frames: 10,
                length_frames: 1000,
                gain: Db(-3.5),
            },
            usize::MAX,
        )
        .unwrap();
    }
    let mut m = Mixer::new();
    m.add_strip(id1, ChannelStrip::default()).unwrap();
    m.add_strip(id2, ChannelStrip::default()).unwrap();
    let bus = Bus {
        id: BusId::new(),
        name: "Reverb".into(),
        strip: ChannelStrip::default(),
    };
    let bid = bus.id;
    m.add_bus(bus, 0).unwrap();
    let t = StripTarget::Track(id1);
    m.edit(t, &StripEdit::Volume { value: Db(-6.0) }).unwrap();
    m.edit(t, &StripEdit::Pan { value: Pan(-0.25) }).unwrap();
    m.edit(t, &StripEdit::Solo { value: true }).unwrap();
    m.edit(
        t,
        &StripEdit::Output {
            value: Output::Bus(bid),
        },
    )
    .unwrap();
    m.edit(
        StripTarget::Track(id2),
        &StripEdit::AddSend {
            send: AuxSend {
                id: SendId::new(),
                bus: bid,
                level: Db(-12.0),
                pre_fader: true,
            },
        },
    )
    .unwrap();
    for slot in presets::InstrumentPreset::LeadVocal.chain() {
        m.edit(t, &StripEdit::AddEffect { slot, index: None })
            .unwrap();
    }
    for slot in presets::InstrumentPreset::MasterBus.chain() {
        m.edit(
            StripTarget::Master,
            &StripEdit::AddEffect { slot, index: None },
        )
        .unwrap();
    }
    m.edit(
        StripTarget::Bus(bid),
        &StripEdit::AddEffect {
            slot: EffectSlot::new(Effect::Reverb(ReverbParams::default())),
            index: None,
        },
    )
    .unwrap();
    m.edit(StripTarget::Bus(bid), &StripEdit::Mute { value: true })
        .unwrap();
    let mut r = RecordingSetup::default();
    r.set_punch(Some((Tick(960), Tick(1920)))).unwrap();
    r.set_metronome(MetronomeConfig {
        enabled: true,
        volume: 0.5,
        subdivision: 2,
        accent_first_beat: false,
        count_in_bars: 2,
    })
    .unwrap();
    let take = Take {
        id: TakeId::new(),
        source_id: src.id,
        start: Tick(0),
        length_frames: 1000,
        number: 1,
    };
    r.set_track(
        id1,
        TrackRecording {
            armed: true,
            input: Some(InputAssignment {
                left: 0,
                right: Some(1),
            }),
            monitor: MonitorMode::WhenArmed,
            active_take: Some(take.id),
            takes: vec![take],
        },
    )
    .unwrap();
    Session {
        project: p,
        mixer: m,
        recording: r,
        lyrics: app_core::empty_lyrics(),
        scores: Default::default(),
        tuning: Default::default(),
    }
}

#[test]
fn save_load_is_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    let s = rich_session();
    let repo = SqliteSessionRepository;
    repo.save_to(&path, &s).unwrap();
    let loaded = repo.load_from(&path).unwrap();
    assert_eq!(loaded.project, s.project);
    assert_eq!(loaded.recording, s.recording);
    assert_eq!(loaded.mixer, s.mixer);
    // salvar de novo sobre o mesmo arquivo substitui tudo (sem duplicar)
    repo.save_to(&path, &loaded).unwrap();
    assert_eq!(repo.load_from(&path).unwrap(), s);
}

#[test]
fn loading_missing_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(SqliteSessionRepository
        .load_from(&dir.path().join("nao-existe.db"))
        .is_err());
}

#[test]
fn tampered_path_in_db_is_rejected_on_load() {
    // R5: um banco adulterado com caminho fora do projeto não pode ser carregado
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    let s = rich_session();
    SqliteSessionRepository.save_to(&path, &s).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE audio_source SET rel_path = '../../etc/passwd'", [])
        .unwrap();
    drop(c);
    let err = SqliteSessionRepository.load_from(&path).unwrap_err();
    assert_eq!(err.code(), "persistence.corrupt");
}

#[test]
fn failed_save_keeps_previous_state() {
    // estado inconsistente (canal do mixer para faixa inexistente) viola FK → transação desfeita
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    let good = rich_session();
    SqliteSessionRepository.save_to(&path, &good).unwrap();
    let mut bad = good.clone();
    bad.mixer
        .add_strip(TrackId::new(), ChannelStrip::default())
        .unwrap();
    assert!(SqliteSessionRepository.save_to(&path, &bad).is_err());
    assert_eq!(SqliteSessionRepository.load_from(&path).unwrap(), good);
}
