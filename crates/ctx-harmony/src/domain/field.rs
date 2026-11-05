//! Campo harmônico **derivado** dos intervalos do modo (nunca armazenado por tom — ADR-0007).

use super::chord::{Chord, ChordQuality};
use super::scale::{Key, Mode};
use serde::{Deserialize, Serialize};

/// Qualidade diatônica do grau `step` (1–7) do modo, por empilhamento de terças da escala.
pub fn diatonic_quality(mode: Mode, step: u8, seventh: bool) -> ChordQuality {
    let iv = mode.intervals();
    let i = (step.clamp(1, 7) - 1) as usize;
    let rel = |k: usize| (iv[(i + k) % 7] + 12 - iv[i]) % 12;
    let mut stack = vec![0, rel(2), rel(4)];
    if seventh {
        stack.push(rel(6));
    }
    // Invariante coberta pelo teste `every_mode_degree_has_known_quality`: todo empilhamento de
    // terças das três famílias resulta em uma qualidade conhecida.
    ChordQuality::from_intervals(&stack).expect("empilhamento diatônico com qualidade conhecida")
}

/// Um grau do campo harmônico: tríade e tétrade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDegree {
    pub degree: u8,
    pub triad: Chord,
    pub seventh: Chord,
}

/// Campo harmônico completo (7 graus) da tonalidade.
pub fn harmonic_field(key: Key) -> Vec<FieldDegree> {
    key.scale()
        .iter()
        .enumerate()
        .map(|(i, root)| {
            let step = i as u8 + 1;
            FieldDegree {
                degree: step,
                triad: Chord::new(*root, diatonic_quality(key.mode, step, false)),
                seventh: Chord::new(*root, diatonic_quality(key.mode, step, true)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_kernel::music::PitchClass;

    fn names(key: Key, seventh: bool) -> Vec<String> {
        harmonic_field(key)
            .iter()
            .map(|d| {
                if seventh {
                    d.seventh.symbol(key.prefer_flats())
                } else {
                    d.triad.symbol(key.prefer_flats())
                }
            })
            .collect()
    }

    fn key(s: &str) -> Key {
        Key::parse(s).unwrap()
    }

    #[test]
    fn c_major_field() {
        assert_eq!(
            names(key("C"), false),
            ["C", "Dm", "Em", "F", "G", "Am", "Bdim"]
        );
        assert_eq!(
            names(key("C"), true),
            ["Cmaj7", "Dm7", "Em7", "Fmaj7", "G7", "Am7", "Bm7(b5)"]
        );
    }

    #[test]
    fn a_harmonic_minor_dominant_is_major() {
        let k = key("A harmonic_minor");
        let field = harmonic_field(k);
        assert_eq!(field[4].triad.symbol(false), "E");
        assert_eq!(field[4].seventh.symbol(false), "E7");
        assert_eq!(field[6].seventh.symbol(false), "G#dim7");
        assert_eq!(field[2].triad.symbol(false), "Caug");
        assert_eq!(field[0].seventh.symbol(false), "Am(maj7)");
    }

    #[test]
    fn natural_and_melodic_minor_fields() {
        assert_eq!(
            names(key("Am"), false),
            ["Am", "Bdim", "C", "Dm", "Em", "F", "G"]
        );
        assert_eq!(
            names(key("C melodic_minor"), true),
            [
                "Cm(maj7)",
                "Dm7",
                "Ebmaj7(#5)",
                "F7",
                "G7",
                "Am7(b5)",
                "Bm7(b5)"
            ]
        );
        assert_eq!(
            names(key("D dorian"), false),
            ["Dm", "Em", "F", "G", "Am", "Bdim", "C"]
        );
    }

    #[test]
    fn every_mode_degree_has_known_quality() {
        for m in Mode::ALL {
            for step in 1..=7 {
                let _ = diatonic_quality(m, step, false);
                let _ = diatonic_quality(m, step, true);
            }
        }
    }

    #[test]
    fn field_is_transposition_invariant() {
        for m in Mode::ALL {
            let c = harmonic_field(Key::new(PitchClass::new(0).unwrap(), m));
            for t in 1..12 {
                let k = Key::new(PitchClass::new(t).unwrap(), m);
                let other = harmonic_field(k);
                for (a, b) in c.iter().zip(&other) {
                    assert_eq!(a.triad.transpose(t as i32), b.triad);
                    assert_eq!(a.seventh.transpose(t as i32), b.seventh);
                }
            }
        }
    }
}
