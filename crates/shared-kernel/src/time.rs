//! Unidades de tempo e formato de áudio.

use serde::{Deserialize, Serialize};

/// Resolução musical: pulsos por semínima.
pub const PPQ: i64 = 960;

/// Posição musical em pulsos (1 semínima = [`PPQ`] ticks). Nunca negativa em posições válidas.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Tick(pub i64);

impl Tick {
    pub const ZERO: Tick = Tick(0);

    /// Duração de `n` semínimas.
    pub const fn quarters(n: i64) -> Tick {
        Tick(n * PPQ)
    }
}

impl std::ops::Add for Tick {
    type Output = Tick;
    fn add(self, rhs: Tick) -> Tick {
        Tick(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Tick {
    type Output = Tick;
    fn sub(self, rhs: Tick) -> Tick {
        Tick(self.0 - rhs.0)
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FormatError {
    #[error("taxa de amostragem não suportada: {0} Hz")]
    UnsupportedSampleRate(u32),
    #[error("resolução não suportada: {0} bits")]
    UnsupportedBitDepth(u16),
}

/// Taxa de amostragem suportada pelo projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct SampleRate(u32);

impl SampleRate {
    pub const SUPPORTED: [u32; 4] = [44_100, 48_000, 88_200, 96_000];
    pub const HZ_48000: SampleRate = SampleRate(48_000);

    pub fn new(hz: u32) -> Result<Self, FormatError> {
        if Self::SUPPORTED.contains(&hz) {
            Ok(Self(hz))
        } else {
            Err(FormatError::UnsupportedSampleRate(hz))
        }
    }

    pub const fn hz(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for SampleRate {
    type Error = FormatError;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        SampleRate::new(v)
    }
}

impl From<SampleRate> for u32 {
    fn from(v: SampleRate) -> u32 {
        v.0
    }
}

/// Resolução de gravação/exportação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct BitDepth(u16);

impl BitDepth {
    pub const SUPPORTED: [u16; 3] = [16, 24, 32];
    pub const BITS_24: BitDepth = BitDepth(24);

    pub fn new(bits: u16) -> Result<Self, FormatError> {
        if Self::SUPPORTED.contains(&bits) {
            Ok(Self(bits))
        } else {
            Err(FormatError::UnsupportedBitDepth(bits))
        }
    }

    pub const fn bits(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for BitDepth {
    type Error = FormatError;
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        BitDepth::new(v)
    }
}

impl From<BitDepth> for u16 {
    fn from(v: BitDepth) -> u16 {
        v.0
    }
}

/// Formato de áudio do projeto (definido na criação).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    pub sample_rate: SampleRate,
    pub bit_depth: BitDepth,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: SampleRate::HZ_48000,
            bit_depth: BitDepth::BITS_24,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_format_is_48k_24bit() {
        let f = AudioFormat::default();
        assert_eq!(f.sample_rate.hz(), 48_000);
        assert_eq!(f.bit_depth.bits(), 24);
    }

    #[test]
    fn rejects_unsupported_rates_and_depths() {
        assert!(SampleRate::new(22_050).is_err());
        assert!(BitDepth::new(8).is_err());
        assert!(serde_json::from_str::<SampleRate>("12345").is_err());
        assert_eq!(
            serde_json::from_str::<SampleRate>("96000").unwrap().hz(),
            96_000
        );
    }

    #[test]
    fn tick_arithmetic() {
        assert_eq!(Tick::quarters(4), Tick(3840));
        assert_eq!(Tick(10) + Tick(5) - Tick(3), Tick(12));
    }
}
