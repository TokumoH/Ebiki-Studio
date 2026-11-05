//! Delay estéreo com realimentação (< 1), filtro passa-baixa (*high-cut*) no laço, mix e modo
//! *ping-pong*. Buffer de 2 s pré-alocado em `prepare`; tempo suavizado (sem estalos, com leve
//! glissando ao mudar) e leitura com interpolação linear.

use crate::smoothing::ParamSmoother;
use crate::util::{clamp_param, flush, safe_sr, sanitize};
use crate::Processor;
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayParams {
    pub time_ms: f32,
    pub feedback: f32,
    pub high_cut_hz: f32,
    pub mix: f32,
    pub ping_pong: bool,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            time_ms: 375.0,
            feedback: 0.35,
            high_cut_hz: 6_000.0,
            mix: 0.2,
            ping_pong: false,
        }
    }
}

impl DelayParams {
    pub fn clamped(self) -> Self {
        Self {
            time_ms: clamp_param(self.time_ms, 1.0, MAX_DELAY_MS, 375.0),
            feedback: clamp_param(self.feedback, 0.0, 0.95, 0.35),
            high_cut_hz: clamp_param(self.high_cut_hz, 500.0, 20_000.0, 6_000.0),
            mix: clamp_param(self.mix, 0.0, 1.0, 0.2),
            ..self
        }
    }
}

pub const MAX_DELAY_MS: f32 = 2_000.0;

#[derive(Debug, Clone)]
pub struct Delay {
    params: DelayParams,
    sample_rate: f32,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write: usize,
    time_samples: ParamSmoother,
    feedback: ParamSmoother,
    mix: ParamSmoother,
    lp_coef: f32,
    lp_l: f32,
    lp_r: f32,
}

impl Delay {
    pub fn new() -> Self {
        let mut d = Self {
            params: DelayParams::default(),
            sample_rate: 48_000.0,
            buf_l: Vec::new(),
            buf_r: Vec::new(),
            write: 0,
            time_samples: ParamSmoother::new(1.0),
            feedback: ParamSmoother::new(0.0),
            mix: ParamSmoother::new(0.0),
            lp_coef: 0.0,
            lp_l: 0.0,
            lp_r: 0.0,
        };
        d.prepare(48_000.0, 0);
        d
    }

    pub fn set_params(&mut self, p: DelayParams) {
        let p = p.clamped();
        self.params = p;
        let max = self.buf_l.len().saturating_sub(2).max(1) as f32;
        self.time_samples
            .set_target((p.time_ms * self.sample_rate / 1_000.0).clamp(1.0, max));
        self.feedback.set_target(p.feedback);
        self.mix.set_target(p.mix);
        let fc = p.high_cut_hz.min(0.45 * self.sample_rate);
        self.lp_coef = (-TAU * fc / self.sample_rate).exp();
    }

    pub fn params(&self) -> DelayParams {
        self.params
    }

    #[inline]
    fn read(buf: &[f32], write: usize, delay: f32) -> f32 {
        let len = buf.len();
        let d = delay.max(1.0);
        let di = d as usize;
        let frac = d - di as f32;
        let i0 = (write + len - di % len) % len;
        let i1 = (i0 + len - 1) % len;
        buf[i0] * (1.0 - frac) + buf[i1] * frac
    }
}

impl Default for Delay {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Delay {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        let len = (MAX_DELAY_MS * 0.001 * self.sample_rate) as usize + 4;
        self.buf_l = vec![0.0; len];
        self.buf_r = vec![0.0; len];
        self.time_samples.set_time(50.0, self.sample_rate);
        self.feedback.set_time(20.0, self.sample_rate);
        self.mix.set_time(20.0, self.sample_rate);
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.buf_l.iter_mut().for_each(|v| *v = 0.0);
        self.buf_r.iter_mut().for_each(|v| *v = 0.0);
        self.write = 0;
        self.lp_l = 0.0;
        self.lp_r = 0.0;
        self.time_samples.snap();
        self.feedback.snap();
        self.mix.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let len = self.buf_l.len();
        if len < 4 || self.buf_r.len() != len {
            return;
        }
        let n = left.len().min(right.len());
        let a = self.lp_coef;
        let ping_pong = self.params.ping_pong;
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let (xl, xr) = (sanitize(*l), sanitize(*r));
            let d = self.time_samples.next_value();
            let fb = self.feedback.next_value();
            let mix = self.mix.next_value();
            let wl = Self::read(&self.buf_l, self.write, d);
            let wr = Self::read(&self.buf_r, self.write, d);
            // High-cut apenas no laço de realimentação: o 1º eco sai íntegro.
            self.lp_l = flush(wl * (1.0 - a) + self.lp_l * a);
            self.lp_r = flush(wr * (1.0 - a) + self.lp_r * a);
            let (in_l, in_r) = if ping_pong {
                // Entrada mono no lado esquerdo; realimentação cruzada L↔R.
                ((xl + xr) * 0.5 + fb * self.lp_r, fb * self.lp_l)
            } else {
                (xl + fb * self.lp_l, xr + fb * self.lp_r)
            };
            self.buf_l[self.write] = in_l;
            self.buf_r[self.write] = in_r;
            self.write += 1;
            if self.write >= len {
                self.write = 0;
            }
            *l = xl * (1.0 - mix) + wl * mix;
            *r = xr * (1.0 - mix) + wr * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{impulse, peak, process_in_blocks};

    const SR: f32 = 48_000.0;

    fn delay(p: DelayParams) -> Delay {
        let mut d = Delay::new();
        d.prepare(SR, 128);
        d.set_params(p);
        d.reset();
        d
    }

    fn argmax_after(x: &[f32], from: usize) -> usize {
        let mut best = from;
        for i in from..x.len() {
            if x[i].abs() > x[best].abs() {
                best = i;
            }
        }
        best
    }

    #[test]
    fn echo_appears_at_exact_offset() {
        let mut d = delay(DelayParams {
            time_ms: 10.0,
            feedback: 0.5,
            high_cut_hz: 20_000.0,
            mix: 0.5,
            ping_pong: false,
        });
        let mut l = impulse(2_000, 0, 1.0);
        let mut r = l.clone();
        process_in_blocks(&mut d, &mut l, &mut r, 64);
        assert!((l[0] - 0.5).abs() < 1e-6, "seco");
        assert!(
            (l[480] - 0.5).abs() < 1e-6,
            "1º eco em 480 amostras: {}",
            l[480]
        );
        assert!(l[1..480].iter().all(|v| *v == 0.0));
        assert_eq!(argmax_after(&l, 481), 960, "2º eco");
        assert!(l[960] < l[480] && l[960] > 0.1);
    }

    #[test]
    fn feedback_tail_decays_with_high_cut() {
        let mut d = delay(DelayParams {
            time_ms: 5.0,
            feedback: 0.95,
            high_cut_hz: 2_000.0,
            mix: 1.0,
            ping_pong: false,
        });
        let mut l = impulse(48_000, 0, 1.0);
        let mut r = l.clone();
        process_in_blocks(&mut d, &mut l, &mut r, 128);
        assert!(peak(&l[40_000..]) < 0.01 * peak(&l[..1_000]));
    }

    #[test]
    fn ping_pong_alternates_channels() {
        let mut d = delay(DelayParams {
            time_ms: 10.0,
            feedback: 0.6,
            high_cut_hz: 20_000.0,
            mix: 1.0,
            ping_pong: true,
        });
        let mut l = impulse(2_000, 0, 1.0);
        let mut r = l.clone();
        d.process(&mut l, &mut r);
        assert!(l[480] > 0.9 && r[480].abs() < 1e-6, "1º eco à esquerda");
        assert!(r[960] > 0.45 && l[960].abs() < 1e-3, "2º eco à direita");
        assert!(
            l[1_440] > 0.25 && r[1_440].abs() < 1e-3,
            "3º eco à esquerda"
        );
    }

    #[test]
    fn time_change_is_click_free_and_feedback_is_clamped() {
        let mut d = delay(DelayParams {
            feedback: 5.0,
            ..Default::default()
        });
        assert!(d.params().feedback < 1.0);
        let mut l = crate::signals::sine(300.0, 0.5, SR, 9_600, 0.0);
        let mut r = l.clone();
        d.process(&mut l[..4_800], &mut r[..4_800]);
        d.set_params(DelayParams {
            time_ms: 50.0,
            mix: 1.0,
            ..Default::default()
        });
        d.process(&mut l[4_800..], &mut r[4_800..]);
        let max_step = l
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(max_step < 0.2, "salto {max_step}");
    }

    #[test]
    fn max_time_fits_preallocated_buffer() {
        let mut d = delay(DelayParams {
            time_ms: 2_000.0,
            mix: 1.0,
            ..Default::default()
        });
        let mut l = impulse(96_100, 0, 1.0);
        let mut r = l.clone();
        process_in_blocks(&mut d, &mut l, &mut r, 512);
        assert!((l[96_000] - 1.0).abs() < 1e-6);
    }
}
