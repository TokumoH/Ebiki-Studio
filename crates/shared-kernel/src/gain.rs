//! Ganho em decibéis e panorâmica.

use serde::{Deserialize, Serialize};

/// Valor em decibéis. Valores abaixo de [`Db::FLOOR`] são tratados como silêncio.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Db(pub f32);

impl Db {
    /// Abaixo deste valor o ganho linear é 0 (silêncio).
    pub const FLOOR: f32 = -144.0;
    /// Ganho máximo permitido em faders.
    pub const FADER_MAX: f32 = 12.0;
    pub const UNITY: Db = Db(0.0);

    /// Converte para ganho linear de amplitude.
    pub fn to_linear(self) -> f32 {
        if !self.0.is_finite() || self.0 <= Self::FLOOR {
            0.0
        } else {
            10f32.powf(self.0 / 20.0)
        }
    }

    /// Converte ganho linear em dB (0 ou negativo vira [`Db::FLOOR`]).
    pub fn from_linear(linear: f32) -> Db {
        if !linear.is_finite() || linear <= 0.0 {
            Db(Self::FLOOR)
        } else {
            Db((20.0 * linear.log10()).max(Self::FLOOR))
        }
    }

    /// Limita ao intervalo de um fader: [FLOOR, FADER_MAX]. NaN vira FLOOR.
    pub fn clamp_fader(self) -> Db {
        if self.0.is_nan() {
            Db(Self::FLOOR)
        } else {
            Db(self.0.clamp(Self::FLOOR, Self::FADER_MAX))
        }
    }
}

/// Panorâmica de −1,0 (esquerda) a +1,0 (direita).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Pan(pub f32);

impl Pan {
    pub const CENTER: Pan = Pan(0.0);

    pub fn clamped(self) -> Pan {
        if self.0.is_nan() {
            Pan::CENTER
        } else {
            Pan(self.0.clamp(-1.0, 1.0))
        }
    }

    /// Ganhos (esquerdo, direito) pela lei de potência constante (−3 dB no centro).
    pub fn gains(self) -> (f32, f32) {
        let p = self.clamped().0;
        let angle = (p + 1.0) * std::f32::consts::FRAC_PI_4;
        (angle.cos(), angle.sin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn minus_six_db_is_half_amplitude() {
        assert!((Db(-6.0).to_linear() - 0.501_187).abs() < 1e-5);
        assert_eq!(Db::UNITY.to_linear(), 1.0);
        assert_eq!(Db(Db::FLOOR).to_linear(), 0.0);
        assert_eq!(Db(f32::NEG_INFINITY).to_linear(), 0.0);
        assert_eq!(Db(f32::NAN).to_linear(), 0.0);
    }

    #[test]
    fn from_linear_handles_silence() {
        assert_eq!(Db::from_linear(0.0).0, Db::FLOOR);
        assert_eq!(Db::from_linear(-1.0).0, Db::FLOOR);
        assert!((Db::from_linear(1.0).0).abs() < 1e-6);
    }

    #[test]
    fn clamp_fader_limits_range() {
        assert_eq!(Db(40.0).clamp_fader().0, Db::FADER_MAX);
        assert_eq!(Db(-500.0).clamp_fader().0, Db::FLOOR);
        assert_eq!(Db(f32::NAN).clamp_fader().0, Db::FLOOR);
    }

    #[test]
    fn pan_law_is_constant_power() {
        let (l, r) = Pan::CENTER.gains();
        assert!((l - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((r - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        let (l, r) = Pan(-1.0).gains();
        assert!((l - 1.0).abs() < 1e-6 && r.abs() < 1e-6);
        let (l, r) = Pan(1.0).gains();
        assert!(l.abs() < 1e-6 && (r - 1.0).abs() < 1e-6);
    }

    proptest! {
        #[test]
        fn db_linear_roundtrip(db in -120.0f32..12.0) {
            let back = Db::from_linear(Db(db).to_linear()).0;
            prop_assert!((back - db).abs() < 1e-3);
        }

        #[test]
        fn pan_power_sums_to_one(p in -1.0f32..=1.0) {
            let (l, r) = Pan(p).gains();
            prop_assert!((l * l + r * r - 1.0).abs() < 1e-5);
        }
    }
}
