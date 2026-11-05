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
