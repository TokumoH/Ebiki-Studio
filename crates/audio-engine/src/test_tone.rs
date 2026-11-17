//! Tom de teste da saída ("Testar som") — Pure Fabrication de tempo real.
//!
//! Senoide curta com rampas de entrada e saída, somada à saída do **dispositivo** depois do master
//! (como o "Ouvir entrada"): não passa pela mixagem, não é gravada (a gravação lê a entrada) e não é
//! exportada (a exportação usa outro núcleo, offline). Sem alocação: o estado é só um oscilador.

use std::f32::consts::TAU;

/// Frequência do tom (Hz).
pub const TEST_TONE_FREQ_HZ: f32 = 440.0;
/// Duração aceita (segundos).
pub const TEST_TONE_MIN_SECONDS: f32 = 0.05;
pub const TEST_TONE_MAX_SECONDS: f32 = 5.0;
/// Nível aceito (dBFS de pico).
pub const TEST_TONE_MIN_DBFS: f32 = -60.0;
pub const TEST_TONE_MAX_DBFS: f32 = 0.0;

/// Oscilador do tom no núcleo. `remaining == 0` = em silêncio.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TestTone {
    phase: f32,
    inc: f32,
    amplitude: f32,
    total: u64,
    remaining: u64,
    fade: u64,
}

impl TestTone {
    /// Começa (ou recomeça) um tom de `frames` quadros com pico `amplitude` (linear).
    pub(crate) fn start(&mut self, frames: u64, amplitude: f32, sample_rate: u32) {
        let amplitude = if amplitude.is_finite() {
            amplitude.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let fade = (sample_rate as u64 / 100).max(1); // ~10 ms
        *self = Self {
            phase: 0.0,
            inc: TAU * TEST_TONE_FREQ_HZ / sample_rate.max(1) as f32,
            amplitude,
            total: frames,
            remaining: frames,
            fade: fade.min((frames / 2).max(1)),
        };
    }

    pub(crate) fn is_active(&self) -> bool {
        self.remaining > 0
    }

    fn next(&mut self) -> f32 {
        if self.remaining == 0 {
            return 0.0;
        }
        let idx = self.total - self.remaining;
        let fade_in = (idx + 1) as f32 / self.fade as f32;
        let fade_out = self.remaining as f32 / self.fade as f32;
        let env = fade_in.min(fade_out).min(1.0);
        let v = self.amplitude * env * self.phase.sin();
        self.phase += self.inc;
        if self.phase >= TAU {
            self.phase -= TAU;
        }
        self.remaining -= 1;
        v
    }

    /// Soma o tom à saída intercalada (mesmo sinal em todos os canais).
    pub(crate) fn add_to(&mut self, output: &mut [f32], out_channels: usize) {
        if out_channels == 0 || !self.is_active() {
            return;
        }
        for frame in output.chunks_exact_mut(out_channels) {
            if !self.is_active() {
                return;
            }
            let v = self.next();
            for x in frame {
                *x += v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_for_the_requested_frames_with_fades() {
        let mut t = TestTone::default();
        t.start(4_800, 0.5, 48_000);
        let mut out = vec![0.0f32; 6_000 * 2];
        t.add_to(&mut out, 2);
        let l: Vec<f32> = out.iter().step_by(2).copied().collect();
        assert!(l[0].abs() < 0.01, "começa com rampa");
        let peak = l.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        assert!((peak - 0.5).abs() < 0.01, "pico {peak}");
        assert!(
            l[4_800..].iter().all(|v| *v == 0.0),
            "acaba no tempo pedido"
        );
        assert!(l[4_799].abs() < 0.01, "termina com rampa");
        assert_eq!(out[100], out[101], "mesmo sinal nos dois canais");
        assert!(!t.is_active());
    }

    #[test]
    fn invalid_amplitude_is_silent() {
        let mut t = TestTone::default();
        t.start(100, f32::NAN, 48_000);
        let mut out = vec![0.0f32; 100];
        t.add_to(&mut out, 1);
        assert!(out.iter().all(|v| *v == 0.0));
    }
}
