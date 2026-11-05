//! Tablatura ⇄ altura: altura a partir de corda/traste/capotraste e melhor posição para uma altura.
//!
//! Convenção: corda 1 = mais aguda (`tuning[0]`); traste relativo ao capotraste
//! (`altura = afinação + capotraste + traste`).

use crate::error::NotationError;
use crate::model::{TabConfig, TabPosition};

pub fn check_position(tab: &TabConfig, pos: TabPosition) -> Result<(), NotationError> {
    if pos.string == 0 || pos.string as usize > tab.tuning.len() {
        return Err(NotationError::InvalidTab("corda inexistente"));
    }
    if pos.fret as u16 + tab.capo as u16 > tab.frets as u16 {
        return Err(NotationError::InvalidTab("traste além do braço"));
    }
    Ok(())
}

/// Altura MIDI de uma posição.
pub fn pitch_of(tab: &TabConfig, pos: TabPosition) -> Result<u8, NotationError> {
    check_position(tab, pos)?;
    let m = tab.tuning[pos.string as usize - 1] as i32 + tab.capo as i32 + pos.fret as i32;
    if m > 127 {
        return Err(NotationError::PitchOutOfRange(m));
    }
    Ok(m as u8)
}

/// Todas as posições possíveis de uma altura (ordenadas por corda).
pub fn positions_for(tab: &TabConfig, midi: u8) -> Vec<TabPosition> {
    let mut out = Vec::new();
    for (i, &open) in tab.tuning.iter().enumerate() {
        let fret = midi as i32 - open as i32 - tab.capo as i32;
        if fret >= 0 && fret + tab.capo as i32 <= tab.frets as i32 {
            out.push(TabPosition {
                string: (i + 1) as u8,
                fret: fret as u8,
            });
        }
    }
    out
}

/// Melhor posição: mais próxima da mão atual (`hand_fret`, traste de referência) ou, sem referência,
/// a de menor traste (cordas soltas preferidas).
pub fn best_position(tab: &TabConfig, midi: u8, hand_fret: Option<u8>) -> Option<TabPosition> {
    positions_for(tab, midi).into_iter().min_by_key(|p| {
        let dist = match hand_fret {
            Some(h) if p.fret != 0 => (p.fret as i32 - h as i32).abs(),
            Some(_) => 1,
            None => p.fret as i32,
        };
        (dist, p.fret, p.string)
    })
}

/// Distribui um acorde em cordas distintas minimizando a abertura (traste máx − mín, ignorando soltas)
/// e depois a soma dos trastes. `None` se impossível.
pub fn assign_chord(tab: &TabConfig, notes: &[u8]) -> Option<Vec<TabPosition>> {
    if notes.len() > tab.tuning.len() {
        return None;
    }
    let options: Vec<Vec<TabPosition>> = notes.iter().map(|&n| positions_for(tab, n)).collect();
    if options.iter().any(Vec::is_empty) {
        return None;
    }
    let mut best: Option<((i32, i32), Vec<TabPosition>)> = None;
    let mut current = Vec::with_capacity(notes.len());
    search(&options, 0, &mut current, &mut best);
    best.map(|(_, v)| v)
}

fn cost(ps: &[TabPosition]) -> (i32, i32) {
    let fretted: Vec<i32> = ps
        .iter()
        .filter(|p| p.fret > 0)
        .map(|p| p.fret as i32)
        .collect();
    let span = match (fretted.iter().max(), fretted.iter().min()) {
        (Some(a), Some(b)) => a - b,
        _ => 0,
    };
    (span, ps.iter().map(|p| p.fret as i32).sum())
}

fn search(
    options: &[Vec<TabPosition>],
    i: usize,
    current: &mut Vec<TabPosition>,
    best: &mut Option<((i32, i32), Vec<TabPosition>)>,
) {
    if i == options.len() {
        let c = cost(current);
        if best.as_ref().is_none_or(|(b, _)| c < *b) {
            *best = Some((c, current.clone()));
        }
        return;
    }
    for p in &options[i] {
        if current.iter().any(|q| q.string == p.string) {
            continue;
        }
        current.push(*p);
        search(options, i + 1, current, best);
        current.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn tp(string: u8, fret: u8) -> TabPosition {
        TabPosition { string, fret }
    }

    #[test]
    fn pitch_from_string_fret_capo() {
        let mut g = TabConfig::standard_guitar();
        assert_eq!(pitch_of(&g, tp(6, 0)).unwrap(), 40);
        assert_eq!(pitch_of(&g, tp(1, 5)).unwrap(), 69);
        g.capo = 2;
        assert_eq!(pitch_of(&g, tp(6, 0)).unwrap(), 42);
        assert!(pitch_of(&g, tp(7, 0)).is_err());
        assert!(pitch_of(&g, tp(1, 23)).is_err());
    }

    #[test]
    fn best_position_prefers_low_frets_or_hand() {
        let g = TabConfig::standard_guitar();
        assert_eq!(best_position(&g, 64, None), Some(tp(1, 0)));
        assert_eq!(best_position(&g, 64, Some(9)), Some(tp(3, 9)));
        assert_eq!(best_position(&g, 30, None), None);
    }

    #[test]
    fn chord_assignment_uses_distinct_strings() {
        let g = TabConfig::standard_guitar();
        // Dó maior aberto: C3 E3 G3 C4 E4
        let pos = assign_chord(&g, &[48, 52, 55, 60, 64]).unwrap();
        let mut strings: Vec<u8> = pos.iter().map(|p| p.string).collect();
        strings.sort();
        strings.dedup();
        assert_eq!(strings.len(), 5);
        for (p, n) in pos.iter().zip([48, 52, 55, 60, 64]) {
            assert_eq!(pitch_of(&g, *p).unwrap(), n);
        }
        assert!(assign_chord(&g, &[40, 40, 40, 40, 40, 40, 40]).is_none());
    }

    proptest! {
        #[test]
        fn every_position_maps_back(midi in 0u8..=127, capo in 0u8..5) {
            let mut g = TabConfig::standard_guitar();
            g.capo = capo;
            for p in positions_for(&g, midi) {
                prop_assert_eq!(pitch_of(&g, p).unwrap(), midi);
            }
        }
    }
}
