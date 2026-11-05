//! Suavização de parâmetros para evitar ruído de "zíper" em mudanças bruscas.

use crate::util::{clamp_param, safe_sr, time_coef};
use crate::Processor;

/// Rampa **linear** de ganho: chega ao alvo em exatamente `ramp_ms`.
///
/// Uso típico: fader, mute, mix. Também é um [`Processor`] que multiplica os dois canais.
#[derive(Debug, Clone)]
pub struct GainSmoother {
    current: f32,
    target: f32,
    step: f32,
    remaining: u32,
    ramp_ms: f32,
    ramp_samples: u32,
}

impl GainSmoother {
    /// Rampa padrão (20 ms).
    pub const DEFAULT_RAMP_MS: f32 = 20.0;

    pub fn new(initial: f32) -> Self {
        let g = clamp_param(initial, -1.0e6, 1.0e6, 0.0);
        let mut s = Self {
            current: g,
            target: g,
            step: 0.0,
            remaining: 0,
            ramp_ms: Self::DEFAULT_RAMP_MS,
            ramp_samples: 1,
        };
        s.prepare_rate(48_000.0);
        s
    }

    /// Recalcula a duração da rampa para a taxa de amostragem.
    pub fn prepare_rate(&mut self, sample_rate: f32) {
        let sr = safe_sr(sample_rate);
        self.ramp_samples = ((self.ramp_ms * 0.001 * sr).round() as u32).max(1);
    }

    /// Altera a duração da rampa (0..=1000 ms) — vale para a próxima mudança de alvo.
    pub fn set_ramp_ms(&mut self, ms: f32, sample_rate: f32) {
        self.ramp_ms = clamp_param(ms, 0.0, 1_000.0, Self::DEFAULT_RAMP_MS);
        self.prepare_rate(sample_rate);
    }

    /// Define um novo alvo; o valor caminha linearmente até ele.
    pub fn set_target(&mut self, target: f32) {
        let t = clamp_param(target, -1.0e6, 1.0e6, self.target);
        if t == self.target && self.remaining == 0 {
            return;
        }
        self.target = t;
        self.remaining = self.ramp_samples;
        self.step = (self.target - self.current) / self.ramp_samples as f32;
    }

    /// Salta imediatamente para o alvo.
    pub fn snap(&mut self) {
        self.current = self.target;
        self.remaining = 0;
        self.step = 0.0;
    }

    /// Valor atual (sem avançar).
    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    /// `true` enquanto a rampa não terminou.
    pub fn is_smoothing(&self) -> bool {
        self.remaining > 0
    }

    /// Avança uma amostra e retorna o valor.
    #[inline]
    pub fn next_value(&mut self) -> f32 {
        if self.remaining > 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            } else {
                self.current += self.step;
            }
        }
        self.current
    }
}

impl Default for GainSmoother {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Processor for GainSmoother {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.prepare_rate(sample_rate);
        self.snap();
    }

    fn reset(&mut self) {
        self.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        if !self.is_smoothing() {
            let g = self.current;
            if g == 1.0 {
                return;
            }
            for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
                *l *= g;
                *r *= g;
            }
            return;
        }
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let g = self.next_value();
            *l *= g;
            *r *= g;
        }
    }
}

/// Suavizador exponencial (um polo) para parâmetros contínuos (frequência, mix, drive…).
#[derive(Debug, Clone, Copy)]
pub struct ParamSmoother {
    current: f32,
    target: f32,
    coef: f32,
}

impl ParamSmoother {
    pub fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            coef: 0.0,
        }
    }

    /// Constante de tempo do suavizador.
    pub fn set_time(&mut self, ms: f32, sample_rate: f32) {
        self.coef = time_coef(ms, safe_sr(sample_rate));
    }

    pub fn set_target(&mut self, v: f32) {
        if v.is_finite() {
            self.target = v;
        }
    }

    pub fn snap(&mut self) {
        self.current = self.target;
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    /// `true` quando o valor atual já está a menos de `1e-5` (relativo) do alvo.
    pub fn is_settled(&self) -> bool {
        (self.current - self.target).abs() <= 1.0e-5 * self.target.abs().max(1.0)
    }

    /// Avança uma amostra.
    #[inline]
    pub fn next_value(&mut self) -> f32 {
        if self.is_settled() {
            self.current = self.target;
        } else {
            self.current = self.target + self.coef * (self.current - self.target);
        }
        self.current
    }

    /// Avança `n` amostras de uma vez (aproximação por potência do coeficiente).
    #[inline]
    pub fn advance(&mut self, n: usize) -> f32 {
        if self.is_settled() {
            self.current = self.target;
        } else {
            let k = self.coef.powi(n.min(i32::MAX as usize) as i32);
            self.current = self.target + k * (self.current - self.target);
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_ramp_reaches_target_in_exact_time() {
        let mut g = GainSmoother::new(0.0);
        g.set_ramp_ms(10.0, 48_000.0);
        g.set_target(1.0);
        let vals: Vec<f32> = (0..480).map(|_| g.next_value()).collect();
        assert!(
            (vals[239] - 0.5).abs() < 0.01,
            "meio da rampa: {}",
            vals[239]
        );
        assert_eq!(vals[479], 1.0);
        assert!(!g.is_smoothing());
        // Monotônica: sem saltos.
        assert!(vals.windows(2).all(|w| w[1] >= w[0]));
        assert!(vals.windows(2).all(|w| (w[1] - w[0]) < 0.003));
    }

    #[test]
    fn gain_smoother_processes_both_channels() {
        let mut g = GainSmoother::new(0.5);
        g.prepare(48_000.0, 64);
        let mut l = [1.0f32; 8];
        let mut r = [-1.0f32; 8];
        g.process(&mut l, &mut r);
        assert!(l.iter().all(|v| *v == 0.5));
        assert!(r.iter().all(|v| *v == -0.5));
        g.set_target(f32::NAN);
        assert_eq!(g.target(), 0.5, "NaN é ignorado");
    }

    #[test]
    fn param_smoother_converges_exponentially() {
        let mut p = ParamSmoother::new(0.0);
        p.set_time(5.0, 48_000.0);
        p.set_target(1.0);
        for _ in 0..240 {
            p.next_value();
        }
        assert!((p.current() - 0.632).abs() < 0.01);
        for _ in 0..48_000 {
            p.next_value();
        }
        assert_eq!(p.current(), 1.0);
        let mut q = ParamSmoother::new(0.0);
        q.set_time(5.0, 48_000.0);
        q.set_target(1.0);
        q.advance(240);
        assert!((q.current() - 0.632).abs() < 0.01);
    }
}
