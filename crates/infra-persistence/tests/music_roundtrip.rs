//! Fases 6–8: letra, afinação e partituras sobrevivem a salvar → carregar (identidade).
use app_core::Session;
use ctx_notation::arrange::{chords_to_score, ChordSlot};
use ctx_notation::commands::{EditScore, ScoreEdit};
use ctx_notation::*;
use ctx_project::*;
use ctx_sync::{LyricTrack, LyricUnit, Span, SyllableRef};
use ctx_tuning::*;
use infra_persistence::SqliteSessionRepository;
use shared_kernel::command::Command;
use shared_kernel::id::AudioSourceId;
use shared_kernel::music::{MidiNote, PitchClass};
use shared_kernel::{AudioFormat, Tick};

/// Partitura com 2 partes (piano em pauta dupla + violão com tablatura), quiáltera, técnicas,
/// articulação, dinâmica, letra, ligadura de expressão, armadura e mudança de andamento.
pub fn rich_score() -> Score {
    let slots: Vec<ChordSlot> = (0..4)
        .map(|i| ChordSlot {
            start_tick: Tick(i * 960 * 3),
            duration_ticks: Tick(960 * 3),
            notes: vec![50 + i as u8, 62, 66, 69],
        })
        .collect();
    let mut s = chords_to_score(
        "Ensaio",
        100.0,
        Meter::new(3, 4).unwrap(),
        KeySignature::new(2, Mode::Major).unwrap(),
        &slots,
    )
    .unwrap();
    let mut guitar = s.blank_part(
        "Violão",
        24,
        vec![Staff::standard_and_tab(
            Clef::Treble8vb,
            TabConfig::standard_guitar(),
        )],
    );
    let mut e4 = Note::new(Pitch::new(Step::E, 0, 4).unwrap());
    e4.tab = Some(TabPosition { string: 1, fret: 0 });
    e4.techniques = vec![Technique::Bend { cents: 200 }, Technique::Vibrato];
    let mut first = Event::chord(Duration::quarter(), vec![e4]);
    first.articulations = vec![Articulation::Accent];
    first.dynamic = Some(Dynamic::F);
    first.lyric = Some(Lyric {
        text: "lá".into(),
        syllabic: Syllabic::Single,
    });
    let eighth3 = Duration::new(NoteValue::Eighth).with_tuplet(Tuplet::TRIPLET);
    let mut events = vec![first];
    for step in [Step::F, Step::G, Step::A] {
        events.push(Event::note(
            eighth3,
            Pitch::new(step, if step == Step::F { 1 } else { 0 }, 4).unwrap(),
        ));
    }
    events.push(Event::rest(Duration::quarter()));
    guitar.measures[0].voices[0].events = events;
    EditScore::new(ScoreEdit::AddPart {
        index: 1,
        part: Box::new(guitar),
    })
    .apply(&mut s)
    .unwrap();
    let ids: Vec<EventId> = s.parts[1].measures[0].voices[0]
        .events
        .iter()
        .map(|e| e.id)
        .collect();
    for edit in [
        ScoreEdit::AddSpanner {
            part: 1,
            spanner: Spanner {
                kind: SpannerKind::Slur,
                start: ids[0],
                end: ids[3],
            },
        },
        ScoreEdit::SetHeader {
            title: "Ensaio nº 1".into(),
            composer: "Fulano \"de Tal\"".into(),
        },
        ScoreEdit::SetTempo {
            tick: Tick(960 * 6),
            bpm: 72.0,
        },
        ScoreEdit::SetRepeat {
            measure: 1,
            start: true,
            end: None,
        },
    ] {
        EditScore::new(edit).apply(&mut s).unwrap();
    }
    s.check_invariants().unwrap();
    s
}

fn rich_music_session() -> Session {
    let mut p = Project::new("Fases 6-8", AudioFormat::default()).unwrap();
    let hash = "b".repeat(64);
    let src = AudioSource {
        id: AudioSourceId::new(),
        rel_path: RelPath::new(format!("audio/imported/{hash}.wav")).unwrap(),
        content_hash: hash,
        original_name: "voz.wav".into(),
        channels: 1,
        length_frames: 96_000,
    };
    p.add_source(src.clone()).unwrap();
    let mut s = Session::new(p);
    // letra com tempos parciais e caracteres especiais
    let mut lyrics = LyricTrack::from_plain_text("Olá, coração\nvai 'cantar' já").unwrap();
    lyrics
        .set_unit_span(
            LyricUnit::Line { line: 0 },
            Span::new(Tick(0), Tick(1920)).unwrap(),
        )
        .unwrap();
    lyrics
        .set_unit_span(
            LyricUnit::Syllable(SyllableRef::new(1, 0, 0)),
            Span::new(Tick(2000), Tick(2400)).unwrap(),
        )
        .unwrap();
    s.lyrics = lyrics;
    // afinação: notas, edições e escala
    let mut map = PitchCorrectionMap::new(src.id);
    map.replace_notes(vec![
        DetectedNote::new(0, 10_000, 69.46).unwrap(),
        DetectedNote::new(12_000, 30_000, 71.9).unwrap(),
        DetectedNote::new(30_000, 50_000, 64.0).unwrap(),
    ])
    .unwrap();
    let n = map.notes().to_vec();
    map.set_edit(n[0].id, NoteEdit::to_target(MidiNote(69)))
        .unwrap();
    map.set_edit(
        n[1].id,
        NoteEdit {
            target: None,
            cents_offset: -12.5,
            flatten: 0.4,
            formant_semitones: 1.0,
        },
    )
    .unwrap();
    let mut scale = ScaleCorrection::major(PitchClass::new(9).unwrap());
    scale.enabled = true;
    scale.retune_speed_ms = 35.0;
    scale.humanize = 0.25;
    map.set_scale(scale).unwrap();
    s.tuning = TuningBook::from_parts(vec![map], TunerSettings::new(432.0).unwrap());
    // partituras: rica + vazia (sem partes)
    let blank = Score::new("Rascunho", 90.0, Meter::new(4, 4).unwrap()).unwrap();
    s.scores = ScoreBook::from_entries(vec![
        ScoreEntry {
            id: ScoreId::new(),
            score: rich_score(),
        },
        ScoreEntry {
            id: ScoreId::new(),
            score: blank,
        },
    ])
    .unwrap();
    s
}

#[test]
fn lyrics_tuning_and_scores_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    let s = rich_music_session();
    let repo = SqliteSessionRepository;
    repo.save_to(&path, &s).unwrap();
    let loaded = repo.load_from(&path).unwrap();
    assert_eq!(loaded.lyrics, s.lyrics);
    assert_eq!(loaded.tuning, s.tuning);
    assert_eq!(loaded.scores, s.scores);
    assert_eq!(loaded, s);
    // regravar não duplica
    repo.save_to(&path, &loaded).unwrap();
    assert_eq!(repo.load_from(&path).unwrap(), s);
}

#[test]
fn score_header_lives_in_columns_not_in_the_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    let s = rich_music_session();
    SqliteSessionRepository.save_to(&path, &s).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    let (title, body): (String, String) = c
        .query_row(
            "SELECT title, body FROM score ORDER BY position LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(title, "Ensaio nº 1");
    assert!(
        !body.contains("Ensaio nº 1"),
        "título não se repete no corpo"
    );
    assert!(
        !body.contains("Violão"),
        "nome da parte não se repete no corpo"
    );
    let names: String = c
        .query_row(
            "SELECT part_names FROM v_score_summary ORDER BY position LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(names, "Piano / Violão");
    // a projeção de palavras deriva texto e intervalo das sílabas
    let (text, start, end): (String, i64, i64) = c
        .query_row(
            "SELECT text, start_tick, end_tick FROM v_lyric_words WHERE line = 0 AND word = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(text, "coração");
    assert!(start > 0 && end == 1920);
    // renomear a parte só na coluna é refletido na carga (fonte única da verdade)
    c.execute(
        "UPDATE score_part SET name = 'Guitarra' WHERE position = 1",
        [],
    )
    .unwrap();
    drop(c);
    let loaded = SqliteSessionRepository.load_from(&path).unwrap();
    assert_eq!(loaded.scores.entries()[0].score.parts[1].name, "Guitarra");
}

#[test]
fn corrupted_score_document_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    SqliteSessionRepository
        .save_to(&path, &rich_music_session())
        .unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE score SET doc_version = 99", []).unwrap();
    drop(c);
    let err = SqliteSessionRepository.load_from(&path).unwrap_err();
    assert_eq!(err.code(), "persistence.corrupt");
}

#[test]
fn removed_source_drops_its_unreachable_correction_map_on_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.db");
    let mut s = rich_music_session();
    let orphan = PitchCorrectionMap::new(AudioSourceId::new());
    s.tuning.put(orphan.source, Some(orphan)).unwrap();
    SqliteSessionRepository.save_to(&path, &s).unwrap();
    let loaded = SqliteSessionRepository.load_from(&path).unwrap();
    assert_eq!(loaded.tuning.maps().len(), 1);
}
