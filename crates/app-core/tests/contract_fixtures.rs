//! Gera fixtures JSON do contrato IPC (fonte da verdade para os tipos/zod da UI).
//! A CI roda este teste e falha se os arquivos versionados ficarem diferentes (git diff).
mod support;
use app_core::*;
use ctx_mixing::commands::{CreateBus, EditStrip};
use ctx_mixing::*;
use ctx_recording::commands::{SetLoop, SetPunch, SetTrackRecording};
use ctx_recording::{InputAssignment, MonitorMode, TrackRecording};
use shared_kernel::{AudioFormat, Db, Pan, Tick};
use support::*;

fn normalize(mut v: serde_json::Value) -> serde_json::Value {
    // ids e caminhos variam a cada execução: substitui por marcadores estáveis preservando o formato
    fn walk(v: &mut serde_json::Value, ids: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) => {
                if s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4 {
                    let i = ids.iter().position(|x| x == s).unwrap_or_else(|| {
                        ids.push(s.clone());
                        ids.len() - 1
                    });
                    *s = format!("00000000-0000-7000-8000-{i:012}");
                } else if s.contains(".amproj") {
                    *s = "/home/usuario/Musica/Demo.amproj".into();
                } else if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                    *s = "a".repeat(64);
                } else if s.starts_with("audio/imported/") {
                    *s = format!("audio/imported/{}.wav", "a".repeat(64));
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(|x| walk(x, ids)),
            serde_json::Value::Object(o) => o.values_mut().for_each(|x| walk(x, ids)),
            _ => {}
        }
    }
    let mut ids = Vec::new();
    walk(&mut v, &mut ids);
    v
}

#[test]
fn write_ipc_fixtures() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let (h, mut core) = audio_engine::create_engine(audio_engine::EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 2).unwrap();
    st.create_project(dir.path(), "Demo", AudioFormat::default())
        .unwrap();
    let v = st
        .import_audio(&[dc_wav(dir.path(), "voz.wav", 0.3, 48_000)], None, Tick(0))
        .unwrap();
    let tid: shared_kernel::id::TrackId = v.tracks[0].id.parse().unwrap();
    st.execute(
        on_project(ctx_project::commands::SetTempo::new(Tick(3840), 90.0)),
        None,
    )
    .unwrap();
    let bus = Bus {
        id: BusId::new(),
        name: "Reverb".into(),
        strip: ChannelStrip::default(),
    };
    let bid = bus.id;
    st.execute(on_mixer(CreateBus::new(bus, 0)), None).unwrap();
    let t = StripTarget::Track(tid);
    for e in [
        StripEdit::Volume { value: Db(-6.0) },
        StripEdit::Pan { value: Pan(-0.3) },
        StripEdit::AddEffect {
            slot: EffectSlot::new(Effect::Compressor(CompressorParams::default())),
            index: None,
        },
        StripEdit::AddEffect {
            slot: EffectSlot::new(Effect::Eq(EqParams::default())),
            index: None,
        },
        StripEdit::AddSend {
            send: AuxSend {
                id: SendId::new(),
                bus: bid,
                level: Db(-12.0),
                pre_fader: false,
            },
        },
    ] {
        st.execute(on_mixer(EditStrip::new(t, e)), None).unwrap();
    }
    st.execute(
        on_mixer(EditStrip::new(
            StripTarget::Bus(bid),
            StripEdit::AddEffect {
                slot: EffectSlot::new(Effect::Reverb(ReverbParams::default())),
                index: None,
            },
        )),
        None,
    )
    .unwrap();
    let rec = TrackRecording {
        armed: true,
        input: Some(InputAssignment {
            left: 0,
            right: Some(1),
        }),
        monitor: MonitorMode::WhenArmed,
        ..Default::default()
    };
    st.execute(
        on_recording(SetTrackRecording::new(tid, rec, "Armar")),
        None,
    )
    .unwrap();
    st.execute(
        on_recording(SetPunch::new(Some((Tick(960), Tick(3840))))),
        None,
    )
    .unwrap();
    st.execute(
        on_recording(SetLoop::new(Some((Tick(0), Tick(7680))))),
        None,
    )
    .unwrap();
    st.play().unwrap();
    audio_engine::offline::render(&mut core, 2048, 512);
    let session = normalize(serde_json::to_value(st.view().unwrap()).unwrap());
    let transport = normalize(serde_json::to_value(st.transport().unwrap()).unwrap());
    let meters = normalize(serde_json::to_value(st.meters()).unwrap());
    let edits: Vec<_> = [
        StripEdit::Volume { value: Db(-3.0) },
        StripEdit::Mute { value: true },
        StripEdit::Output {
            value: Output::Bus(bid),
        },
        StripEdit::Output {
            value: Output::Master,
        },
        StripEdit::RemoveEffect {
            id: EffectId::new(),
        },
        StripEdit::SendLevel {
            id: SendId::new(),
            value: Db(-6.0),
        },
    ]
    .iter()
    .map(|e| serde_json::to_value(e).unwrap())
    .collect();
    let targets = vec![
        serde_json::to_value(StripTarget::Track(tid)).unwrap(),
        serde_json::to_value(StripTarget::Bus(bid)).unwrap(),
        serde_json::to_value(StripTarget::Master).unwrap(),
    ];
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/ipc/__fixtures__");
    std::fs::create_dir_all(&out).unwrap();
    let write = |name: &str, v: serde_json::Value| {
        std::fs::write(
            out.join(name),
            serde_json::to_string_pretty(&v).unwrap() + "\n",
        )
        .unwrap();
    };
    write("session.json", session);
    write("transport.json", transport);
    write("meters.json", meters);
    write(
        "strip-edits.json",
        normalize(serde_json::Value::Array(edits)),
    );
    write(
        "strip-targets.json",
        normalize(serde_json::Value::Array(targets)),
    );
    let err = serde_json::to_value(st.create_track("", None).unwrap_err()).unwrap();
    write("error.json", err);
}

// ---- Harmonia e bases (Fases 4–5) ----

/// Caminhos dependentes da máquina/execução viram marcadores estáveis.
fn normalize_composition(v: serde_json::Value) -> serde_json::Value {
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::String(s) => {
                if let Some(rest) = s.strip_prefix("audio/generated/") {
                    let ext = rest.rsplit('.').next().unwrap_or("wav").to_owned();
                    *s = format!("audio/generated/{}.{ext}", "a".repeat(64));
                } else if s.len() > 37
                    && s.as_bytes()[36] == b'.'
                    && s[..36].chars().filter(|c| *c == '-').count() == 4
                {
                    // arquivo copiado de base do usuário: `<uuid>.<ext>`
                    *s = format!("00000000-0000-7000-8000-00000000ffff{}", &s[36..]);
                } else if s.starts_with('/') || s.contains(":\\") {
                    *s = "/home/usuario/Musica/Composicao".into();
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(walk),
            serde_json::Value::Object(o) => o.values_mut().for_each(walk),
            _ => {}
        }
    }
    let mut v = normalize(v);
    walk(&mut v);
    v
}

fn to_json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap()
}

#[test]
fn write_composition_ipc_fixtures() {
    use app_core::composition::ctx_harmony::{Genre, Key, Mode};
    use app_core::composition::ctx_library::{
        BaseCatalog, BaseKey, BaseQuery, Instrument, KeyMode, Meter, StylePattern,
    };
    use app_core::composition::*;
    use shared_kernel::music::PitchClass;

    let key = |tonic: u8, mode: Mode| Key::new(PitchClass::new(tonic).unwrap(), mode);
    let dir = tempfile::tempdir().unwrap();
    let mut c = Composer::open(
        Box::new(MemoryLibraryStore::new(Library {
            bases: BaseCatalog::from_bases(starter_bases().unwrap()).unwrap(),
            ..Library::default()
        })),
        BaseRoots {
            user: dir.path().join("bases"),
            bundled: dir.path().join("bundled"),
        },
    )
    .unwrap();
    let mut st = studio();
    st.create_project(dir.path(), "Composição", AudioFormat::default())
        .unwrap();

    let c_major = key(0, Mode::Ionian);
    let keys = keys_catalog();
    let field = field_view(c_major);
    let progressions = c.suggest_progressions(c_major, Some(Genre::Pop), 3);
    let next = c
        .suggest_next(c_major, &["Dm7".into(), "G7".into()], 4)
        .unwrap();
    let user = c
        .add_user_progression(&UserProgressionInput {
            name: "Minha samba".into(),
            mode: Mode::Ionian,
            genres: [Genre::Samba].into(),
            beats_per_bar: 4,
            steps: "I VI7 ii7 V7".into(),
        })
        .unwrap();
    let history = c.history_view();
    let realize = RealizeRequest {
        progression: ProgressionRef::Cifra {
            chords: "C:4 G7:4".into(),
            beats_per_bar: 4,
        },
        key: c_major,
        bpm: None,
        bars: None,
        voicing: VoicingRequest::default(),
    };
    let realization = c.realize(&realize, 120.0).unwrap();
    let inserted = st.insert_progression(&c, &realize, None, Tick(0)).unwrap();

    let options = base_options();
    let gen = GenerateBaseRequest {
        progression: ProgressionRef::Cifra {
            chords: "Am:4 F:4".into(),
            beats_per_bar: 4,
        },
        key: key(9, Mode::Aeolian),
        bpm: 90.0,
        bars: 1,
        style: StylePattern::PopStrum,
        drums: None,
        bass: None,
    };
    let generated = c.generate_base(&gen).unwrap();
    let saved = c
        .save_generated_base(&SaveGeneratedBaseRequest {
            name: "Pop em Lá menor".into(),
            generation: gen,
            genre: Some("pop".into()),
            tags: vec!["suave".into()],
        })
        .unwrap();
    let wav = dc_wav(dir.path(), "loop.wav", 0.2, 4_800);
    let user_base = c
        .add_user_base(
            &wav,
            &NewUserBase {
                name: "Loop".into(),
                bpm: 96.0,
                key: Some(BaseKey {
                    tonic: PitchClass::new(9).unwrap(),
                    mode: KeyMode::Minor,
                }),
                meter: Some(Meter::FOUR_FOUR),
                genre: Some("pop".into()),
                instruments: [Instrument::Drums].into(),
                tags: vec!["loop".into()],
                progression: Some("Am F C G".into()),
            },
        )
        .unwrap();
    let mut bases: Vec<_> = c
        .search_bases(&BaseQuery::default())
        .unwrap()
        .into_iter()
        .filter(|b| b.id != saved.id && b.id != user_base.id)
        .take(2)
        .collect();
    bases.push(saved.clone());
    bases.push(user_base);
    let project_key = Some(BaseKey {
        tonic: PitchClass::new(0).unwrap(),
        mode: KeyMode::Major,
    });
    let mismatch = st
        .base_mismatch(&c, saved.id, Tick(0), project_key)
        .unwrap();
    let mut req = InsertBaseRequest {
        base_id: saved.id,
        at: Tick(0),
        track: None,
        project_key,
        decision: None,
    };
    let needs = st.insert_base(&c, &req).unwrap();
    req.decision = Some(BaseDecision::Adapt);
    let done = st.insert_base(&c, &req).unwrap();
    let synth = infra_adapters::ConfigurableSynthRenderer::default().status();
    let err = c
        .suggest_next(c_major, &["H#".into()], 4)
        .map(|_| ())
        .unwrap_err();

    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/ipc/__fixtures__");
    std::fs::create_dir_all(&out).unwrap();
    for (name, v) in [
        ("keys", to_json(&keys)),
        ("field", to_json(&field)),
        ("progressions", to_json(&progressions)),
        ("suggest-next", to_json(&next)),
        ("user-progression", to_json(&user)),
        ("library-history", to_json(&history)),
        ("realization", to_json(&realization)),
        ("inserted-clip", to_json(&inserted)),
        ("base-options", to_json(&options)),
        ("generated", to_json(&generated)),
        ("bases", to_json(&bases)),
        ("mismatch", to_json(&mismatch)),
        ("insert-base-needs-decision", to_json(&needs)),
        ("insert-base-inserted", to_json(&done)),
        ("synth-status", to_json(&synth)),
        ("error", to_json(&err)),
    ] {
        std::fs::write(
            out.join(format!("composition-{name}.json")),
            serde_json::to_string_pretty(&normalize_composition(v)).unwrap() + "\n",
        )
        .unwrap();
    }
}
