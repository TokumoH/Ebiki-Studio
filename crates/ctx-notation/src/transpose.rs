//! Transposição cromática com grafia enarmônica pela armadura de destino.
//!
//! - Escopo `Score`: as armaduras também são transpostas (para a tonalidade com menos acidentes).
//! - Escopos `Part`/`Events`: as armaduras ficam; as notas são regrafadas pela armadura do compasso.
//! - Tablatura: mantém a corda se o novo traste couber; senão busca a melhor posição perto da anterior.
//! - Atômica: se alguma nota sair da faixa MIDI, nada é alterado.

use crate::error::NotationError;
use crate::model::{EventId, Note, Part, Score, TabConfig, TabPosition};
use crate::pitch::{KeySignature, Pitch};
use crate::tab;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "scope", content = "target")]
pub enum TransposeScope {
    Score,
    Part(usize),
    Events(Vec<EventId>),
}

/// Transpõe uma nota (altura + tablatura) grafando pela armadura `key`.
pub fn transpose_note(
    note: &mut Note,
    semitones: i32,
    key: &KeySignature,
    tab_cfg: Option<&TabConfig>,
) -> Result<(), NotationError> {
    let new = note.pitch.midi() + semitones;
    if !(0..=127).contains(&new) {
        return Err(NotationError::PitchOutOfRange(new));
    }
    note.pitch = Pitch::from_midi(new as u8, key);
    if let (Some(cfg), Some(old)) = (tab_cfg, note.tab) {
        let same_string = old.fret as i32 + semitones;
        let keep = TabPosition {
            string: old.string,
            fret: same_string.clamp(0, 255) as u8,
        };
        note.tab = if same_string >= 0 && tab::pitch_of(cfg, keep) == Ok(new as u8) {
            Some(keep)
        } else {
            tab::best_position(cfg, new as u8, Some(old.fret))
        };
    }
    Ok(())
}

fn transpose_part(
    part: &mut Part,
    semitones: i32,
    only: Option<&HashSet<EventId>>,
    transpose_keys: bool,
) -> Result<(), NotationError> {
    let staves = part.staves.clone();
    for m in &mut part.measures {
        if transpose_keys {
            m.key = m.key.transposed(semitones);
        }
        let key = m.key;
        for v in &mut m.voices {
            let tab_cfg = staves.get(v.staff as usize).and_then(|s| s.tab.as_ref());
            for e in &mut v.events {
                if only.is_some_and(|set| !set.contains(&e.id)) {
                    continue;
                }
                if let Some(notes) = e.notes_mut() {
                    for n in notes {
                        transpose_note(n, semitones, &key, tab_cfg)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Transpõe o escopo por `semitones` (negativo = para baixo).
pub fn transpose(
    score: &mut Score,
    scope: &TransposeScope,
    semitones: i32,
) -> Result<(), NotationError> {
    if semitones.abs() > 127 {
        return Err(NotationError::PitchOutOfRange(semitones));
    }
    match scope {
        TransposeScope::Score => {
            let mut parts = score.parts.clone();
            for p in &mut parts {
                transpose_part(p, semitones, None, true)?;
            }
            score.parts = parts;
        }
        TransposeScope::Part(i) => {
            let mut p = score.part(*i)?.clone();
            transpose_part(&mut p, semitones, None, false)?;
            score.parts[*i] = p;
        }
        TransposeScope::Events(ids) => {
            let set: HashSet<EventId> = ids.iter().copied().collect();
            for id in &set {
                if score.find_event(*id).is_none() {
                    return Err(NotationError::EventIdNotFound(id.0));
                }
            }
            let mut parts = score.parts.clone();
            for p in &mut parts {
                transpose_part(p, semitones, Some(&set), false)?;
            }
            score.parts = parts;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::Duration;
    use crate::model::*;
    use crate::pitch::{Mode, Step};

    fn score_with(notes: &[(Step, i8, i8)], staff: Staff) -> Score {
        let mut s = Score::new("T", 100.0, Meter::COMMON).unwrap();
        let mut m = Measure::new(KeySignature::c_major());
        m.voices[0].events = notes
            .iter()
            .map(|&(st, a, o)| Event::note(Duration::quarter(), Pitch::new(st, a, o).unwrap()))
            .collect();
        s.push_part(Part {
            name: "P".into(),
            program: 0,
            percussion: false,
            staves: vec![staff],
            measures: vec![m],
            spanners: vec![],
        })
        .unwrap();
        s
    }

    fn names(s: &Score) -> Vec<String> {
        s.parts[0]
            .events()
            .flat_map(|e| e.notes().iter().map(|n| n.pitch.name()))
            .collect()
    }

    #[test]
    fn score_transposition_moves_key_and_spells_with_it() {
        // C E G B em Dó → +1 = Réb maior: Db F Ab C
        let mut s = score_with(
            &[
                (Step::C, 0, 4),
                (Step::E, 0, 4),
                (Step::G, 0, 4),
                (Step::B, 0, 4),
            ],
            Staff::standard(Clef::Treble),
        );
        transpose(&mut s, &TransposeScope::Score, 1).unwrap();
        assert_eq!(
            s.parts[0].measures[0].key,
            KeySignature::new(-5, Mode::Major).unwrap()
        );
        assert_eq!(names(&s), vec!["Db4", "F4", "Ab4", "C5"]);
        transpose(&mut s, &TransposeScope::Score, 1).unwrap();
        assert_eq!(s.parts[0].measures[0].key.fifths, 2);
        assert_eq!(names(&s), vec!["D4", "F#4", "A4", "C#5"]);
        s.check_invariants().unwrap();
    }

    #[test]
    fn part_transposition_keeps_key() {
        let mut s = score_with(&[(Step::C, 0, 4)], Staff::standard(Clef::Treble));
        transpose(&mut s, &TransposeScope::Part(0), 3).unwrap();
        assert_eq!(s.parts[0].measures[0].key.fifths, 0);
        assert_eq!(names(&s), vec!["D#4"]);
    }

    #[test]
    fn selection_only_touches_selected_events() {
        let mut s = score_with(
            &[(Step::C, 0, 4), (Step::D, 0, 4)],
            Staff::standard(Clef::Treble),
        );
        let id = s.parts[0].measures[0].voices[0].events[1].id;
        transpose(&mut s, &TransposeScope::Events(vec![id]), 2).unwrap();
        assert_eq!(names(&s), vec!["C4", "E4"]);
        assert!(transpose(&mut s, &TransposeScope::Events(vec![EventId(999)]), 2).is_err());
    }

    #[test]
    fn out_of_range_is_atomic() {
        let mut s = score_with(
            &[(Step::C, 0, 4), (Step::G, 0, 9)],
            Staff::standard(Clef::Treble),
        );
        let before = s.clone();
        assert!(transpose(&mut s, &TransposeScope::Score, 12).is_err());
        assert_eq!(s, before);
    }

    #[test]
    fn tab_follows_transposition() {
        let mut s = score_with(
            &[(Step::E, 0, 2), (Step::C, 0, 5)],
            Staff::standard_and_tab(Clef::Treble8vb, TabConfig::standard_guitar()),
        );
        s.parts[0].measures[0].voices[0].events[0]
            .notes_mut()
            .unwrap()[0]
            .tab = Some(TabPosition { string: 6, fret: 0 });
        s.parts[0].measures[0].voices[0].events[1]
            .notes_mut()
            .unwrap()[0]
            .tab = Some(TabPosition { string: 1, fret: 8 });
        transpose(&mut s, &TransposeScope::Part(0), -2).unwrap();
        let ev = &s.parts[0].measures[0].voices[0].events;
        // E2 −2 não cabe na 6ª corda solta → sem posição (abaixo da afinação)
        assert_eq!(ev[0].notes()[0].tab, None);
        assert_eq!(
            ev[1].notes()[0].tab,
            Some(TabPosition { string: 1, fret: 6 })
        );
    }

    proptest::proptest! {
        /// Transpor +n e depois −n devolve as mesmas alturas MIDI; a grafia segue a armadura.
        #[test]
        fn transpose_roundtrip_preserves_midi(semis in -12i32..=12, midis in proptest::collection::vec(36u8..=96, 1..8)) {
            let mut s = Score::new("T", 100.0, Meter::COMMON).unwrap();
            let mut m = Measure::new(KeySignature::c_major());
            m.voices[0].events = midis
                .iter()
                .map(|&k| Event::note(Duration::new(crate::duration::NoteValue::Sixteenth), Pitch::from_midi(k, &KeySignature::c_major())))
                .collect();
            s.push_part(Part {
                name: "P".into(),
                program: 0,
                percussion: false,
                staves: vec![Staff::standard(Clef::Treble)],
                measures: vec![m],
                spanners: vec![],
            })
            .unwrap();
            let before: Vec<i32> = s.parts[0].events().flat_map(|e| e.notes().iter().map(|n| n.pitch.midi())).collect();
            transpose(&mut s, &TransposeScope::Score, semis).unwrap();
            let key = s.parts[0].measures[0].key;
            for e in s.parts[0].events() {
                for n in e.notes() {
                    proptest::prop_assert_eq!(Pitch::from_midi(n.pitch.midi() as u8, &key), n.pitch);
                }
            }
            transpose(&mut s, &TransposeScope::Score, -semis).unwrap();
            let after: Vec<i32> = s.parts[0].events().flat_map(|e| e.notes().iter().map(|n| n.pitch.midi())).collect();
            proptest::prop_assert_eq!(before, after);
            proptest::prop_assert_eq!(s.parts[0].measures[0].key.fifths, 0);
        }
    }
}
