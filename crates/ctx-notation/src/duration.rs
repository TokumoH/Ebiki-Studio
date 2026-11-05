//! Figuras rítmicas (semibreve … semifusa de 128), pontos de aumento e quiálteras.

use crate::error::NotationError;
use crate::frac::Frac;
use serde::{Deserialize, Serialize};

/// Figura (valor base) de uma nota ou pausa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoteValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
    OneTwentyEighth,
}

impl NoteValue {
    /// Do maior para o menor.
    pub const ALL: [NoteValue; 8] = [
        NoteValue::Whole,
        NoteValue::Half,
        NoteValue::Quarter,
        NoteValue::Eighth,
        NoteValue::Sixteenth,
        NoteValue::ThirtySecond,
        NoteValue::SixtyFourth,
        NoteValue::OneTwentyEighth,
    ];

    /// Denominador (1 = semibreve, 4 = semínima, 128 = quartifusa).
    pub const fn denominator(self) -> i64 {
        match self {
            Self::Whole => 1,
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Eighth => 8,
            Self::Sixteenth => 16,
            Self::ThirtySecond => 32,
            Self::SixtyFourth => 64,
            Self::OneTwentyEighth => 128,
        }
    }

    pub fn from_denominator(d: i64) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.denominator() == d)
    }
}

/// Quiáltera: `actual` notas no tempo de `normal` (tercina = 3:2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tuplet {
    pub actual: u8,
    pub normal: u8,
}

impl Tuplet {
    pub const TRIPLET: Tuplet = Tuplet {
        actual: 3,
        normal: 2,
    };

    pub fn new(actual: u8, normal: u8) -> Result<Self, NotationError> {
        if !(2..=32).contains(&actual) || !(1..=32).contains(&normal) || actual == normal {
            return Err(NotationError::InvalidDuration("quiáltera fora de 2–32"));
        }
        Ok(Self { actual, normal })
    }
}

pub const MAX_DOTS: u8 = 3;

/// Duração escrita: figura + pontos + quiáltera opcional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Duration {
    pub value: NoteValue,
    #[serde(default)]
    pub dots: u8,
    #[serde(default)]
    pub tuplet: Option<Tuplet>,
}

impl Duration {
    pub const fn new(value: NoteValue) -> Self {
        Self {
            value,
            dots: 0,
            tuplet: None,
        }
    }

    pub const fn quarter() -> Self {
        Self::new(NoteValue::Quarter)
    }

    pub fn dotted(mut self, dots: u8) -> Self {
        self.dots = dots;
        self
    }

    pub fn with_tuplet(mut self, tuplet: Tuplet) -> Self {
        self.tuplet = Some(tuplet);
        self
    }

    pub fn validate(&self) -> Result<(), NotationError> {
        if self.dots > MAX_DOTS {
            return Err(NotationError::InvalidDuration("no máximo 3 pontos"));
        }
        if let Some(t) = self.tuplet {
            Tuplet::new(t.actual, t.normal)?;
        }
        Ok(())
    }

    /// Duração exata em semibreves.
    pub fn fraction(&self) -> Frac {
        let d = self.dots.min(MAX_DOTS) as u32;
        let base = Frac::new(1, self.value.denominator());
        let dotted = base.mul((1i64 << (d + 1)) - 1, 1i64 << d);
        match self.tuplet {
            Some(t) if t.actual > 0 => dotted.mul(t.normal as i64, t.actual as i64),
            _ => dotted,
        }
    }

    /// Duração em ticks (arredondada; use [`Duration::fraction`] para somas exatas).
    pub fn ticks(&self) -> i64 {
        self.fraction().to_ticks()
    }

    /// Duração exata (sem quiáltera, até 2 pontos) correspondente a `f`, se existir.
    pub fn from_fraction_exact(f: Frac) -> Option<Self> {
        for v in NoteValue::ALL {
            for dots in 0..=2 {
                let d = Duration::new(v).dotted(dots);
                if d.fraction() == f {
                    return Some(d);
                }
            }
        }
        None
    }

    /// Decompõe uma duração qualquer numa sequência de figuras (ligáveis) cuja soma é `total`.
    /// Usa figuras simples/pontuadas; se a fração exigir, usa tercinas. Retorna também o resto
    /// que não pôde ser representado (menor que uma quartifusa de tercina).
    pub fn decompose(total: Frac) -> (Vec<Duration>, Frac) {
        let mut out = Vec::new();
        if !total.is_positive() {
            return (out, Frac::ZERO);
        }
        let binary = is_binary(total);
        let triplet = !binary && is_binary(total.mul(3, 2));
        let mut candidates: Vec<Duration> = Vec::new();
        for v in NoteValue::ALL {
            for dots in [1u8, 0] {
                let mut d = Duration::new(v).dotted(dots);
                if triplet {
                    d = d.with_tuplet(Tuplet::TRIPLET);
                }
                candidates.push(d);
            }
        }
        candidates.sort_by_key(|d| std::cmp::Reverse(d.fraction()));
        let mut rest = total;
        while rest.is_positive() && out.len() < 64 {
            match candidates.iter().find(|d| d.fraction() <= rest) {
                Some(d) => {
                    rest = rest - d.fraction();
                    out.push(*d);
                }
                None => break,
            }
        }
        (out, rest)
    }
}

/// Denominador potência de 2 até 128·4 (representável com figuras e pontos)?
fn is_binary(f: Frac) -> bool {
    let d = f.den();
    d > 0 && d & (d - 1) == 0 && d <= 512
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use shared_kernel::time::PPQ;

    #[test]
    fn fractions_and_ticks() {
        assert_eq!(Duration::quarter().ticks(), PPQ);
        assert_eq!(Duration::new(NoteValue::Whole).ticks(), 4 * PPQ);
        assert_eq!(Duration::new(NoteValue::OneTwentyEighth).ticks(), 30);
        assert_eq!(Duration::quarter().dotted(1).ticks(), PPQ * 3 / 2);
        assert_eq!(Duration::quarter().dotted(2).ticks(), PPQ * 7 / 4);
        assert_eq!(
            Duration::new(NoteValue::Eighth)
                .with_tuplet(Tuplet::TRIPLET)
                .ticks(),
            PPQ / 3
        );
        assert_eq!(
            Duration::new(NoteValue::Eighth)
                .with_tuplet(Tuplet::TRIPLET)
                .fraction(),
            Frac::new(1, 12)
        );
    }

    #[test]
    fn validation() {
        assert!(Duration::quarter().dotted(4).validate().is_err());
        assert!(Tuplet::new(1, 1).is_err());
        assert!(Tuplet::new(5, 4).is_ok());
    }

    #[test]
    fn decompose_binary_and_triplets() {
        let (d, r) = Duration::decompose(Frac::new(5, 16));
        assert!(r.is_zero());
        assert_eq!(
            d.iter()
                .map(|x| x.fraction())
                .fold(Frac::ZERO, |a, b| a + b),
            Frac::new(5, 16)
        );
        let (d, r) = Duration::decompose(Frac::new(1, 12));
        assert!(r.is_zero());
        assert_eq!(
            d,
            vec![Duration::new(NoteValue::Eighth).with_tuplet(Tuplet::TRIPLET)]
        );
        assert_eq!(
            Duration::from_fraction_exact(Frac::new(3, 8)),
            Some(Duration::quarter().dotted(1))
        );
    }

    proptest! {
        #[test]
        fn decompose_sums_to_total(n in 1i64..512) {
            let total = Frac::new(n, 128);
            let (parts, rest) = Duration::decompose(total);
            prop_assert!(rest.is_zero());
            let sum = parts.iter().fold(Frac::ZERO, |a, d| a + d.fraction());
            prop_assert_eq!(sum, total);
        }
    }
}
