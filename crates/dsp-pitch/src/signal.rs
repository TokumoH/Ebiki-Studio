//! Geradores de sinais sintéticos determinísticos (testes, simulação e calibração).

use std::f32::consts::TAU;

/// Seno de frequência `hz`.
pub fn sine(hz: f32, sample_rate: f32, len: usize, amplitude: f32) -> Vec<f32> {
    let w = TAU as f64 * hz as f64 / sample_rate as f64;
    (0..len)
        .map(|i| amplitude * (w * i as f64).sin() as f32)
        .collect()
}

/// Dente de serra ingênuo (rico em harmônicos) de frequência `hz`.
pub fn sawtooth(hz: f32, sample_rate: f32, len: usize, amplitude: f32) -> Vec<f32> {
    let step = hz as f64 / sample_rate as f64;
    (0..len)
        .map(|i| {
            let phase = (step * i as f64).fract();
            amplitude * (2.0 * phase - 1.0) as f32
        })
        .collect()
}

/// Ruído branco uniforme determinístico (xorshift32).
pub fn noise(len: usize, amplitude: f32, seed: u32) -> Vec<f32> {
    let mut s = seed.max(1);
    (0..len)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            amplitude * ((s as f32 / u32::MAX as f32) * 2.0 - 1.0)
        })
        .collect()
}

/// Sequência de senos com fase contínua: `(hz, duração em amostras)`. `hz = 0` gera silêncio.
pub fn melody(notes: &[(f32, usize)], sample_rate: f32, amplitude: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(notes.iter().map(|n| n.1).sum());
    let mut phase = 0.0f64;
    for &(hz, len) in notes {
        let w = TAU as f64 * hz as f64 / sample_rate as f64;
        for _ in 0..len {
            out.push(if hz > 0.0 {
                amplitude * phase.sin() as f32
            } else {
                0.0
            });
            phase = (phase + w) % std::f64::consts::TAU;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generators_have_expected_shape() {
        let s = sine(1000.0, 48_000.0, 48, 1.0);
        assert!(s[0].abs() < 1e-6);
        assert!((s[12] - 1.0).abs() < 1e-5, "quarto de período = pico");
        let saw = sawtooth(100.0, 1000.0, 20, 1.0);
        assert!((saw[0] + 1.0).abs() < 1e-6 && saw[9] > 0.7);
        let n1 = noise(100, 1.0, 7);
        assert_eq!(n1, noise(100, 1.0, 7));
        assert!(n1.iter().all(|v| v.abs() <= 1.0));
        let m = melody(&[(440.0, 10), (0.0, 5)], 48_000.0, 1.0);
        assert_eq!(m.len(), 15);
        assert!(m[10..].iter().all(|v| *v == 0.0));
    }
}
