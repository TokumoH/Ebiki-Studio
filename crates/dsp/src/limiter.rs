//! Limitador *brickwall* com *lookahead* (~1,5 ms).
//!
//! Algoritmo (garantia matemática de teto):
//! 1. ganho requerido por amostra `g_req[n] = min(1, teto / max(|L|,|R|))`;
//! 2. mínimo deslizante de `g_req` numa janela de `W = L + 1` amostras (L = lookahead);
//! 3. release exponencial que **só sobe** (`r[n] ≤ mín[n]`);
//! 4. média móvel (janela retangular) de `W` amostras → ataque suave em rampa;
//! 5. o sinal é atrasado `L` amostras.
//!
//! Cada termo da média cobre a amostra atrasada, logo o ganho aplicado ≤ `g_req` dela e a saída
//! nunca passa do teto. Um *clamp* final protege contra erros de arredondamento.

use crate::smoothing::GainSmoother;
use crate::util::{clamp_param, db_to_lin, lin_to_db, safe_sr, sanitize, time_coef};
use crate::Processor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimiterParams {
    pub ceiling_db: f32,
    pub input_gain_db: f32,
    pub release_ms: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self {
            ceiling_db: -1.0,
            input_gain_db: 0.0,
            release_ms: 80.0,
        }
    }
}

impl LimiterParams {
    pub fn clamped(self) -> Self {
        Self {
            ceiling_db: clamp_param(self.ceiling_db, -24.0, 0.0, -1.0),
            input_gain_db: clamp_param(self.input_gain_db, 0.0, 24.0, 0.0),
            release_ms: clamp_param(self.release_ms, 1.0, 1_000.0, 80.0),
        }
    }
}

/// Lookahead em milissegundos.
pub const LIMITER_LOOKAHEAD_MS: f32 = 1.5;

#[derive(Debug, Clone)]
pub struct Limiter {
    params: LimiterParams,
    sample_rate: f32,
    ceiling: f32,
    rel: f32,
    input_gain: GainSmoother,
    lookahead: usize,
    /// Tamanho das janelas (`lookahead + 1`).
    win: usize,
    pos: usize,
    delay_l: Vec<f32>,
    delay_r: Vec<f32>,
    greq: Vec<f32>,
    smooth: Vec<f32>,
    sum: f64,
    released: f32,
    last_gain: f32,
}

impl Limiter {
    pub fn new() -> Self {
        let mut l = Self {
            params: LimiterParams::default(),
            sample_rate: 48_000.0,
            ceiling: 1.0,
            rel: 0.0,
            input_gain: GainSmoother::new(1.0),
            lookahead: 0,
            win: 1,
            pos: 0,
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            greq: Vec::new(),
            smooth: Vec::new(),
            sum: 0.0,
            released: 1.0,
            last_gain: 1.0,
        };
        l.prepare(48_000.0, 0);
        l
    }

    pub fn set_params(&mut self, p: LimiterParams) {
        self.params = p.clamped();
        self.ceiling = db_to_lin(self.params.ceiling_db);
        self.rel = time_coef(self.params.release_ms, self.sample_rate);
        self.input_gain
            .set_target(db_to_lin(self.params.input_gain_db));
    }

    pub fn params(&self) -> LimiterParams {
        self.params
    }

    /// Redução de ganho atual em dB (≥ 0).
    pub fn gain_reduction_db(&self) -> f32 {
        -lin_to_db(self.last_gain).min(0.0)
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Limiter {
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        self.lookahead =
            ((LIMITER_LOOKAHEAD_MS * 0.001 * self.sample_rate).round() as usize).max(1);
        self.win = self.lookahead + 1;
        self.delay_l = vec![0.0; self.win];
        self.delay_r = vec![0.0; self.win];
        self.greq = vec![1.0; self.win];
        self.smooth = vec![1.0; self.win];
        self.input_gain.prepare(self.sample_rate, max_block);
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.delay_l.iter_mut().for_each(|v| *v = 0.0);
        self.delay_r.iter_mut().for_each(|v| *v = 0.0);
        self.greq.iter_mut().for_each(|v| *v = 1.0);
        self.smooth.iter_mut().for_each(|v| *v = 1.0);
        self.sum = self.win as f64;
        self.pos = 0;
        self.released = 1.0;
        self.last_gain = 1.0;
        self.input_gain.snap();
    }

    fn latency_samples(&self) -> usize {
        self.lookahead
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let w = self.win;
        if self.delay_l.len() != w
            || self.delay_r.len() != w
            || self.greq.len() != w
            || self.smooth.len() != w
        {
            return; // não preparado: passa o sinal sem alteração
        }
        // Recalcula a soma exata a cada bloco (evita deriva numérica).
        self.sum = self.smooth.iter().map(|v| *v as f64).sum();
        let ceil = self.ceiling;
        let inv_w = 1.0 / w as f64;
        let n = left.len().min(right.len());
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let g_in = self.input_gain.next_value();
            let xl = sanitize(*l) * g_in;
            let xr = sanitize(*r) * g_in;
            let pos = self.pos;
            self.delay_l[pos] = xl;
            self.delay_r[pos] = xr;
            let pk = xl.abs().max(xr.abs());
            self.greq[pos] = if pk > ceil { ceil / pk } else { 1.0 };
            let hold = self.greq.iter().fold(1.0f32, |m, v| m.min(*v));
            self.released = if hold < self.released {
                hold
            } else {
                hold + self.rel * (self.released - hold)
            };
            self.sum += self.released as f64 - self.smooth[pos] as f64;
            self.smooth[pos] = self.released;
            let g = ((self.sum * inv_w) as f32).min(1.0);
            self.last_gain = g;
            let read = if pos + 1 == w { 0 } else { pos + 1 };
            *l = (self.delay_l[read] * g).clamp(-ceil, ceil);
            *r = (self.delay_r[read] * g).clamp(-ceil, ceil);
            self.pos = read;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{noise, peak, process_in_blocks, sine};

    const SR: f32 = 48_000.0;

    fn limiter(p: LimiterParams) -> Limiter {
        let mut l = Limiter::new();
        l.prepare(SR, 128);
        l.set_params(p);
        l.reset();
        l
    }

    #[test]
    fn hot_signals_never_exceed_ceiling() {
        let ceiling_db = -1.0;
        let ceil = db_to_lin(ceiling_db);
        let mut lim = limiter(LimiterParams {
            ceiling_db,
            input_gain_db: 12.0,
            release_ms: 50.0,
        });
        // Ruído a +12 dB de entrada (pico ≈ 4×) + seno quente + impulsos isolados.
        let len = 96_000;
        let mut l = noise(len, 1.0, 3);
        let mut r = sine(97.0, 2.0, SR, len, 0.3);
        l[50_000] = 50.0;
        r[70_000] = -80.0;
        process_in_blocks(&mut lim, &mut l, &mut r, 64);
        let p = peak(&l).max(peak(&r));
        assert!(p <= ceil, "pico {p} > teto {ceil}");
        assert!(lim.gain_reduction_db() > 0.0);
    }

    #[test]
    fn transient_is_caught_by_lookahead_without_clipping_shape() {
        let mut lim = limiter(LimiterParams {
            ceiling_db: -6.0,
            ..Default::default()
        });
        let mut l = vec![0.0f32; 4_000];
        l[1_000] = 1.0;
        let mut r = l.clone();
        lim.process(&mut l, &mut r);
        let la = lim.latency_samples();
        assert_eq!(la, 72);
        // O impulso sai atrasado, exatamente no teto (ganho calculado, não clamp).
        assert!(
            (l[1_000 + la] - db_to_lin(-6.0)).abs() < 1e-4,
            "{}",
            l[1_000 + la]
        );
        assert!(peak(&l) <= db_to_lin(-6.0));
    }

    #[test]
    fn quiet_signal_passes_unchanged_but_delayed() {
        let mut lim = limiter(LimiterParams::default());
        let x = sine(1_000.0, 0.3, SR, 2_000, 0.0);
        let mut l = x.clone();
        let mut r = x.clone();
        lim.process(&mut l, &mut r);
        let la = lim.latency_samples();
        for i in la..2_000 {
            assert!((l[i] - x[i - la]).abs() < 1e-6);
        }
        assert_eq!(lim.gain_reduction_db(), 0.0);
    }

    #[test]
    fn gain_recovers_with_release() {
        let mut lim = limiter(LimiterParams {
            ceiling_db: -10.0,
            release_ms: 20.0,
            ..Default::default()
        });
        let mut l = vec![1.0f32; 480];
        let mut r = l.clone();
        lim.process(&mut l, &mut r);
        assert!((lim.gain_reduction_db() - 10.0).abs() < 0.1);
        let mut z = vec![0.0f32; 9_600];
        let mut z2 = z.clone();
        lim.process(&mut z, &mut z2);
        assert!(lim.gain_reduction_db() < 0.1);
    }
}
