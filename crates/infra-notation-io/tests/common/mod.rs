//! Partituras sintéticas compartilhadas pelos testes de integração.
#![allow(dead_code)]

use ctx_notation::duration::{Duration, NoteValue, Tuplet};
use ctx_notation::model::*;
use ctx_notation::pitch::{KeySignature, Mode, Pitch, Step};
use shared_kernel::time::Tick;

pub fn p(step: Step, alter: i8, oct: i8) -> Pitch {
    Pitch::new(step, alter, oct).unwrap()
}

pub fn q() -> Duration {
    Duration::quarter()
}

pub fn d(v: NoteValue) -> Duration {
    Duration::new(v)
}

/// Partitura rica: piano em pauta dupla + violão com tablatura.
pub fn rich_score() -> Score {
    let mut s = Score::new("Peça <Teste> & Cia", 90.0, Meter::COMMON).unwrap();
    s.composer = "Compositora Ç".into();
    let g = KeySignature::new(1, Mode::Major).unwrap();
    let bb_minor = KeySignature::new(-5, Mode::Minor).unwrap();

    // ── Piano: compasso 1 (4/4, Sol maior)
    let mut m1 = Measure::new(g);
    m1.repeat_start = true;
    let mut e1 = Event::note(q().dotted(1), p(Step::G, 0, 4));
    e1.dynamic = Some(Dynamic::Mp);
    e1.articulations = vec![Articulation::Staccato, Articulation::Accent];
    e1.lyric = Some(Lyric {
        text: "lá".into(),
        syllabic: Syllabic::Single,
    });
    let mut grace = Event::note(d(NoteValue::Sixteenth), p(Step::B, 0, 4));
    grace.grace = Some(GraceKind::Acciaccatura);
    let e2 = Event::note(d(NoteValue::Eighth), p(Step::A, 0, 4));
    let trip = d(NoteValue::Eighth).with_tuplet(Tuplet::TRIPLET);
    let mut t1 = Event::note(trip, p(Step::F, 1, 4));
    t1.articulations = vec![Articulation::Tenuto];
    let t2 = Event::note(trip, p(Step::E, 0, 4));
    let mut t3 = Event::note(trip, p(Step::D, 0, 4));
    t3.articulations = vec![Articulation::Marcato];
    let mut tied = Event::chord(
        q(),
        vec![Note::new(p(Step::C, 0, 5)), Note::new(p(Step::E, 0, 5))],
    );
    for n in tied.notes_mut().unwrap() {
        n.tie = true;
    }
    m1.voices[0].events = vec![e1, grace, e2, t1, t2, t3, tied];
    let mut lh = Voice::new(1);
    lh.events = vec![
        Event::chord(
            d(NoteValue::Half),
            vec![Note::new(p(Step::G, 0, 2)), Note::new(p(Step::D, 0, 3))],
        ),
        Event::rest(d(NoteValue::Half)),
    ];
    m1.voices.push(lh);

    // compasso 2 (3/4, Sib menor, fim de repetição 3x)
    let mut m2 = Measure::new(bb_minor);
    m2.meter = Some(Meter::new(3, 4).unwrap());
    m2.repeat_end = Some(3);
    let mut a = Event::chord(
        q(),
        vec![Note::new(p(Step::C, 0, 5)), Note::new(p(Step::E, 0, 5))],
    );
    a.dynamic = Some(Dynamic::Ff);
    let mut fermata = Event::note(d(NoteValue::Half), p(Step::D, -1, 5));
    fermata.articulations = vec![Articulation::Fermata];
    m2.voices[0].events = vec![a, fermata];
    let mut lh2 = Voice::new(1);
    lh2.events = vec![Event::rest(d(NoteValue::Half).dotted(1))];
    m2.voices.push(lh2);
    let mut v3 = Voice::new(0);
    v3.events = vec![
        Event::rest(q()),
        Event::note(d(NoteValue::Eighth).dotted(2), p(Step::B, -1, 4)),
        Event::note(d(NoteValue::ThirtySecond), p(Step::A, 0, 4)),
        Event::note(q(), p(Step::G, -1, 4)),
    ];
    m2.voices.push(v3);

    let piano = Part {
        name: "Piano".into(),
        program: 0,
        percussion: false,
        staves: vec![Staff::standard(Clef::Treble), Staff::standard(Clef::Bass)],
        measures: vec![m1, m2],
        spanners: vec![],
    };
    s.push_part(piano).unwrap();
    let ids: Vec<EventId> = s.parts[0].measures[0].voices[0]
        .events
        .iter()
        .map(|e| e.id)
        .collect();
    s.parts[0].spanners = vec![
        Spanner {
            kind: SpannerKind::Slur,
            start: ids[2],
            end: ids[5],
        },
        Spanner {
            kind: SpannerKind::Crescendo,
            start: ids[3],
            end: ids[6],
        },
    ];

    // ── Violão com tablatura (capotraste 2)
    let mut tab = TabConfig::standard_guitar();
    tab.capo = 2;
    let mut gm1 = Measure::new(g);
    gm1.repeat_start = true;
    let tn = |pitch: Pitch, string: u8, fret: u8, techs: Vec<Technique>| Note {
        pitch,
        tie: false,
        tab: Some(TabPosition { string, fret }),
        techniques: techs,
    };
    gm1.voices[0].events = vec![
        Event::chord(
            q(),
            vec![tn(
                p(Step::A, 0, 2),
                6,
                3,
                vec![Technique::HammerOn, Technique::PalmMute],
            )],
        ),
        Event::chord(
            q(),
            vec![tn(p(Step::B, 0, 2), 5, 0, vec![Technique::PullOff])],
        ),
        Event::chord(
            q(),
            vec![tn(
                p(Step::C, 1, 3),
                5,
                2,
                vec![Technique::Bend { cents: 150 }, Technique::Slide],
            )],
        ),
        Event::chord(
            q(),
            vec![tn(
                p(Step::E, 0, 4),
                3,
                7,
                vec![
                    Technique::Harmonic {
                        harmonic: HarmonicKind::Natural,
                    },
                    Technique::LetRing,
                    Technique::Vibrato,
                ],
            )],
        ),
    ];
    let mut gm2 = Measure::new(bb_minor);
    gm2.meter = Some(Meter::new(3, 4).unwrap());
    gm2.repeat_end = Some(3);
    gm2.voices[0].events = vec![Event::chord(
        d(NoteValue::Half).dotted(1),
        vec![
            tn(p(Step::G, 0, 3), 4, 3, vec![]),
            tn(p(Step::B, 0, 3), 3, 2, vec![]),
        ],
    )];
    s.push_part(Part {
        name: "Violão".into(),
        program: 25,
        percussion: false,
        staves: vec![Staff::standard_and_tab(Clef::Treble8vb, tab)],
        measures: vec![gm1, gm2],
        spanners: vec![],
    })
    .unwrap();
    s.tempo.set_tempo(Tick(3840), 72.0).unwrap();
    s.check_invariants().unwrap();
    s
}

/// Forma canônica para comparação (ids renumerados, spanners ordenados).
pub fn canonical(mut s: Score) -> Score {
    s.renumber_events();
    for p in &mut s.parts {
        p.spanners
            .sort_by_key(|sp| (sp.start, sp.end, sp.kind as u8));
    }
    s
}
