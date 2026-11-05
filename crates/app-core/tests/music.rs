//! Integração das Fases 6–8 pelo `Studio`: notação, letra, afinação, time-stretch, afinador,
//! corretor em tempo real e persistência dos novos agregados.
mod support;
use app_core::lyrics_service::LyricsFileFormat;
use app_core::notation_service::*;
use app_core::*;
use audio_engine::{create_engine, offline, EngineConfig};
use ctx_mixing::commands::EditStrip;
use ctx_mixing::{Effect, EffectSlot, PitchCorrectorParams, StripEdit, StripTarget};
use ctx_notation::commands::{ScoreEdit, VoiceEdit};
use ctx_notation::playback::{render, PlaybackOptions};
use ctx_notation::{Duration, Event, Mode, NoteValue, Pitch, ScoreId, Step, VoiceLoc};
use ctx_project::commands::SetTempo;
use ctx_project::ClipId;
use ctx_sync::{Granularity, LyricUnit, Span};
use ctx_tuning::{NoteEdit, ScaleCorrection};
use shared_kernel::id::AudioSourceId;
use shared_kernel::music::{MidiNote, PitchClass};
use shared_kernel::{AudioFormat, Tick};
use support::*;

fn new_project(st: &mut Studio, dir: &std::path::Path) -> SessionView {
    st.create_project(dir, "Música", AudioFormat::default())
        .unwrap()
}

fn template() -> ScoreTemplate {
    ScoreTemplate {
        title: "Estudo".into(),
        composer: "Eu".into(),
        bpm: 120.0,
        meter_numerator: 4,
        meter_denominator: 4,
        key_fifths: 1,
        key_mode: Mode::Major,
        measures: 4,
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
    }
}

fn whole(step: Step, octave: i8) -> Event {
    Event::note(
        Duration::new(NoteValue::Whole),
        Pitch::new(step, 0, octave).unwrap(),
    )
}

/// Partitura com notas nas duas partes (para exportar/tocar).
fn score_with_notes(st: &mut Studio) -> ScoreId {
    let (id, v) = st.score_create(&template()).unwrap();
    assert_eq!(v.scores.len(), 1);
    assert_eq!(v.scores[0].parts, vec!["Voz", "Violão"]);
    assert_eq!(v.scores[0].measures, 4);
    for (m, step) in [(0, Step::G), (1, Step::A), (2, Step::B), (3, Step::D)] {
        st.score_edit_voice(
            id,
            VoiceLoc::new(0, m, 0),
            VoiceEdit::Replace {
                index: 0,
                event: whole(step, 4),
            },
            None,
        )
        .unwrap();
        st.score_edit_voice(
            id,
            VoiceLoc::new(1, m, 0),
            VoiceEdit::Replace {
                index: 0,
                event: whole(step, 3),
            },
            None,
        )
        .unwrap();
    }
    id
}

// ------------------------------------------------------------------ notação

#[test]
fn score_edits_undo_and_redo_through_the_studio_history() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let (id, _) = st.score_create(&template()).unwrap();
    let blank = st.score_get(id).unwrap();
    st.score_edit_voice(
        id,
        VoiceLoc::new(0, 0, 0),
        VoiceEdit::Replace {
            index: 0,
            event: whole(Step::C, 5),
        },
        None,
    )
    .unwrap();
    let v = st
        .score_edit(
            id,
            ScoreEdit::SetHeader {
                title: "Outro".into(),
                composer: "Ela".into(),
            },
        )
        .unwrap();
    assert_eq!(v.scores[0].title, "Outro");
    let edited = st.score_get(id).unwrap();
    assert_eq!(
        edited.parts[0].measures[0].voices[0].events[0].notes()[0]
            .pitch
            .midi(),
        72
    );
    assert!(st
        .score_validate(id, None)
        .unwrap()
        .measure_issues
        .is_empty());
    let v = st.undo().unwrap();
    assert_eq!(v.scores[0].title, "Estudo");
    st.undo().unwrap();
    assert_eq!(
        st.score_get(id).unwrap(),
        blank,
        "desfazer restaura a partitura exata"
    );
    st.redo().unwrap();
    st.redo().unwrap();
    assert_eq!(st.score_get(id).unwrap(), edited);
    // edição inválida não altera nada (atômica) e não entra no histórico
    let before = st.session().unwrap().clone();
    assert!(st
        .score_edit_voice(id, VoiceLoc::new(9, 0, 0), VoiceEdit::FillRests, None)
        .is_err());
    assert_eq!(st.session().unwrap(), &before);
    // remover e desfazer
    let v = st.score_delete(id).unwrap();
    assert!(v.scores.is_empty());
    st.undo().unwrap();
    assert_eq!(st.score_get(id).unwrap(), edited);
}

#[test]
fn import_export_musicxml_roundtrip_via_studio() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let id = score_with_notes(&mut st);
    let original = st.score_get(id).unwrap();
    let xml = dir.path().join("estudo.musicxml");
    st.score_export(id, &xml, ScoreExportFormat::MusicXml)
        .unwrap();
    let r = st.score_import(&xml).unwrap();
    assert_eq!(r.view.scores.len(), 2);
    let imported = st.score_get(r.score_id.parse().unwrap()).unwrap();
    assert_eq!(imported.title, original.title);
    assert_eq!(imported.composer, original.composer);
    assert_eq!(imported.parts.len(), 2);
    assert_eq!(
        imported.parts[1].staves, original.parts[1].staves,
        "tablatura preservada"
    );
    assert_eq!(
        render(&imported, &PlaybackOptions::default()),
        render(&original, &PlaybackOptions::default()),
        "reprodução idêntica após ida e volta"
    );
    // MXL e MIDI também saem e voltam
    let mxl = dir.path().join("estudo.mxl");
    st.score_export(id, &mxl, ScoreExportFormat::Mxl).unwrap();
    assert!(std::fs::read(&mxl).unwrap().starts_with(b"PK"));
    let mid = dir.path().join("estudo.mid");
    st.score_export(id, &mid, ScoreExportFormat::Midi).unwrap();
    assert!(std::fs::read(&mid).unwrap().starts_with(b"MThd"));
    let r2 = st.score_import(&mxl).unwrap();
    let from_mxl = st.score_get(r2.score_id.parse().unwrap()).unwrap();
    assert_eq!(
        render(&from_mxl, &PlaybackOptions::default()),
        render(&original, &PlaybackOptions::default())
    );
    let r3 = st.score_import(&mid).unwrap();
    assert_eq!(r3.view.scores.len(), 4);
    // importação é um único desfazer
    st.undo().unwrap();
    assert_eq!(st.view().unwrap().scores.len(), 3);
    // arquivo desconhecido → erro estável, nada muda
    let junk = dir.path().join("x.txt");
    std::fs::write(&junk, b"nada a ver").unwrap();
    assert_eq!(
        st.score_import(&junk).unwrap_err().code,
        "notation_io.unsupported"
    );
}

#[test]
fn score_audition_and_render_to_track_is_one_undo() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let id = score_with_notes(&mut st);
    let wav = st.score_audition(id).unwrap();
    assert_eq!(&wav[0..4], b"RIFF");
    // 4 compassos 4/4 a 120 BPM = 8 s + cauda
    let frames = (wav.len() - 44) / 8;
    assert!(frames as f64 / SR as f64 >= 8.0);
    let before = st.session().unwrap().clone();
    let v = st.score_render_to_track(id, None, Tick(960)).unwrap();
    assert_eq!(v.tracks.len(), 1);
    assert_eq!(v.tracks[0].name, "Estudo");
    assert_eq!(v.tracks[0].clips.len(), 1);
    assert_eq!(v.tracks[0].clips[0].start, Tick(960));
    assert!(v.sources[0]
        .rel_path
        .as_str()
        .starts_with("audio/rendered/"));
    assert_eq!(
        v.history.undo_label.as_deref(),
        Some("Renderizar partitura")
    );
    st.undo().unwrap();
    assert_eq!(
        st.session().unwrap(),
        &before,
        "fonte + faixa + clipe num único desfazer"
    );
    // partitura sem notas não renderiza
    let (empty, _) = st.score_create(&template()).unwrap();
    assert_eq!(
        st.score_audition(empty).unwrap_err().code,
        "app.empty_score"
    );
}

#[test]
fn score_from_chords_and_out_of_scale_report() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let slots = vec![
        ctx_notation::arrange::ChordSlot {
            start_tick: Tick(0),
            duration_ticks: Tick(3840),
            notes: vec![48, 64, 67],
        },
        ctx_notation::arrange::ChordSlot {
            start_tick: Tick(3840),
            duration_ticks: Tick(3840),
            notes: vec![53, 66, 69], // Fá# fora de Dó maior
        },
    ];
    let (id, v) = st
        .score_create_from_chords(
            "Acordes",
            ctx_notation::Meter::new(4, 4).unwrap(),
            ctx_notation::KeySignature::new(0, Mode::Major).unwrap(),
            &slots,
        )
        .unwrap();
    assert_eq!(v.scores[0].parts, vec!["Piano"]);
    let report = st.score_validate(id, None).unwrap();
    assert!(report.measure_issues.is_empty());
    assert_eq!(report.out_of_scale.len(), 1);
    assert_eq!(report.out_of_scale[0].pitch.midi(), 66);
    let with_scale = st
        .score_validate(id, Some(&[0, 2, 4, 5, 6, 7, 9, 11]))
        .unwrap();
    assert!(with_scale.out_of_scale.is_empty());
}

// --------------------------------------------------------------------- letra

#[test]
fn lyrics_lrc_export_follows_tempo_changes_and_reimports() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    // 120 BPM até o compasso 2 (tick 3840 = 2 s), depois 60 BPM
    st.execute(on_project(SetTempo::new(Tick(3840), 60.0)), None)
        .unwrap();
    let v = st.lyrics_set_text("Primeira linha\nsegunda linha").unwrap();
    assert_eq!(v.lyrics.lines().len(), 2);
    assert_eq!(v.history.undo_label.as_deref(), Some("Importar letra"));
    st.lyrics_set_span(
        LyricUnit::Line { line: 0 },
        Span::new(Tick(1920), Tick(3840)).unwrap(),
        None,
    )
    .unwrap();
    st.lyrics_set_span(
        LyricUnit::Line { line: 1 },
        Span::new(Tick(3840 + 960), Tick(3840 + 1920)).unwrap(),
        None,
    )
    .unwrap();
    let lrc = st
        .lyrics_render(LyricsFileFormat::Lrc {
            granularity: Granularity::Line,
        })
        .unwrap();
    // linha 1: tick 1920 = 1 s; linha 2: 2 s + 960 ticks a 60 BPM (1 s) = 3 s
    assert!(lrc.contains("[00:01.00]Primeira linha"), "{lrc}");
    assert!(lrc.contains("[00:03.00]segunda linha"), "{lrc}");
    let srt = st.lyrics_render(LyricsFileFormat::Srt).unwrap();
    assert!(srt.contains("00:00:03,000 --> 00:00:04,000"), "{srt}");
    // exporta (sílaba), desloca, reimporta: volta aos tempos exportados
    let file = dir.path().join("letra.lrc");
    st.lyrics_export(
        &file,
        LyricsFileFormat::Lrc {
            granularity: Granularity::Syllable,
        },
    )
    .unwrap();
    let exported = st.session().unwrap().lyrics.clone();
    st.lyrics_shift(
        ctx_sync::SyllableRef::new(0, 0, 0),
        ctx_sync::SyllableRef::new(1, 1, 1),
        480,
        None,
    )
    .unwrap();
    let v = st.lyrics_import(&file).unwrap();
    let back: Vec<_> = v.lyrics.syllables().map(|(_, s)| s.span).collect();
    let orig: Vec<_> = exported.syllables().map(|(_, s)| s.span).collect();
    assert_eq!(back.len(), orig.len());
    for (a, b) in back.iter().zip(&orig) {
        let (a, b) = (a.unwrap(), b.unwrap());
        // precisão LRC = 10 ms (≤ 20 ticks a 60 BPM)
        assert!((a.start.0 - b.start.0).abs() <= 20, "{a:?} vs {b:?}");
    }
    // tudo desfazível até a letra vazia
    for _ in 0..5 {
        st.undo().unwrap();
    }
    assert!(st.session().unwrap().lyrics.lines().is_empty());
}

#[test]
fn tap_sync_applies_atomically_and_undoes() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    st.lyrics_set_text("can-to a-qui").unwrap();
    let before = st.session().unwrap().lyrics.clone();
    let v = st
        .lyrics_apply_tap_sync(Granularity::Word, 0, &[Tick(0), Tick(960)], Tick(1920))
        .unwrap();
    let spans: Vec<_> = v.lyrics.syllables().filter_map(|(_, s)| s.span).collect();
    assert_eq!(spans.first().unwrap().start, Tick(0));
    assert_eq!(spans.last().unwrap().end, Tick(1920));
    // toques fora de ordem: erro e nada muda
    let snapshot = st.session().unwrap().clone();
    let e = st
        .lyrics_apply_tap_sync(Granularity::Word, 0, &[Tick(960), Tick(10)], Tick(2000))
        .unwrap_err();
    assert_eq!(e.code, "sync.tap_out_of_order");
    assert_eq!(st.session().unwrap(), &snapshot);
    st.undo().unwrap();
    assert_eq!(st.session().unwrap().lyrics, before);
}

// ------------------------------------------------------------------ afinação

#[test]
fn tuning_analyze_apply_corrects_pitch_and_undo_restores_original_clip() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    // Lá desafinado: 452 Hz (+46,6 cents)
    let v = st
        .import_audio(
            &[sine_wav(dir.path(), "voz.wav", 452.0, 48_000)],
            None,
            Tick(0),
        )
        .unwrap();
    let source: AudioSourceId = v.sources[0].id;
    let clip: ClipId = v.tracks[0].clips[0].id.parse().unwrap();
    let v = st.tuning_analyze(source).unwrap();
    assert_eq!(v.tuning.maps.len(), 1);
    let map = st.tuning_get(source).unwrap();
    assert_eq!(map.notes().len(), 1, "{:?}", map.notes());
    let note = map.notes()[0];
    assert!((note.midi - 69.466).abs() < 0.05, "midi {}", note.midi);
    // sem correções: nada a aplicar
    assert_eq!(
        st.tuning_apply(source, clip).unwrap_err().code,
        "tuning.nothing_to_apply"
    );
    st.tuning_edit_note(source, note.id, NoteEdit::to_target(MidiNote(69)), None)
        .unwrap();
    let before_apply = st.session().unwrap().clone();
    let v = st.tuning_apply(source, clip).unwrap();
    assert_eq!(
        v.history.undo_label.as_deref(),
        Some("Aplicar correção de afinação")
    );
    let new_clip = &v.tracks[0].clips[0];
    assert_ne!(
        new_clip.source_id,
        source.to_string(),
        "clipe aponta para a fonte nova"
    );
    assert_eq!(new_clip.length_frames, 48_000);
    let new_source: AudioSourceId = new_clip.source_id.parse().unwrap();
    let buf = st.sources().unwrap()[&new_source].clone();
    let hz = median_pitch(&buf.channels[0][9_600..40_000]);
    assert!((hz - 440.0).abs() < 2.0, "corrigido para {hz:.2} Hz");
    let orig = st.sources().unwrap()[&source].clone();
    assert!((median_pitch(&orig.channels[0][9_600..40_000]) - 452.0).abs() < 2.0);
    // o arquivo novo existe no projeto; o original continua lá
    let root = std::path::PathBuf::from(&v.project.root);
    let rel = &v
        .sources
        .iter()
        .find(|s| s.id == new_source)
        .unwrap()
        .rel_path;
    assert!(root.join(rel.as_str()).is_file());
    st.undo().unwrap();
    assert_eq!(
        st.session().unwrap(),
        &before_apply,
        "desfazer devolve o clipe original"
    );
    assert_eq!(
        st.view().unwrap().tracks[0].clips[0].source_id,
        source.to_string()
    );
}

#[test]
fn scale_correction_applies_without_manual_edits() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let v = st
        .import_audio(
            &[sine_wav(dir.path(), "voz.wav", 452.0, 48_000)],
            None,
            Tick(0),
        )
        .unwrap();
    let source: AudioSourceId = v.sources[0].id;
    let clip: ClipId = v.tracks[0].clips[0].id.parse().unwrap();
    st.tuning_analyze(source).unwrap();
    let mut scale = ScaleCorrection::major(PitchClass::new(0).unwrap());
    scale.enabled = true;
    let v = st.tuning_set_scale(source, scale).unwrap();
    assert!(v.tuning.maps[0].scale.enabled);
    let v = st.tuning_apply(source, clip).unwrap();
    let new_source: AudioSourceId = v.tracks[0].clips[0].source_id.parse().unwrap();
    let hz = median_pitch(&st.sources().unwrap()[&new_source].channels[0][9_600..40_000]);
    assert!(
        (hz - 440.0).abs() < 2.0,
        "escala de Dó puxa para Lá 440: {hz:.2}"
    );
    // referência do afinador é desfazível e validada
    assert_eq!(
        st.tuning_set_reference(500.0).unwrap_err().code,
        "tuning.invalid_reference"
    );
    let v = st.tuning_set_reference(432.0).unwrap();
    assert!((v.tuning.a4_hz - 432.0).abs() < 1e-9);
    let v = st.tuning_clear(source).unwrap();
    assert!(v.tuning.maps.is_empty());
    st.undo().unwrap();
    assert_eq!(st.view().unwrap().tuning.maps.len(), 1);
}

#[test]
fn time_stretch_to_project_tempo_has_expected_length_and_keeps_pitch() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let v = st
        .import_audio(
            &[sine_wav(dir.path(), "loop.wav", 220.0, 96_000)],
            None,
            Tick(0),
        )
        .unwrap();
    let clip: ClipId = v.tracks[0].clips[0].id.parse().unwrap();
    let before = st.session().unwrap().clone();
    // loop gravado a 100 BPM num projeto de 120 BPM → 0,8333 da duração
    let v = st.clip_time_stretch(clip, 100.0, None).unwrap();
    let c = &v.tracks[0].clips[0];
    let expected = 96_000.0 * 100.0 / 120.0;
    let ratio = c.length_frames as f64 / expected;
    assert!(
        (ratio - 1.0).abs() < 0.01,
        "duração {} vs {expected}",
        c.length_frames
    );
    assert_eq!(c.offset_frames, 0);
    let src: AudioSourceId = c.source_id.parse().unwrap();
    let hz = median_pitch(&st.sources().unwrap()[&src].channels[0][8_000..70_000]);
    assert!((hz - 220.0).abs() < 1.5, "altura preservada: {hz:.2}");
    assert_eq!(
        v.history.undo_label.as_deref(),
        Some("Ajustar ao andamento")
    );
    st.undo().unwrap();
    assert_eq!(st.session().unwrap(), &before);
    assert_eq!(
        st.clip_time_stretch(clip, 120.0, None).unwrap_err().code,
        "app.nothing_to_do"
    );
}

#[test]
fn pitch_corrector_insert_pulls_452_hz_to_440_in_the_full_engine() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio_dsp();
    new_project(&mut st, dir.path());
    let v = st
        .import_audio(
            &[sine_wav(dir.path(), "voz.wav", 452.0, 96_000)],
            None,
            Tick(0),
        )
        .unwrap();
    let tid: shared_kernel::id::TrackId = v.tracks[0].id.parse().unwrap();
    st.execute(
        on_mixer(EditStrip::new(
            StripTarget::Track(tid),
            StripEdit::AddEffect {
                slot: EffectSlot::new(Effect::PitchCorrector(PitchCorrectorParams {
                    retune_ms: 5.0,
                    ..PitchCorrectorParams::major(0)
                })),
                index: None,
            },
        )),
        None,
    )
    .unwrap();
    let (l, r) = st.render_mixdown(None, 0.0).unwrap();
    let hz = median_pitch(&l[48_000..90_000]);
    assert!((hz - 440.0).abs() < 2.0, "insert corrigiu para {hz:.2} Hz");
    assert!((median_pitch(&r[48_000..90_000]) - hz).abs() < 0.5);
    // bypass: sinal original
    st.undo().unwrap();
    let (l, _) = st.render_mixdown(None, 0.0).unwrap();
    assert!((median_pitch(&l[48_000..90_000]) - 452.0).abs() < 2.0);
}

#[test]
fn tuner_reads_the_armed_input_through_the_engine_tap() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let (h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 1).unwrap();
    new_project(&mut st, dir.path());
    assert_eq!(st.tuner_start(3).unwrap_err().code, "app.input_unavailable");
    assert_eq!(st.tuner_read().unwrap_err().code, "app.tuner_off");
    st.tuner_start(0).unwrap();
    // 0,3 s de Lá 446 Hz (+23,4 cents) na entrada
    let input: Vec<f32> = (0..14_400)
        .map(|i| 0.4 * (std::f32::consts::TAU * 446.0 * i as f32 / SR as f32).sin())
        .collect();
    offline::render_with_input(&mut core, &input, 1, 256);
    let r = st.tuner_read().unwrap().expect("leitura vozeada");
    assert_eq!(r.note, 69);
    assert_eq!((r.name, r.octave), ("A", 4));
    assert!((r.cents - 23.4).abs() < 2.0, "cents {}", r.cents);
    // desligar remove a derivação da engine
    st.tuner_stop().unwrap();
    assert!(!st.tuner_active());
    offline::render_with_input(&mut core, &input, 1, 256);
    assert_eq!(st.tuner_read().unwrap_err().code, "app.tuner_off");
}

// -------------------------------------------------------------- persistência

#[test]
fn new_aggregates_survive_save_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    let v = new_project(&mut st, dir.path());
    let root = std::path::PathBuf::from(&v.project.root);
    let id = score_with_notes(&mut st);
    st.lyrics_set_text("olá mundo").unwrap();
    st.lyrics_set_span(
        LyricUnit::Line { line: 0 },
        Span::new(Tick(0), Tick(960)).unwrap(),
        None,
    )
    .unwrap();
    let v = st
        .import_audio(
            &[sine_wav(dir.path(), "voz.wav", 452.0, 48_000)],
            None,
            Tick(0),
        )
        .unwrap();
    let source = v.sources[0].id;
    st.tuning_analyze(source).unwrap();
    let note = st.tuning_get(source).unwrap().notes()[0].id;
    st.tuning_edit_note(source, note, NoteEdit::to_target(MidiNote(69)), None)
        .unwrap();
    st.tuning_set_reference(442.0).unwrap();
    st.save().unwrap();
    let saved = st.session().unwrap().clone();
    st.close().unwrap();
    let v = st.open_project(&root).unwrap();
    assert_eq!(st.session().unwrap(), &saved);
    assert_eq!(v.scores[0].id, id.to_string());
    assert_eq!(v.lyrics.text(), "olá mundo");
    assert!((v.tuning.a4_hz - 442.0).abs() < 1e-9);
}

// ------------------------------------------------------- UI (alphaTab / bytes)

#[test]
fn score_musicxml_feeds_the_renderer_and_bytes_import_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = studio();
    new_project(&mut st, dir.path());
    let id = score_with_notes(&mut st);
    let original = st.score_get(id).unwrap();
    let xml = st.score_musicxml(id).unwrap();
    assert!(xml.contains("<score-partwise"), "MusicXML partwise");
    assert!(xml.contains("Estudo"), "título no documento");
    // bytes vindos da UI (sem caminho): mesmo importador, reprodução idêntica
    let r = st.score_import_bytes(xml.as_bytes(), "ignorado").unwrap();
    assert_eq!(r.view.scores.len(), 2);
    let back = st.score_get(r.score_id.parse().unwrap()).unwrap();
    assert_eq!(back.title, "Estudo");
    assert_eq!(
        render(&back, &PlaybackOptions::default()),
        render(&original, &PlaybackOptions::default())
    );
    // MIDI (sem metadados de título garantidos): título nunca fica vazio
    let midi = infra_notation_io::midi::export_midi(
        &original,
        &infra_notation_io::midi::MidiExportOptions::default(),
    )
    .unwrap();
    let r = st.score_import_bytes(&midi, "Riff").unwrap();
    assert!(!r.view.scores.last().unwrap().title.trim().is_empty());
    assert_eq!(r.view.scores.len(), 3);
    assert_eq!(
        st.score_import_bytes(b"lixo", "x").unwrap_err().code,
        "notation_io.unsupported"
    );
    assert!(st.score_musicxml(ScoreId::new()).is_err());
}
