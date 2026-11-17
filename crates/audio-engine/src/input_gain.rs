//! Ganho de entrada (pré-amplificação digital do microfone/interface) — Pure Fabrication de tempo real.
//!
//! O ganho é aplicado às amostras do dispositivo **antes** de qualquer uso da entrada: gravação,
//! medidores de entrada, derivação do afinador e as duas monitorações (dispositivo e faixa). Como a
//! entrada chega como fatia imutável do callback, o núcleo escreve o resultado num buffer de rascunho
//! pré-alocado ([`INPUT_SCRATCH_SAMPLES`]); nada é alocado no tempo real.
//!
//! Mudanças de ganho seguem uma rampa linear por quadro (~10 ms) para não gerar estalos ("zipper").
//! Amostras não finitas (NaN/inf) viram silêncio.

use crate::MAX_BLOCK;

/// Faixa aceita (dB). 0 dB = sinal do dispositivo sem alteração.
pub const INPUT_GAIN_MIN_DB: f32 = 0.0;
pub const INPUT_GAIN_MAX_DB: f32 = 24.0;

/// Tamanho do rascunho (amostras intercaladas): `MAX_BLOCK` quadros de até 64 canais. Com mais canais,
/// o núcleo processa blocos menores (`INPUT_SCRATCH_SAMPLES / canais` quadros).
pub const INPUT_SCRATCH_SAMPLES: usize = MAX_BLOCK * 64;

/// dB → ganho linear, limitado à faixa aceita; valor não finito = 0 dB.
pub fn input_gain_linear(db: f32) -> f32 {
    let db = if db.is_finite() {
        db.clamp(INPUT_GAIN_MIN_DB, INPUT_GAIN_MAX_DB)
    } else {
        0.0
    };
    10f32.powf(db / 20.0)
}

/// Estado do ganho no núcleo (rampa por quadro até `target`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct InputGain {
    current: f32,
    target: f32,
    step: f32,
    ramp_frames: f32,
}

impl InputGain {
    pub(crate) fn new(sample_rate: u32) -> Self {
        Self {
            current: 1.0,
            target: 1.0,
            step: 0.0,
            ramp_frames: (sample_rate as f32 / 100.0).max(1.0),
        }
    }

    /// Novo alvo linear (limitado a 0..+24 dB; não finito = 0 dB). A rampa dura ~10 ms.
    pub(crate) fn set_target(&mut self, linear: f32) {
        let max = input_gain_linear(INPUT_GAIN_MAX_DB);
        let t = if linear.is_finite() {
            linear.clamp(1.0, max)
        } else {
            1.0
        };
        self.target = t;
        self.step = (t - self.current).abs() / self.ramp_frames;
    }

    /// Ganho aplicado no último quadro processado.
    #[cfg(test)]
    pub(crate) fn current(&self) -> f32 {
        self.current
    }

    /// Escreve `input` × ganho em `out` (mesmo tamanho, intercalado com `channels`). Sem alocação.
    pub(crate) fn apply(&mut self, input: &[f32], channels: usize, out: &mut [f32]) {
        if channels == 0 {
            return;
        }
        for (src, dst) in input
            .chunks_exact(channels)
            .zip(out.chunks_exact_mut(channels))
        {
            if self.current != self.target {
                self.current = if self.current < self.target {
                    (self.current + self.step).min(self.target)
                } else {
                    (self.current - self.step).max(self.target)
                };
            }
            let g = self.current;
            for (s, d) in src.iter().zip(dst.iter_mut()) {
                *d = if s.is_finite() { s * g } else { 0.0 };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_conversion_clamps_and_ignores_nan() {
        assert_eq!(input_gain_linear(0.0), 1.0);
        assert!((input_gain_linear(12.0) - 3.981).abs() < 1e-3);
        assert_eq!(input_gain_linear(-10.0), 1.0, "abaixo de 0 dB = 0 dB");
        assert!((input_gain_linear(40.0) - input_gain_linear(24.0)).abs() < 1e-6);
        assert_eq!(input_gain_linear(f32::NAN), 1.0);
        assert_eq!(input_gain_linear(f32::INFINITY), 1.0);
    }

    #[test]
    fn ramps_per_frame_without_jumps() {
        let mut g = InputGain::new(48_000); // rampa de 480 quadros
        g.set_target(input_gain_linear(12.0));
        let input = vec![0.1f32; 1_000];
        let mut out = vec![0.0f32; 1_000];
        g.apply(&input, 1, &mut out);
        let max_jump = out
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(max_jump < 0.002, "rampa suave, salto máximo {max_jump}");
        assert!(
            out[0] > 0.1 && out[0] < 0.101,
            "começa perto de 0 dB: {}",
            out[0]
        );
        assert!(
            (out[999] - 0.398).abs() < 1e-3,
            "chega a +12 dB: {}",
            out[999]
        );
        assert!((g.current() - input_gain_linear(12.0)).abs() < 1e-6);
    }

    #[test]
    fn same_gain_for_all_channels_of_a_frame_and_nan_becomes_silence() {
        let mut g = InputGain::new(48_000);
        g.set_target(f32::NAN);
        let mut out = [9.0f32; 4];
        g.apply(&[0.5, f32::NAN, -0.25, f32::INFINITY], 2, &mut out);
        assert_eq!(out, [0.5, 0.0, -0.25, 0.0]);
        g.set_target(100.0);
        assert!(
            (g.target - input_gain_linear(24.0)).abs() < 1e-6,
            "limitado a +24 dB"
        );
    }
}
