//! Fração racional exata (em unidades de semibreve) para somar durações sem erro de arredondamento.

use serde::{Deserialize, Serialize};
use shared_kernel::time::PPQ;
use std::cmp::Ordering;

/// Fração normalizada (denominador > 0, irredutível).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frac {
    num: i64,
    den: i64,
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Frac {
    pub const ZERO: Frac = Frac { num: 0, den: 1 };

    /// Cria `num/den`. Denominador zero vira zero (valor seguro).
    pub fn new(num: i64, den: i64) -> Self {
        Self::from_i128(num as i128, den as i128)
    }

    /// Reduz em 128 bits; resultados fora de 64 bits saturam (nunca há pânico por overflow).
    fn from_i128(num: i128, den: i128) -> Self {
        if den == 0 {
            return Self::ZERO;
        }
        let g = gcd(num, den).max(1);
        let s = if den < 0 { -1 } else { 1 };
        let (mut n, mut d) = (s * num / g, s * den / g);
        while d > i64::MAX as i128 || n.abs() > i64::MAX as i128 {
            n /= 2;
            d = (d / 2).max(1);
        }
        Self {
            num: n as i64,
            den: d as i64,
        }
    }

    pub const fn num(self) -> i64 {
        self.num
    }

    pub const fn den(self) -> i64 {
        self.den
    }

    pub fn is_zero(self) -> bool {
        self.num == 0
    }

    pub fn is_positive(self) -> bool {
        self.num > 0
    }

    /// Multiplica por `num/den`.
    pub fn mul(self, num: i64, den: i64) -> Self {
        Self::from_i128(
            self.num as i128 * num as i128,
            self.den as i128 * den as i128,
        )
    }

    /// Converte (semibreves) em ticks, arredondando ao mais próximo.
    pub fn to_ticks(self) -> i64 {
        let n = self.num as i128 * (PPQ as i128 * 4);
        let d = self.den as i128;
        let q = n.div_euclid(d);
        let r = n.rem_euclid(d);
        let t = if r * 2 >= d { q + 1 } else { q };
        t.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    /// Converte ticks em semibreves (exato).
    pub fn from_ticks(ticks: i64) -> Self {
        Self::new(ticks, PPQ * 4)
    }
}

impl std::ops::Add for Frac {
    type Output = Frac;
    fn add(self, o: Frac) -> Frac {
        Frac::from_i128(
            self.num as i128 * o.den as i128 + o.num as i128 * self.den as i128,
            self.den as i128 * o.den as i128,
        )
    }
}

impl std::ops::Sub for Frac {
    type Output = Frac;
    fn sub(self, o: Frac) -> Frac {
        Frac::from_i128(
            self.num as i128 * o.den as i128 - o.num as i128 * self.den as i128,
            self.den as i128 * o.den as i128,
        )
    }
}

impl std::ops::AddAssign for Frac {
    fn add_assign(&mut self, o: Frac) {
        *self = *self + o;
    }
}

impl PartialOrd for Frac {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frac {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.num as i128 * o.den as i128).cmp(&(o.num as i128 * self.den as i128))
    }
}

impl std::fmt::Display for Frac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn normalizes_and_compares() {
        assert_eq!(Frac::new(2, 4), Frac::new(1, 2));
        assert_eq!(Frac::new(1, -2), Frac::new(-1, 2));
        assert_eq!(Frac::new(1, 0), Frac::ZERO);
        assert!(Frac::new(1, 3) < Frac::new(1, 2));
        assert_eq!(Frac::new(1, 3) + Frac::new(1, 6), Frac::new(1, 2));
        assert_eq!(Frac::new(1, 4).to_ticks(), PPQ);
        assert_eq!(Frac::from_ticks(PPQ * 2), Frac::new(1, 2));
    }

    #[test]
    fn extreme_values_saturate_without_panic() {
        let big = Frac::new(i64::MAX, 3);
        let _ = big + big;
        let _ = big.mul(i64::MAX, 1);
        let _ = Frac::new(i64::MIN + 1, -1) - big;
        let _ = big.to_ticks();
    }

    proptest! {
        #[test]
        fn add_sub_roundtrip(a in -1000i64..1000, b in 1i64..256, c in -1000i64..1000, d in 1i64..256) {
            let x = Frac::new(a, b);
            let y = Frac::new(c, d);
            prop_assert_eq!(x + y - y, x);
        }

        #[test]
        fn ticks_roundtrip(t in 0i64..1_000_000) {
            prop_assert_eq!(Frac::from_ticks(t).to_ticks(), t);
        }
    }
}
