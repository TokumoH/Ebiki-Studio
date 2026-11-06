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
        const M: i128 = i64::MAX as i128;
        // denominador grande: reduz os dois (valor quase inalterado, perda relativa ~2⁻⁶³)
        while d > M {
            n /= 2;
            d /= 2;
        }
        if n.abs() > M {
            // |valor| > 1: melhor aproximação com denominador D que ainda cabe em 64 bits
            let sign = n.signum();
            let (q, r) = (n.abs() / d, n.abs() % d);
            if q >= M {
                n = sign * M; // além do i64: satura
                d = 1;
            } else {
                let den = (M / (q + 1)).clamp(1, d);
                let frac = (r * den + d / 2) / d; // r·den < 2¹²⁶: sem overflow
                n = sign * (q * den + frac);
                d = den;
                let g = gcd(n, d).max(1);
                (n, d) = (n / g, d / g);
            }
        }
        Self {
            num: n as i64,
            den: d.max(1) as i64,
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
        let as_f = |f: Frac| f.num() as f64 / f.den() as f64;
        let close = |got: f64, want: f64| ((got - want) / want).abs() < 1e-6;
        let big = Frac::new(i64::MAX, 3);
        // além do i64: a fração é aproximada, mas mantém sinal, denominador válido e valor
        let s = big + big;
        assert!(s.den() > 0 && s.is_positive());
        assert!(close(as_f(s), 2.0 * i64::MAX as f64 / 3.0), "{s:?}");
        let m = big.mul(i64::MAX, 1);
        assert!(m.den() > 0 && m.is_positive(), "{m:?}");
        let d = Frac::new(i64::MIN + 1, -1) - big;
        assert!(close(as_f(d), 2.0 * i64::MAX as f64 / 3.0), "{d:?}");
        // ticks além do i64 saturam no limite
        assert_eq!(big.to_ticks(), i64::MAX);
        assert_eq!(Frac::new(-i64::MAX, 3).to_ticks(), i64::MIN);
    }

    proptest! {
        /// Somas que estouram 64 bits continuam com erro relativo desprezível.
        #[test]
        fn huge_sums_keep_relative_precision(a in (i64::MAX / 4)..i64::MAX, b in 1i64..1000, c in (i64::MAX / 4)..i64::MAX, d in 1i64..1000) {
            let exact = a as f64 / b as f64 + c as f64 / d as f64;
            let got = Frac::new(a, b) + Frac::new(c, d);
            let gotf = got.num() as f64 / got.den() as f64;
            if exact < i64::MAX as f64 {
                prop_assert!(((gotf - exact) / exact).abs() < 1e-9, "{got:?} vs {exact}");
            } else {
                prop_assert_eq!(got, Frac::new(i64::MAX, 1));
            }
        }

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
