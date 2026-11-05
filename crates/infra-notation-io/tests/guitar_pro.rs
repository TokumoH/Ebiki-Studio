use ctx_notation::analysis::validate_measures;
use ctx_notation::duration::{Duration, NoteValue, Tuplet};
use ctx_notation::model::*;
use ctx_notation::pitch::Mode;
use ctx_notation::playback::{performance_order, render, PlaybackOptions};
use infra_notation_io::archive::write_zip;
use infra_notation_io::gp::*;
use infra_notation_io::{detect_format, import_any, NotationFormat};
use shared_kernel::time::Tick;

const GPIF: &[u8] = include_bytes!("data/sample.gpif");

fn gp_file() -> Vec<u8> {
    write_zip(&[("VERSION", b"7.0", true), (GPIF_ENTRY, GPIF, false)]).unwrap()
}

#[test]
fn imports_gp7_core_subset() {
    let bytes = gp_file();
    assert_eq!(detect_format(&bytes), Some(NotationFormat::GuitarPro7));
    let imp = import_gp(&bytes).unwrap();
    let s = &imp.score;
    s.check_invariants().unwrap();
    assert_eq!(s.title, "Riff de Teste");
    assert_eq!(s.composer, "Compositora");
    assert_eq!(s.tempo.bpm_at(Tick(0)), 100.0);
    assert_eq!(s.tempo.bpm_at(Tick(3840)), 140.0);
    assert_eq!(s.meter_at(1), Meter::new(3, 4).unwrap());
    assert!(
        validate_measures(s).is_empty(),
        "{:?}",
        validate_measures(s)
    );

    let p = &s.parts[0];
    assert_eq!(p.name, "Guitarra");
    assert_eq!(p.program, 29);
    assert_eq!(
        p.staves[0].tab.as_ref().unwrap().tuning,
        vec![64, 59, 55, 50, 45, 40]
    );
    assert_eq!(p.measures[0].key.fifths, -1);
    assert_eq!(p.measures[0].key.mode, Mode::Minor);
    assert!(p.measures[0].repeat_start);
    assert_eq!(p.measures[1].repeat_end, Some(3));

    let e = &p.measures[0].voices[0].events;
    assert_eq!(e.len(), 5);
    assert_eq!(e[0].duration, Duration::quarter().dotted(1));
    assert_eq!(e[0].dynamic, Some(Dynamic::F));
    assert_eq!(e[1].dynamic, None, "dinâmica repetida não é remarcada");
    assert_eq!(e[0].articulations, vec![Articulation::Accent]);
    let n0 = &e[0].notes()[0];
    assert_eq!(n0.pitch.midi(), 45);
    assert_eq!(n0.tab, Some(TabPosition { string: 6, fret: 5 }));
    assert_eq!(n0.techniques, vec![Technique::PalmMute]);
    assert_eq!(
        e[1].notes()[0].techniques,
        vec![Technique::PullOff],
        "D3 → C3 é pull-off"
    );
    assert_eq!(
        e[2].duration,
        Duration::new(NoteValue::Eighth).with_tuplet(Tuplet::TRIPLET)
    );
    assert_eq!(
        e[3].notes()[0].techniques,
        vec![Technique::Bend { cents: 200 }, Technique::Vibrato]
    );
    assert!(e[4].notes()[0].tie);

    let e2 = &p.measures[1].voices[0].events;
    assert_eq!(e2[0].dynamic, Some(Dynamic::Mp));
    assert_eq!(e2[0].notes().len(), 2);
    assert!(e2[0].notes()[0].techniques.contains(&Technique::LetRing));
    assert!(e2[0].notes()[1].techniques.contains(&Technique::Slide));
    assert!(e2[1].is_rest());

    // reprodução: ligadura C4 (colcheia de tercina + mínima) e repetição 3x
    let order: Vec<usize> = performance_order(s, true)
        .iter()
        .map(|m| m.measure)
        .collect();
    assert_eq!(order, vec![0, 1, 0, 1, 0, 1]);
    let pb = render(s, &PlaybackOptions::default());
    let c4 = pb.iter().find(|n| n.key == 60).unwrap();
    assert_eq!(c4.duration_ticks, Tick(320 + 1920));
}

#[test]
fn raw_gpif_and_auto_detection() {
    assert_eq!(detect_format(GPIF), Some(NotationFormat::GuitarPro7));
    let a = import_gpif(GPIF).unwrap();
    let b = import_any(&gp_file()).unwrap();
    assert_eq!(a.score, b.score);
}

#[test]
fn gp_without_gpif_entry_is_unsupported() {
    let z = write_zip(&[("score.gpx", b"BCFZ", false)]).unwrap();
    assert!(import_gp(&z).is_err());
}

#[test]
fn mutated_gpif_never_panics() {
    let text = String::from_utf8_lossy(GPIF).into_owned();
    let mutations = [
        ("<Fret>5</Fret>", "<Fret>999</Fret>"),
        ("<String>3</String>", "<String>42</String>"),
        ("40 45 50 55 59 64", "x y 300"),
        ("<Time>3/4</Time>", "<Time>0/7</Time>"),
        ("<Value>100 2</Value>", "<Value>NaN</Value>"),
        ("<Beats>5 6</Beats>", "<Beats>5 5 5 6 99</Beats>"),
        ("num=\"3\"", "num=\"0\""),
        (
            "<AccidentalCount>-1</AccidentalCount>",
            "<AccidentalCount>-40</AccidentalCount>",
        ),
        ("</Notes>", ""),
    ];
    for (a, b) in mutations {
        let m = text.replace(a, b);
        let _ = import_gpif(m.as_bytes());
    }
}
