//! Política de divergência de BPM/tom ao inserir uma base no projeto (RF-4.5).
//!
//! O domínio só **calcula** a divergência; a decisão (ajustar ou manter) é sempre do usuário — a
//! UI pergunta sempre que [`MismatchReport::needs_decision`] for verdadeiro.

use super::base::BaseKey;
use super::error::LibraryError;
use serde::{Deserialize, Serialize};
use shared_kernel::tempo::{MAX_BPM, MIN_BPM};

/// Diferenças entre a base e o projeto.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MismatchReport {
    /// Fator de time-stretch para casar o andamento: `bpm_projeto / bpm_base` (> 1 = acelerar).
    pub bpm_ratio: f64,
    /// Menor transposição (−6…+5 semitons) que leva a base ao tom do projeto; `None` se algum dos
    /// tons for desconhecido. Se os modos diferem, mira a relativa do projeto no modo da base.
    pub semitone_shift: Option<i8>,
    /// Base maior em projeto menor (ou vice-versa).
    pub mode_differs: bool,
    /// Há algo a decidir (a UI deve perguntar).
    pub needs_decision: bool,
}

/// Tolerância de BPM considerada "igual".
pub const BPM_EPSILON: f64 = 1e-3;

fn check_bpm(bpm: f64) -> Result<(), LibraryError> {
    if bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm) {
        Ok(())
    } else {
        Err(LibraryError::InvalidBpm)
    }
}

/// Calcula a divergência entre base e projeto.
pub fn mismatch(
    base_bpm: f64,
    base_key: Option<BaseKey>,
    project_bpm: f64,
    project_key: Option<BaseKey>,
) -> Result<MismatchReport, LibraryError> {
    check_bpm(base_bpm)?;
    check_bpm(project_bpm)?;
    let bpm_ratio = project_bpm / base_bpm;
    let (semitone_shift, mode_differs) = match (base_key, project_key) {
        (Some(b), Some(p)) => {
            let target = if b.mode == p.mode {
                p.tonic
            } else {
                p.relative_tonic()
            };
            let up = b.tonic.interval_to(target) as i8; // 0..=11
            let shift = if up > 5 { up - 12 } else { up };
            (Some(shift), b.mode != p.mode)
        }
        _ => (None, false),
    };
    let needs_decision = (project_bpm - base_bpm).abs() > BPM_EPSILON
        || semitone_shift.is_some_and(|s| s != 0)
        || mode_differs;
    Ok(MismatchReport {
        bpm_ratio,
        semitone_shift,
        mode_differs,
        needs_decision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::base::KeyMode;
    use proptest::prelude::*;
    use shared_kernel::music::PitchClass;

    fn key(t: &str, m: KeyMode) -> Option<BaseKey> {
        Some(BaseKey::new(PitchClass::parse(t).unwrap(), m))
    }

    #[test]
    fn identical_base_needs_no_decision() {
        let r = mismatch(
            120.0,
            key("C", KeyMode::Major),
            120.0,
            key("C", KeyMode::Major),
        )
        .unwrap();
        assert_eq!(r.bpm_ratio, 1.0);
        assert_eq!(r.semitone_shift, Some(0));
        assert!(!r.needs_decision);
    }

    #[test]
    fn bpm_and_key_differences_ask_the_user() {
        let r = mismatch(
            100.0,
            key("C", KeyMode::Major),
            120.0,
            key("D", KeyMode::Major),
        )
        .unwrap();
        assert!((r.bpm_ratio - 1.2).abs() < 1e-12);
        assert_eq!(r.semitone_shift, Some(2));
        assert!(r.needs_decision);
        let down = mismatch(
            90.0,
            key("C", KeyMode::Major),
            90.0,
            key("A", KeyMode::Major),
        )
        .unwrap();
        assert_eq!(down.semitone_shift, Some(-3), "menor deslocamento");
        let tritone = mismatch(
            90.0,
            key("C", KeyMode::Major),
            90.0,
            key("F#", KeyMode::Major),
        )
        .unwrap();
        assert_eq!(tritone.semitone_shift, Some(-6));
    }

    #[test]
    fn mode_difference_targets_relative() {
        let r = mismatch(
            90.0,
            key("A", KeyMode::Minor),
            90.0,
            key("C", KeyMode::Major),
        )
        .unwrap();
        assert_eq!(r.semitone_shift, Some(0));
        assert!(r.mode_differs);
        assert!(r.needs_decision, "modo diferente sempre pergunta");
        let r = mismatch(
            90.0,
            key("C", KeyMode::Major),
            90.0,
            key("E", KeyMode::Minor),
        )
        .unwrap();
        assert_eq!(r.semitone_shift, Some(-5), "G maior (relativa de Mi menor)");
    }

    #[test]
    fn unknown_keys_and_invalid_bpm() {
        let r = mismatch(120.0, None, 120.0, key("C", KeyMode::Major)).unwrap();
        assert_eq!(r.semitone_shift, None);
        assert!(!r.needs_decision);
        assert_eq!(
            mismatch(f64::NAN, None, 120.0, None),
            Err(LibraryError::InvalidBpm)
        );
        assert_eq!(
            mismatch(120.0, None, 0.0, None),
            Err(LibraryError::InvalidBpm)
        );
        assert_eq!(KeyMode::Minor.name_pt(), "menor");
    }

    proptest! {
        #[test]
        fn prop_shift_lands_on_target(bt in 0u8..12, pt in 0u8..12, bm in any::<bool>(), pm in any::<bool>(),
                                      b_bpm in 20.0f64..999.0, p_bpm in 20.0f64..999.0) {
            let mode = |m: bool| if m { KeyMode::Major } else { KeyMode::Minor };
            let b = BaseKey::new(PitchClass::new(bt).unwrap(), mode(bm));
            let p = BaseKey::new(PitchClass::new(pt).unwrap(), mode(pm));
            let r = mismatch(b_bpm, Some(b), p_bpm, Some(p)).unwrap();
            let s = r.semitone_shift.unwrap();
            prop_assert!((-6..=5).contains(&s));
            let moved = b.tonic.transpose(s as i32);
            let target = if b.mode == p.mode { p.tonic } else { p.relative_tonic() };
            prop_assert_eq!(moved, target);
            prop_assert!((r.bpm_ratio * b_bpm - p_bpm).abs() < 1e-9);
        }
    }
}
