mod common;

use common::*;
use ctx_notation::analysis::validate_measures;
use ctx_notation::duration::NoteValue;
use ctx_notation::model::*;
use ctx_notation::pitch::{KeySignature, Mode, Step};
use ctx_notation::playback::{render, PlaybackOptions};
use infra_notation_io::midi::*;
use infra_notation_io::{detect_format, import_any, NotationFormat};
use midly::num::{u15, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use shared_kernel::time::Tick;

fn grid_score() -> Score {
    let mut s = Score::new("Grade", 110.0, Meter::COMMON).unwrap();
    let mut m1 = Measure::new(KeySignature::new(-2, Mode::Major).unwrap());
    let mut a = Event::note(q(), p(Step::B, -1, 4));
    a.dynamic = Some(Dynamic::F);
    let mut b = Event::chord(
        d(NoteValue::Half),
        vec![Note::new(p(Step::D, 0, 5)), Note::new(p(Step::F, 0, 5))],
    );
    b.dynamic = Some(Dynamic::P);
    m1.voices[0].events = vec![
        a,
        b,
        Event::note(d(NoteValue::Eighth), p(Step::C, 0, 5)),
        Event::rest(d(NoteValue::Eighth)),
    ];
    let mut m2 = Measure::new(KeySignature::new(-2, Mode::Major).unwrap());
    m2.meter = Some(Meter::new(3, 4).unwrap());
    m2.voices[0].events = vec![Event::note(d(NoteValue::Half).dotted(1), p(Step::E, -1, 4))];
    s.push_part(Part {
        name: "Sopro".into(),
        program: 65,
        percussion: false,
        staves: vec![Staff::standard(Clef::Treble)],
        measures: vec![m1, m2],
        spanners: vec![],
    })
    .unwrap();
    let mut drums = s.blank_part("Bateria", 0, vec![Staff::standard(Clef::Percussion)]);
    drums.percussion = true;
    drums.measures[0].voices[0].events = vec![
        Event::note(q(), p(Step::C, 0, 3)),
        Event::note(q(), p(Step::D, 0, 3)),
        Event::note(q(), p(Step::C, 0, 3)),
        Event::note(q(), p(Step::D, 0, 3)),
    ];
    s.push_part(drums).unwrap();
    s.tempo.set_tempo(Tick(3840), 90.0).unwrap();
    s
}

fn keyed(s: &Score) -> Vec<(i64, i64, u8, u8, u8, u8)> {
    let mut v: Vec<_> = render(s, &PlaybackOptions::default())
        .into_iter()
        .map(|e| {
            (
                e.start_tick.0,
                e.duration_ticks.0,
                e.key,
                e.velocity,
                e.channel,
                e.program,
            )
        })
        .collect();
    v.sort();
    v
}

#[test]
fn export_import_roundtrip_keeps_performance_and_structure() {
    let s = grid_score();
    let bytes = export_midi(&s, &MidiExportOptions::default()).unwrap();
    assert_eq!(detect_format(&bytes), Some(NotationFormat::Midi));
    let imp = import_midi(&bytes, &MidiImportOptions::default()).unwrap();
    let back = imp.score;
    back.check_invariants().unwrap();
    assert_eq!(back.title, "Grade");
    assert_eq!(back.parts.len(), 2);
    assert_eq!(back.parts[0].name, "Sopro");
    assert_eq!(back.parts[0].program, 65);
    assert!(back.parts[1].percussion);
    assert_eq!(back.meters(), s.meters());
    assert_eq!(back.tempo.tempos(), s.tempo.tempos());
    assert_eq!(back.parts[0].measures[0].key.fifths, -2);
    assert!(validate_measures(&back).is_empty());
    assert_eq!(keyed(&back), keyed(&s));
    // grafia pela armadura (Sib maior): Bb, não A#
    assert_eq!(
        back.parts[0].measures[0].voices[0].events[0].notes()[0]
            .pitch
            .name(),
        "Bb4"
    );
    let any = import_any(&bytes).unwrap();
    assert_eq!(any.score.parts.len(), 2);
}

#[test]
fn repeats_are_expanded_on_export() {
    let mut s = grid_score();
    for p in &mut s.parts {
        p.measures[0].repeat_end = Some(2);
    }
    let bytes = export_midi(&s, &MidiExportOptions::default()).unwrap();
    let back = import_midi(&bytes, &MidiImportOptions::default())
        .unwrap()
        .score;
    assert_eq!(back.measure_count(), 3, "compasso 1 tocado duas vezes");
    let flat = export_midi(
        &s,
        &MidiExportOptions {
            expand_repeats: false,
        },
    )
    .unwrap();
    assert_eq!(
        import_midi(&flat, &MidiImportOptions::default())
            .unwrap()
            .score
            .measure_count(),
        2
    );
}

fn ev(delta: u32, kind: TrackEventKind<'static>) -> TrackEvent<'static> {
    TrackEvent {
        delta: u28::new(delta),
        kind,
    }
}

fn on(ch: u8, key: u8, vel: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(ch),
        message: MidiMessage::NoteOn {
            key: u7::new(key),
            vel: u7::new(vel),
        },
    }
}

fn off(ch: u8, key: u8) -> TrackEventKind<'static> {
    TrackEventKind::Midi {
        channel: u4::new(ch),
        message: MidiMessage::NoteOff {
            key: u7::new(key),
            vel: u7::new(0),
        },
    }
}

#[test]
fn imports_format0_with_quantization_and_other_resolution() {
    // 96 PPQ, formato 0, dois canais, notas levemente fora da grade, nota sem note-off
    let track = vec![
        ev(
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(6, 3, 24, 8)),
        ),
        ev(
            0,
            TrackEventKind::Midi {
                channel: u4::new(1),
                message: MidiMessage::ProgramChange {
                    program: u7::new(33),
                },
            },
        ),
        ev(2, on(0, 72, 100)),
        ev(0, on(1, 40, 90)),
        ev(46, off(0, 72)),
        ev(0, on(0, 74, 100)),
        ev(50, on(0, 74, 0)),
        ev(0, on(0, 76, 60)),
        ev(140, off(1, 40)),
        ev(100, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
    ];
    let mut smf = Smf::new(Header::new(
        Format::SingleTrack,
        Timing::Metrical(u15::new(96)),
    ));
    smf.tracks.push(track);
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes).unwrap();
    let imp = import_midi(
        &bytes,
        &MidiImportOptions {
            grid_ticks: 480,
            ..Default::default()
        },
    )
    .unwrap();
    let s = imp.score;
    s.check_invariants().unwrap();
    assert_eq!(s.default_meter(), Meter::new(6, 8).unwrap());
    assert_eq!(s.parts.len(), 2);
    assert!(imp.warnings.iter().any(|w| w.contains("sem fim")));
    let mel = &s.parts[0];
    let first = &mel.measures[0].voices[0].events[0];
    assert_eq!(
        first.duration,
        d(NoteValue::Eighth),
        "quantizado para colcheia"
    );
    assert_eq!(first.notes()[0].pitch.midi(), 72);
    let bass = &s.parts[1];
    assert_eq!(bass.program, 33);
    assert!(bass.staves[0].tab.is_some(), "baixo ganha tablatura");
    assert!(validate_measures(&s).is_empty());
}

#[test]
fn garbage_and_truncated_midi_never_panics() {
    let bytes = export_midi(&grid_score(), &MidiExportOptions::default()).unwrap();
    for cut in 0..bytes.len() {
        let _ = import_midi(&bytes[..cut], &MidiImportOptions::default());
    }
    let mut flipped = bytes.clone();
    for i in (14..flipped.len()).step_by(3) {
        flipped[i] ^= 0x5A;
        let _ = import_midi(&flipped, &MidiImportOptions::default());
    }
}
