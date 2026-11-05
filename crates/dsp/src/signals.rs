//! Geradores de sinais sintéticos determinísticos e medidas offline.
//!
//! Usados pelos testes deste crate e pela futura simulação offline da engine. **Alocam**
//! (retornam `Vec`) e portanto não devem ser chamados na thread de áudio.

use std::f64::consts::TAU;

/// Seno de amplitude de pico `amp`, frequência `freq_hz`, fase inicial `phase` (rad).
pub fn sine(freq_hz: f32, amp: f32, sample_rate: f32, len: usize, phase: f64) -> Vec<f32> {
    let w = TAU * freq_hz as f64 / sample_rate as f64;
    (0..len)
        .map(|n| (amp as f64 * (w * n as f64 + phase).sin()) as f32)
        .collect()
}

/// Impulso unitário de amplitude `amp` na amostra `at`.
pub fn impulse(len: usize, at: usize, amp: f32) -> Vec<f32> {
    let mut v = vec![0.0; len];
    if let Some(s) = v.get_mut(at) {
        *s = amp;
    }
    v
}

/// Ruído branco uniforme em `[-amp, amp]`, determinístico (xorshift32 com `seed`).
pub fn noise(len: usize, amp: f32, seed: u32) -> Vec<f32> {
    let mut s = seed.max(1);
    (0..len)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            amp * ((s as f64 / u32::MAX as f64) * 2.0 - 1.0) as f32
        })
        .collect()
}

/// Pico absoluto de um trecho.
pub fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

/// Valor RMS de um trecho (0 se vazio).
pub fn rms(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    let s: f64 = x.iter().map(|v| (*v as f64) * (*v as f64)).sum();
    (s / x.len() as f64).sqrt() as f32
}

/// Pico em dBFS de um trecho.
pub fn peak_db(x: &[f32]) -> f32 {
    crate::util::lin_to_db(peak(x))
}

/// RMS em dB de um trecho.
pub fn rms_db(x: &[f32]) -> f32 {
    crate::util::lin_to_db(rms(x))
}

/// Processa um par de buffers em blocos de `block` amostras (simula o callback de áudio).
pub fn process_in_blocks<P: crate::Processor + ?Sized>(
    p: &mut P,
    left: &mut [f32],
    right: &mut [f32],
    block: usize,
) {
    let block = block.max(1);
    let n = left.len().min(right.len());
    let mut i = 0;
    while i < n {
        let end = (i + block).min(n);
        p.process(&mut left[i..end], &mut right[i..end]);
        i = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_has_expected_peak_and_rms() {
        let x = sine(1_000.0, 0.5, 48_000.0, 48_000, 0.0);
        assert!((peak(&x) - 0.5).abs() < 1e-3);
        assert!((rms(&x) - 0.5 / 2f32.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn impulse_and_noise_are_deterministic() {
        let i = impulse(4, 2, 1.0);
        assert_eq!(i, vec![0.0, 0.0, 1.0, 0.0]);
        assert_eq!(noise(16, 1.0, 7), noise(16, 1.0, 7));
        assert!(peak(&noise(10_000, 0.3, 1)) <= 0.3);
        assert_eq!(rms(&[]), 0.0);
    }
}
