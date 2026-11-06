//! Curvas de fade compartilhadas entre o domínio (Project) e a engine de áudio.
//!
//! Função pura e sem alocação: pode ser chamada por amostra na thread de áudio.

use serde::{Deserialize, Serialize};

/// Forma da curva de um fade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FadeShape {
    /// Rampa linear de amplitude (soma constante de amplitude no crossfade).
    #[default]
    Linear,
    /// Seno/cosseno (soma constante de potência no crossfade: sem "buraco" de volume).
    EqualPower,
}

impl FadeShape {
    pub const ALL: [FadeShape; 2] = [FadeShape::Linear, FadeShape::EqualPower];

    /// Ganho de amplitude da curva para `t` ∈ [0, 1] (0 = silêncio, 1 = ganho unitário).
    /// Valores fora do intervalo são limitados.
    #[inline]
    pub fn gain(self, t: f32) -> f32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        match self {
            FadeShape::Linear => t,
            FadeShape::EqualPower => (t * std::f32::consts::FRAC_PI_2).sin(),
        }
    }

    /// Código estável (persistência/IPC).
    pub fn code(self) -> &'static str {
        match self {
            FadeShape::Linear => "linear",
            FadeShape::EqualPower => "equalPower",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_start_silent_and_end_at_unity() {
        for s in FadeShape::ALL {
            assert_eq!(s.gain(0.0), 0.0);
            assert!((s.gain(1.0) - 1.0).abs() < 1e-6);
            assert_eq!(s.gain(-1.0), 0.0, "limitado");
            assert!((s.gain(2.0) - 1.0).abs() < 1e-6, "limitado");
            assert_eq!(s.gain(f32::NAN), 0.0);
        }
    }

    #[test]
    fn crossfade_sums_are_constant() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let lin = FadeShape::Linear.gain(t) + FadeShape::Linear.gain(1.0 - t);
            assert!((lin - 1.0).abs() < 1e-6, "linear soma amplitude 1");
            let a = FadeShape::EqualPower.gain(t);
            let b = FadeShape::EqualPower.gain(1.0 - t);
            assert!(
                (a * a + b * b - 1.0).abs() < 1e-5,
                "equal power soma potência 1"
            );
        }
    }

    #[test]
    fn codes_roundtrip() {
        for s in FadeShape::ALL {
            assert_eq!(FadeShape::from_code(s.code()), Some(s));
        }
        assert_eq!(FadeShape::from_code("x"), None);
    }
}
