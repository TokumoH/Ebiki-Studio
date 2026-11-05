//! Quantização para inteiros com dither TPDF (densidade triangular, ±1 LSB).

/// Gerador xorshift64* determinístico (sem dependência externa; reprodutível em testes).
#[derive(Debug, Clone)]
pub(crate) struct Tpdf {
    state: u64,
}

impl Tpdf {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            state: seed | 1, // estado nunca zero
        }
    }

    fn next_unit(&mut self) -> f64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let r = x.wrapping_mul(0x2545_f491_4f6c_dd1d);
        // 53 bits → [0, 1)
        (r >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Ruído triangular em LSB, no intervalo (−1, 1).
    pub(crate) fn sample(&mut self) -> f64 {
        self.next_unit() - self.next_unit()
    }
}

/// Converte `x` (escala ±1,0) para inteiro de `bits` bits, com dither opcional.
/// Entrada não finita vira 0; saída saturada em `[-2^(bits-1), 2^(bits-1) - 1]`.
pub(crate) fn quantize(x: f32, bits: u32, dither: Option<&mut Tpdf>) -> i32 {
    let scale = f64::from(1u32 << (bits - 1));
    let x = if x.is_finite() { f64::from(x) } else { 0.0 };
    let noise = dither.map_or(0.0, Tpdf::sample);
    let v = (x * scale + noise).round();
    v.clamp(-scale, scale - 1.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_without_dither_is_exact_rounding() {
        assert_eq!(quantize(0.0, 16, None), 0);
        assert_eq!(quantize(1.0, 16, None), 32767);
        assert_eq!(quantize(-1.0, 16, None), -32768);
        assert_eq!(quantize(2.0, 24, None), 8_388_607);
        assert_eq!(quantize(f32::NAN, 16, None), 0);
        assert_eq!(quantize(0.5, 16, None), 16384);
    }

    #[test]
    fn tpdf_is_bounded_and_zero_mean() {
        let mut d = Tpdf::new(42);
        let n = 100_000;
        let mut sum = 0.0;
        for _ in 0..n {
            let s = d.sample();
            assert!(s > -1.0 && s < 1.0);
            sum += s;
        }
        assert!((sum / n as f64).abs() < 0.01);
    }

    #[test]
    fn dithered_error_is_within_1_5_lsb() {
        let mut d = Tpdf::new(7);
        for i in 0..10_000 {
            let x = (i as f32 * 0.000_123).sin() * 0.9;
            let q = quantize(x, 16, Some(&mut d));
            let err = (f64::from(q) - f64::from(x) * 32768.0).abs();
            assert!(err <= 1.5, "erro {err}");
        }
    }
}
