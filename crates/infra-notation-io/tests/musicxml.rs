mod common;

use common::*;
use ctx_notation::analysis::validate_measures;
use ctx_notation::duration::{Duration, NoteValue};
use ctx_notation::model::*;
use ctx_notation::playback::{render, PlaybackOptions};
use infra_notation_io::musicxml::*;
use infra_notation_io::{detect_format, import_any, NotationFormat};
use shared_kernel::time::Tick;

#[test]
fn musicxml_roundtrip_preserves_rich_score() {
    let s = rich_score();
    let xml = export_musicxml(&s);
    assert!(xml.contains("<score-partwise"));
    assert!(xml.contains("Peça &lt;Teste&gt; &amp; Cia"));
    let back = import_musicxml(xml.as_bytes()).unwrap();
    assert!(back.warnings.is_empty(), "{:?}", back.warnings);
    assert_eq!(canonical(back.score), canonical(s));
}

#[test]
fn mxl_roundtrip_and_detection() {
    let s = rich_score();
    let z = export_mxl(&s).unwrap();
    assert_eq!(detect_format(&z), Some(NotationFormat::Mxl));
    let back = import_mxl(&z).unwrap();
    assert_eq!(canonical(back.score), canonical(s.clone()));
    let any = import_any(&z).unwrap();
    assert_eq!(canonical(any.score), canonical(s));
}

#[test]
fn roundtrip_keeps_playback_identical() {
    let s = rich_score();
    let back = import_musicxml(export_musicxml(&s).as_bytes())
        .unwrap()
        .score;
    assert_eq!(
        render(&s, &PlaybackOptions::default()),
        render(&back, &PlaybackOptions::default())
    );
}

#[test]
fn imports_hand_written_sample() {
    let bytes = include_bytes!("data/sample.musicxml");
    assert_eq!(detect_format(bytes), Some(NotationFormat::MusicXml));
    let imp = import_musicxml(bytes).unwrap();
    let s = &imp.score;
    assert_eq!(s.title, "Valsa de Teste");
    assert_eq!(s.composer, "Fulano");
    assert_eq!(s.parts.len(), 2);
    assert_eq!(s.default_meter(), Meter::new(3, 4).unwrap());
    assert_eq!(s.tempo.bpm_at(Tick(0)), 96.0);
    assert!(
        validate_measures(s).is_empty(),
        "{:?}",
        validate_measures(s)
    );

    let fl = &s.parts[0];
    assert_eq!(fl.name, "Flauta");
    assert_eq!(fl.program, 73);
    assert_eq!(fl.measures[0].key.fifths, 1);
    let ev = &fl.measures[0].voices[0].events;
    assert_eq!(ev.len(), 4);
    assert_eq!(ev[0].dynamic, Some(Dynamic::P));
    assert_eq!(ev[0].articulations, vec![Articulation::Staccato]);
    assert_eq!(ev[0].lyric.as_ref().unwrap().text, "Can");
    assert_eq!(ev[1].lyric.as_ref().unwrap().syllabic, Syllabic::End);
    assert_eq!(ev[2].notes()[0].pitch.name(), "F#4");
    assert!(ev[3].notes()[0].tie);
    assert_eq!(fl.measures[1].voices.len(), 2);
    assert_eq!(
        fl.measures[1].voices[1].events[0].duration,
        Duration::new(NoteValue::Half).dotted(1)
    );
    assert_eq!(fl.measures[1].repeat_end, Some(2));

    let gt = &s.parts[1];
    let tab = gt.staves[0].tab.as_ref().unwrap();
    assert!(!gt.staves[0].standard, "clave TAB = só tablatura");
    assert_eq!(tab.tuning, vec![64, 59, 55, 50, 45, 40]);
    assert_eq!(tab.capo, 2);
    let gev = &gt.measures[0].voices[0].events;
    assert_eq!(
        gev[0].notes()[0].tab,
        Some(TabPosition { string: 6, fret: 3 })
    );
    assert_eq!(gev[1].notes()[0].techniques, vec![Technique::HammerOn]);
    assert_eq!(
        gev[2].notes()[0].techniques,
        vec![Technique::Bend { cents: 100 }]
    );
    assert_eq!(gt.measures[1].voices[0].events[0].notes().len(), 2);
    // a tablatura corresponde à altura escrita
    for e in gt.events() {
        for n in e.notes() {
            if let Some(pos) = n.tab {
                assert_eq!(
                    ctx_notation::tab::pitch_of(tab, pos).unwrap() as i32,
                    n.pitch.midi()
                );
            }
        }
    }
    // reprodução: ligadura funde Si4 (1 tempo + 1 tempo); repetição duplica os compassos
    let pb = render(s, &PlaybackOptions::default());
    let b4: Vec<_> = pb.iter().filter(|e| e.key == 71).collect();
    assert_eq!(b4.len(), 2);
    assert_eq!(b4[0].duration_ticks, Tick(1920));
}

#[test]
fn malformed_inputs_are_errors_not_panics() {
    let cases: [&[u8]; 6] = [
        b"<score-partwise><part-list></score-partwise>",
        b"<score-partwise><part id=\"P1\"><measure><note><pitch><step>Z</step></pitch><duration>4</duration></note></measure></part></score-partwise>",
        b"<?xml version=\"1.0\"?><!DOCTYPE lol [<!ENTITY a \"aaaa\">]><score-partwise>&a;</score-partwise>",
        b"\xff\xfe\x00<",
        b"<score-partwise><part><measure><attributes><divisions>0</divisions><time><beats>99</beats><beat-type>3</beat-type></time></attributes><note><duration>-5</duration></note></measure></part></score-partwise>",
        b"<score-partwise><part><measure><note><rest/><duration>99999999999</duration><voice>1</voice></note></measure></part></score-partwise>",
    ];
    // estruturalmente inválidos / não-XML / entidades proibidas: erro tipado
    for i in [0, 2, 3] {
        assert!(
            import_musicxml(cases[i]).is_err(),
            "caso {i} deveria ser erro"
        );
    }
    // nota com altura inválida: importação tolerante, nota descartada com aviso
    let w = import_musicxml(cases[1]).unwrap().warnings;
    assert!(w.iter().any(|m| m.contains("altura inválida")), "{w:?}");
    // valores absurdos em XML válido: importa saneado ou falha, mas nunca entra em pânico
    // e, se importar, a partitura resultante é consistente
    for c in &cases[4..] {
        if let Ok(imported) = import_musicxml(c) {
            for part in &imported.score.parts {
                assert!(part.validate_header().is_ok());
                for e in part
                    .measures
                    .iter()
                    .flat_map(|m| &m.voices)
                    .flat_map(|v| &v.events)
                {
                    assert!(e.validate().is_ok(), "evento inválido importado: {e:?}");
                }
            }
        }
    }
}

#[test]
fn imported_partial_files_stay_consistent() {
    // parte 2 com menos compassos e armadura diferente é igualada
    let xml = br#"<score-partwise>
      <part-list><score-part id="A"><part-name>A</part-name></score-part><score-part id="B"><part-name>B</part-name></score-part></part-list>
      <part id="A">
        <measure><attributes><divisions>1</divisions><key><fifths>2</fifths></key></attributes>
          <note><pitch><step>D</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note></measure>
        <measure><note><rest/><duration>4</duration><type>whole</type></note></measure>
      </part>
      <part id="B">
        <measure><attributes><divisions>1</divisions></attributes>
          <note><pitch><step>A</step><octave>3</octave></pitch><duration>4</duration></note></measure>
      </part>
    </score-partwise>"#;
    let imp = import_musicxml(xml).unwrap();
    imp.score.check_invariants().unwrap();
    assert_eq!(imp.score.parts[1].measures.len(), 2);
    assert_eq!(imp.score.parts[1].measures[0].key.fifths, 2);
    assert!(!imp.warnings.is_empty());
    // nota sem <type> deduzida da duração
    assert_eq!(
        imp.score.parts[1].measures[0].voices[0].events[0].duration,
        Duration::new(NoteValue::Whole)
    );
}
