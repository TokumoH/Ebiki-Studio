//! Realização de progressões: acordes → notas MIDI (voicing fechado) + tempo em ticks.
//!
//! Serve para **ouvir** a progressão (RF-5.5), **arrastá-la** para a timeline (RF-5.6) e gerar
//! partitura (Fase 7). Função pura e determinística.

use super::chord::Chord;
use super::error::HarmonyError;
use super::progression::Progression;
use super::scale::Key;
use serde::{Deserialize, Serialize};
use shared_kernel::music::{MidiNote, PitchClass};
use shared_kernel::{Tick, PPQ};

/// Acorde com duração em tempos (semínimas).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimedChord {
    pub chord: Chord,
    pub beats: u16,
}

/// Opções de voicing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicingOptions {
    /// Nota mais grave permitida para as vozes superiores.
    pub low: u8,
    /// Nota mais aguda permitida para as vozes superiores.
    pub high: u8,
    /// Escolhe a inversão que minimiza o movimento das vozes em relação ao acorde anterior.
    pub voice_leading: bool,
    /// Faixa do baixo separado (fundamental ou baixo da cifra). `None` = sem baixo separado.
    pub bass_range: Option<(u8, u8)>,
    /// Posição inicial na timeline.
    pub start: Tick,
}

impl Default for VoicingOptions {
    fn default() -> Self {
        Self {
            low: 52,  // E3
            high: 76, // E5
            voice_leading: true,
            bass_range: Some((36, 52)), // C2–E3
            start: Tick::ZERO,
        }
    }
}

impl VoicingOptions {
    pub fn validate(&self) -> Result<(), HarmonyError> {
        let range_ok = |lo: u8, hi: u8| (21..=108).contains(&lo) && hi <= 108 && hi >= lo + 12;
        if !range_ok(self.low, self.high) {
            return Err(HarmonyError::InvalidVoicingRange);
        }
        if let Some((lo, hi)) = self.bass_range {
            if !range_ok(lo, hi) {
                return Err(HarmonyError::InvalidVoicingRange);
            }
        }
        if self.start.0 < 0 {
            return Err(HarmonyError::InvalidStep("início negativo".into()));
        }
        Ok(())
    }
}

/// Acorde realizado: notas e posição em ticks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizedChord {
    pub chord: Chord,
    pub start: Tick,
    pub duration: Tick,
    /// Vozes superiores, em ordem crescente.
    pub notes: Vec<MidiNote>,
    pub bass: Option<MidiNote>,
}

/// Nota mais grave `>= low` com a classe de altura dada.
fn lowest_at_or_above(pc: PitchClass, low: u8) -> u8 {
    let up = (pc.value() as i32 - (low % 12) as i32).rem_euclid(12) as u8;
    low + up
}

/// Empilha as classes de altura em posição fechada começando em `start_note`.
fn stack(pcs: &[PitchClass], start_note: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(pcs.len());
    let mut prev = start_note;
    for (i, pc) in pcs.iter().enumerate() {
        let n = if i == 0 {
            start_note
        } else {
            lowest_at_or_above(*pc, prev + 1)
        };
        out.push(n);
        prev = n;
    }
    out
}

/// Todas as posições fechadas (inversões × oitavas) que cabem em `[low, high]`.
fn candidates(pcs: &[PitchClass], low: u8, high: u8) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for rot in 0..pcs.len() {
        let rotated: Vec<PitchClass> = pcs[rot..].iter().chain(&pcs[..rot]).copied().collect();
        let mut first = lowest_at_or_above(rotated[0], low);
        while first <= high {
            let v = stack(&rotated, first);
            if v.last().is_some_and(|n| *n <= high) {
                out.push(v);
            }
            first += 12;
        }
    }
    out
}

/// Reduz as notas até caber: primeiro tensões, depois a quinta justa.
fn reductions(chord: &Chord) -> Vec<Vec<PitchClass>> {
    let full = chord.pitch_classes();
    let structure_len = chord.quality.intervals().len().min(full.len());
    let mut out = vec![full.clone()];
    let mut reduced: Vec<PitchClass> = full[..structure_len].to_vec();
    if reduced != full {
        out.push(reduced.clone());
    }
    let fifth = chord.root.transpose(7);
    if reduced.len() > 2 {
        if let Some(i) = reduced.iter().position(|p| *p == fifth) {
            reduced.remove(i);
            out.push(reduced);
        }
    }
    out
}

fn movement(prev: &[u8], next: &[u8]) -> u32 {
    next.iter()
        .map(|n| {
            prev.iter()
                .map(|p| (*n as i32 - *p as i32).unsigned_abs())
                .min()
                .unwrap_or(0)
        })
        .sum::<u32>()
        + prev
            .iter()
            .map(|p| {
                next.iter()
                    .map(|n| (*n as i32 - *p as i32).unsigned_abs())
                    .min()
                    .unwrap_or(0)
            })
            .sum::<u32>()
}

fn voice(chord: &Chord, opts: &VoicingOptions, prev: Option<&[u8]>) -> Vec<u8> {
    for pcs in reductions(chord) {
        let cands = candidates(&pcs, opts.low, opts.high);
        if cands.is_empty() {
            continue;
        }
        let chosen = match prev {
            Some(p) if opts.voice_leading => cands
                .into_iter()
                .min_by_key(|c| (movement(p, c), c[0]))
                .unwrap_or_default(),
            // Sem condução: posição fundamental mais grave que couber (ou a primeira inversão).
            _ => cands.into_iter().next().unwrap_or_default(),
        };
        return chosen;
    }
    // Faixa validada tem ≥ 1 oitava: ao menos fundamental + terça cabem; fallback seguro.
    vec![lowest_at_or_above(chord.root, opts.low)]
}

/// Realiza uma sequência de acordes com durações.
pub fn realize(
    chords: &[TimedChord],
    opts: &VoicingOptions,
) -> Result<Vec<RealizedChord>, HarmonyError> {
    opts.validate()?;
    let mut out = Vec::with_capacity(chords.len());
    let mut at = opts.start;
    let mut prev: Option<Vec<u8>> = None;
    for tc in chords {
        if tc.beats == 0 {
            return Err(HarmonyError::InvalidDuration);
        }
        let notes = voice(&tc.chord, opts, prev.as_deref());
        let bass = opts.bass_range.map(|(lo, _)| {
            let pc = tc.chord.bass.unwrap_or(tc.chord.root);
            MidiNote(lowest_at_or_above(pc, lo))
        });
        let duration = Tick(tc.beats as i64 * PPQ);
        out.push(RealizedChord {
            chord: tc.chord.clone(),
            start: at,
            duration,
            notes: notes.iter().map(|n| MidiNote(*n)).collect(),
            bass,
        });
        at = at + duration;
        prev = Some(notes);
    }
    Ok(out)
}

impl Progression {
    /// Realiza a progressão na tônica dada.
    pub fn realize(
        &self,
        tonic_key: Key,
        opts: &VoicingOptions,
    ) -> Result<Vec<RealizedChord>, HarmonyError> {
        let chords = self.chords_in(tonic_key)?;
        let timed: Vec<TimedChord> = chords
            .into_iter()
            .zip(&self.steps)
            .map(|(chord, s)| TimedChord {
                chord,
                beats: s.beats,
            })
            .collect();
        realize(&timed, opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::catalog::builtin_catalog;
    use proptest::prelude::*;

    fn tc(s: &str, beats: u16) -> TimedChord {
        TimedChord {
            chord: Chord::parse(s).unwrap(),
            beats,
        }
    }

    #[test]
    fn closed_root_position_without_voice_leading() {
        let opts = VoicingOptions {
            voice_leading: false,
            bass_range: None,
            low: 60,
            high: 84,
            ..Default::default()
        };
        let r = realize(&[tc("C", 4), tc("G7", 2)], &opts).unwrap();
        assert_eq!(r[0].notes, vec![MidiNote(60), MidiNote(64), MidiNote(67)]);
        assert_eq!(
            r[1].notes,
            vec![MidiNote(67), MidiNote(71), MidiNote(74), MidiNote(77)]
        );
        assert_eq!(r[0].start, Tick(0));
        assert_eq!(r[0].duration, Tick(4 * PPQ));
        assert_eq!(r[1].start, Tick(4 * PPQ));
        assert_eq!(r[1].duration, Tick(2 * PPQ));
        assert_eq!(r[0].bass, None);
    }

    #[test]
    fn slash_chord_bass_and_offset_start() {
        let opts = VoicingOptions {
            start: Tick(960 * 8),
            ..Default::default()
        };
        let r = realize(&[tc("G/B", 4)], &opts).unwrap();
        assert_eq!(r[0].bass, Some(MidiNote(47)), "Si 2");
        assert_eq!(r[0].start, Tick(960 * 8));
    }

    #[test]
    fn voice_leading_reduces_movement() {
        let chords = [tc("C", 4), tc("F", 4), tc("G7", 4), tc("C", 4), tc("Am", 4)];
        let total = |vl: bool| {
            let opts = VoicingOptions {
                voice_leading: vl,
                ..Default::default()
            };
            let r = realize(&chords, &opts).unwrap();
            r.windows(2)
                .map(|w| {
                    let a: Vec<u8> = w[0].notes.iter().map(|n| n.0).collect();
                    let b: Vec<u8> = w[1].notes.iter().map(|n| n.0).collect();
                    movement(&a, &b)
                })
                .sum::<u32>()
        };
        assert!(
            total(true) < total(false),
            "{} !< {}",
            total(true),
            total(false)
        );
    }

    #[test]
    fn rich_chords_are_reduced_to_fit_narrow_range() {
        let opts = VoicingOptions {
            low: 60,
            high: 72,
            voice_leading: false,
            bass_range: None,
            ..Default::default()
        };
        let r = realize(&[tc("C7(9,13)", 4)], &opts).unwrap();
        assert!(r[0].notes.iter().all(|n| (60..=72).contains(&n.0)));
        assert!(r[0].notes.len() >= 3);
    }

    #[test]
    fn invalid_options_and_durations() {
        let bad = VoicingOptions {
            low: 60,
            high: 65,
            ..Default::default()
        };
        assert_eq!(
            realize(&[tc("C", 4)], &bad),
            Err(HarmonyError::InvalidVoicingRange)
        );
        assert_eq!(
            realize(&[tc("C", 0)], &VoicingOptions::default()),
            Err(HarmonyError::InvalidDuration)
        );
    }

    #[test]
    fn whole_catalog_realizes_in_range_with_exact_timing() {
        let opts = VoicingOptions::default();
        for p in builtin_catalog() {
            for t in [0u8, 5, 11] {
                let key = Key::new(PitchClass::new(t).unwrap(), p.mode);
                let r = p.realize(key, &opts).unwrap();
                assert_eq!(r.len(), p.steps.len());
                let mut at = 0;
                for (rc, s) in r.iter().zip(&p.steps) {
                    assert_eq!(rc.start, Tick(at));
                    assert_eq!(rc.duration, Tick(s.beats as i64 * PPQ));
                    at += s.beats as i64 * PPQ;
                    assert!(rc
                        .notes
                        .iter()
                        .all(|n| (opts.low..=opts.high).contains(&n.0)));
                    let pcs = rc.chord.pitch_classes();
                    assert!(rc.notes.iter().all(|n| pcs.contains(&n.pitch_class())));
                    let b = rc.bass.unwrap().0;
                    assert!((36..=52).contains(&b));
                }
                assert_eq!(at, p.total_beats() as i64 * PPQ);
            }
        }
    }

    proptest! {
        #[test]
        fn prop_notes_in_range_and_ascending(root in 0u8..12, qi in 0usize..18,
                                             low in 40u8..70, span in 12u8..30, vl in any::<bool>()) {
            let chord = Chord::new(PitchClass::new(root).unwrap(), crate::domain::chord::ChordQuality::ALL[qi]);
            let opts = VoicingOptions { low, high: low + span, voice_leading: vl, bass_range: None, start: Tick(0) };
            let r = realize(&[TimedChord { chord: chord.clone(), beats: 1 }, TimedChord { chord, beats: 3 }], &opts).unwrap();
            for rc in &r {
                prop_assert!(!rc.notes.is_empty());
                prop_assert!(rc.notes.iter().all(|n| n.0 >= low && n.0 <= low + span));
                prop_assert!(rc.notes.windows(2).all(|w| w[0] < w[1]));
            }
            prop_assert_eq!(r[1].start, Tick(PPQ));
        }
    }
}
