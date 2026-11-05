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
            serde_json::Value::Object(o) => {
                // chaves também podem ser ids (ex.: edições por nota no mapa de afinação)
                let entries = std::mem::take(o);
                for (k, mut x) in entries {
                    let mut key = serde_json::Value::String(k);
                    walk(&mut key, ids);
                    walk(&mut x, ids);
                    o.insert(key.as_str().unwrap_or_default().to_owned(), x);
                }
            }
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

fn write_fixture(name: &str, v: serde_json::Value) {
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/ipc/__fixtures__");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join(name),
        serde_json::to_string_pretty(&normalize(v)).unwrap() + "\n",
    )
    .unwrap();
}

/// Fixtures das Fases 6–8 (letra, partituras, afinação, afinador, PitchCorrector) para os
/// esquemas zod dos painéis Partitura/Letra/Afinação.
#[test]
fn write_music_ipc_fixtures() {
    use app_core::lyrics_service::LyricsFileFormat;
    use app_core::notation_service::{InstrumentKind, InstrumentTemplate, ScoreTemplate};
    use app_core::tuning_service::TunerReadingView;
    use ctx_notation::commands::{ScoreEdit, VoiceEdit};
    use ctx_notation::transpose::TransposeScope;
    use ctx_notation::{
        Articulation, Duration, Dynamic, Event, KeySignature, Meter, Mode, NoteValue, Pitch, Step,
        TabPosition, VoiceLoc,
    };
    use ctx_sync::{Granularity, LyricUnit, SyllableRef};
    use ctx_tuning::{NoteEdit, ScaleCorrection};
    use shared_kernel::music::{MidiNote, PitchClass};

    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    st.create_project(dir.path(), "Demo", AudioFormat::default())
        .unwrap();
    // áudio (Lá desafinado, 452 Hz) → mapa de afinação com nota detectada e uma edição
    let v = st
        .import_audio(
            &[sine_wav(dir.path(), "voz.wav", 452.0, 48_000)],
            None,
            Tick(0),
        )
        .unwrap();
    let source = v.sources[0].id;
    let tid: shared_kernel::id::TrackId = v.tracks[0].id.parse().unwrap();
    st.tuning_analyze(source).unwrap();
    let note = st.tuning_get(source).unwrap().notes()[0];
    st.tuning_edit_note(source, note.id, NoteEdit::to_target(MidiNote(69)), None)
        .unwrap();
    let c = PitchClass::new(0).unwrap();
    st.tuning_set_scale(source, ScaleCorrection::major(c))
        .unwrap();
    // insert de correção de afinação no canal
    st.execute(
        on_mixer(EditStrip::new(
            StripTarget::Track(tid),
            StripEdit::AddEffect {
                slot: EffectSlot::new(Effect::PitchCorrector(PitchCorrectorParams::major(7))),
                index: None,
            },
        )),
        None,
    )
    .unwrap();
    // letra: duas linhas; a primeira sincronizada por toque (palavras), a segunda sem tempo
    st.lyrics_set_text("Can-to a-qui\nou-tra li-nha").unwrap();
    st.lyrics_apply_tap_sync(Granularity::Word, 0, &[Tick(0), Tick(960)], Tick(1920))
        .unwrap();
    // partitura: melodia + violão (pauta + tablatura) com notas, articulação e dinâmica
    let (sid, _) = st
        .score_create(&ScoreTemplate {
            title: "Estudo".into(),
            composer: "Eu".into(),
            bpm: 100.0,
            meter_numerator: 4,
            meter_denominator: 4,
            key_fifths: 1,
            key_mode: Mode::Major,
            measures: 2,
            instruments: vec![
                InstrumentTemplate {
                    kind: InstrumentKind::Lead,
                    name: Some("Voz".into()),
                },
                InstrumentTemplate {
                    kind: InstrumentKind::Guitar,
                    name: None,
                },
            ],
        })
        .unwrap();
    let q = Duration::new(NoteValue::Quarter);
    let g4 = Pitch::new(Step::G, 0, 4).unwrap();
    let voice_edits = vec![
        VoiceEdit::Replace {
            index: 0,
            event: Event::note(q, g4),
        },
        VoiceEdit::AddArticulation {
            index: 0,
            articulation: Articulation::Staccato,
        },
        VoiceEdit::SetDynamic {
            index: 0,
            dynamic: Some(Dynamic::Mf),
        },
    ];
    for e in &voice_edits {
        st.score_edit_voice(sid, VoiceLoc::new(0, 0, 0), e.clone(), None)
            .unwrap();
    }
    // Fá natural em Sol maior: fora da escala da armadura
    st.score_edit_voice(
        sid,
        VoiceLoc::new(0, 1, 0),
        VoiceEdit::Replace {
            index: 0,
            event: Event::note(
                Duration::new(NoteValue::Whole),
                Pitch::new(Step::F, 0, 4).unwrap(),
            ),
        },
        None,
    )
    .unwrap();
    st.score_edit_voice(
        sid,
        VoiceLoc::new(1, 0, 0),
        VoiceEdit::Replace {
            index: 0,
            event: Event::note(
                Duration::new(NoteValue::Whole),
                Pitch::new(Step::E, 0, 3).unwrap(),
            ),
        },
        None,
    )
    .unwrap();
    st.score_edit_voice(
        sid,
        VoiceLoc::new(1, 0, 0),
        VoiceEdit::SetTab {
            index: 0,
            note: 0,
            tab: Some(TabPosition { string: 4, fret: 2 }),
        },
        None,
    )
    .unwrap();
    let report = st.score_validate(sid, None).unwrap();
    assert!(!report.measure_issues.is_empty() && !report.out_of_scale.is_empty());

    write_fixture(
        "music-session.json",
        serde_json::to_value(st.view().unwrap()).unwrap(),
    );
    write_fixture(
        "score.json",
        serde_json::to_value(st.score_get(sid).unwrap()).unwrap(),
    );
    write_fixture("score-report.json", serde_json::to_value(report).unwrap());
    write_fixture(
        "tuning-map.json",
        serde_json::to_value(st.tuning_get(source).unwrap()).unwrap(),
    );
    write_fixture(
        "tuner-reading.json",
        serde_json::to_value(TunerReadingView {
            note: 69,
            name: "A",
            octave: 4,
            cents: 23.4,
            frequency_hz: 446.0,
            confidence: 0.97,
        })
        .unwrap(),
    );
    // o que a UI envia (requisições) — mesmas formas que o Rust desserializa
    let mut all_voice = voice_edits;
    all_voice.extend([
        VoiceEdit::Insert {
            index: 1,
            event: Event::rest(Duration::new(NoteValue::Eighth)),
        },
        VoiceEdit::Delete { index: 1 },
        VoiceEdit::SetDuration {
            index: 0,
            duration: Duration::new(NoteValue::Half),
        },
        VoiceEdit::SetPitch {
            index: 0,
            note: 0,
            pitch: Pitch::new(Step::F, 1, 4).unwrap(),
        },
        VoiceEdit::SetTie {
            index: 0,
            note: 0,
            tie: true,
        },
        VoiceEdit::SetTab {
            index: 0,
            note: 0,
            tab: None,
        },
        VoiceEdit::SetDynamic {
            index: 0,
            dynamic: None,
        },
        VoiceEdit::FillRests,
    ]);
    write_fixture(
        "voice-edits.json",
        serde_json::to_value(&all_voice).unwrap(),
    );
    let score_edits = vec![
        ScoreEdit::AddMeasure { index: 2 },
        ScoreEdit::RemoveMeasure { index: 1 },
        ScoreEdit::SetKey {
            measure: 0,
            key: KeySignature::new(-2, Mode::Minor).unwrap(),
        },
        ScoreEdit::SetMeter {
            measure: 1,
            meter: Some(Meter::new(3, 4).unwrap()),
        },
        ScoreEdit::Transpose {
            scope: TransposeScope::Score,
            semitones: 2,
        },
        ScoreEdit::Transpose {
            scope: TransposeScope::Part(1),
            semitones: -12,
        },
        ScoreEdit::SetHeader {
            title: "Novo".into(),
            composer: String::new(),
        },
    ];
    write_fixture(
        "score-edits.json",
        serde_json::to_value(&score_edits).unwrap(),
    );
    write_fixture(
        "lyrics-requests.json",
        serde_json::json!({
            "units": [
                LyricUnit::Syllable(SyllableRef::new(0, 1, 0)),
                LyricUnit::Word(ctx_sync::WordRef { line: 0, word: 1 }),
                LyricUnit::Line { line: 1 },
            ],
            "formats": [
                LyricsFileFormat::Lrc { granularity: Granularity::Line },
                LyricsFileFormat::Lrc { granularity: Granularity::Syllable },
                LyricsFileFormat::Srt,
            ],
            "noteEdit": NoteEdit { flatten: 0.5, ..NoteEdit::to_target(MidiNote(70)) },
            "scale": ScaleCorrection::major(PitchClass::new(9).unwrap()),
        }),
    );
}
