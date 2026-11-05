//! Chorus estéreo: linha de atraso modulada por LFO senoidal (base de 7 ms ± profundidade),
//! LFOs dos canais defasados em 90° para abertura estéreo, interpolação linear e mix.

use crate::smoothing::ParamSmoother;
use crate::util::{clamp_param, safe_sr, sanitize};
use crate::Processor;
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChorusParams {
    pub rate_hz: f32,
    pub depth_ms: f32,
    pub mix: f32,
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            rate_hz: 0.8,
            depth_ms: 3.0,
            mix: 0.4,
        }
    }
}

impl ChorusParams {
    pub fn clamped(self) -> Self {
        Self {
            rate_hz: clamp_param(self.rate_hz, 0.05, 10.0, 0.8),
            depth_ms: clamp_param(self.depth_ms, 0.1, 15.0, 3.0),
            mix: clamp_param(self.mix, 0.0, 1.0, 0.4),
        }
    }
}

/// Atraso central do chorus.
pub const CHORUS_BASE_MS: f32 = 7.0;
const MAX_DEPTH_MS: f32 = 15.0;

#[derive(Debug, Clone)]
pub struct Chorus {
    params: ChorusParams,
    sample_rate: f32,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write: usize,
    phase: f32,
    phase_inc: ParamSmoother,
    depth: ParamSmoother,
    mix: ParamSmoother,
}

impl Chorus {
    pub fn new() -> Self {
        let mut c = Self {
            params: ChorusParams::default(),
            sample_rate: 48_000.0,
            buf_l: Vec::new(),
            buf_r: Vec::new(),
            write: 0,
            phase: 0.0,
            phase_inc: ParamSmoother::new(0.0),
            depth: ParamSmoother::new(0.0),
            mix: ParamSmoother::new(0.0),
        };
        c.prepare(48_000.0, 0);
        c
    }

    pub fn set_params(&mut self, p: ChorusParams) {
        let p = p.clamped();
        self.params = p;
        self.phase_inc
            .set_target(TAU * p.rate_hz / self.sample_rate);
        self.depth.set_target(p.depth_ms * 0.001 * self.sample_rate);
        self.mix.set_target(p.mix);
    }

    pub fn params(&self) -> ChorusParams {
        self.params
    }

    #[inline]
    fn read(buf: &[f32], write: usize, delay: f32) -> f32 {
        let len = buf.len();
        let d = delay.clamp(1.0, (len - 2) as f32);
        let di = d as usize;
        let frac = d - di as f32;
        let i0 = (write + len - di) % len;
        let i1 = (i0 + len - 1) % len;
        buf[i0] * (1.0 - frac) + buf[i1] * frac
    }
}

impl Default for Chorus {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Chorus {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        let len = ((CHORUS_BASE_MS + MAX_DEPTH_MS) * 0.001 * self.sample_rate) as usize + 4;
        self.buf_l = vec![0.0; len];
        self.buf_r = vec![0.0; len];
        self.phase_inc.set_time(20.0, self.sample_rate);
        self.depth.set_time(20.0, self.sample_rate);
        self.mix.set_time(20.0, self.sample_rate);
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.buf_l.iter_mut().for_each(|v| *v = 0.0);
        self.buf_r.iter_mut().for_each(|v| *v = 0.0);
        self.write = 0;
        self.phase = 0.0;
        self.phase_inc.snap();
        self.depth.snap();
        self.mix.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let len = self.buf_l.len();
        if len < 4 || self.buf_r.len() != len {
            return;
        }
        let base = CHORUS_BASE_MS * 0.001 * self.sample_rate;
        let n = left.len().min(right.len());
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let (xl, xr) = (sanitize(*l), sanitize(*r));
            self.buf_l[self.write] = xl;
            self.buf_r[self.write] = xr;
            let depth = self.depth.next_value();
            let mix = self.mix.next_value();
            // sen(φ + 90°) = cos φ: uma única chamada a sin_cos para os dois LFOs.
            let (s, c) = self.phase.sin_cos();
            let dl = base + depth * s;
            let dr = base + depth * c;
            let wl = Self::read(&self.buf_l, self.write, dl);
            let wr = Self::read(&self.buf_r, self.write, dr);
            self.phase += self.phase_inc.next_value();
            if self.phase >= TAU {
                self.phase -= TAU;
            }
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
    use crate::signals::{impulse, peak, process_in_blocks, sine};

    const SR: f32 = 48_000.0;

    fn chorus(p: ChorusParams) -> Chorus {
        let mut c = Chorus::new();
        c.prepare(SR, 128);
        c.set_params(p);
        c.reset();
        c
    }

    #[test]
    fn wet_impulse_lands_within_modulation_range() {
        let mut c = chorus(ChorusParams {
            rate_hz: 0.5,
            depth_ms: 2.0,
            mix: 1.0,
        });
        let mut l = impulse(2_000, 0, 1.0);
        let mut r = l.clone();
        c.process(&mut l, &mut r);
        // Atraso entre 5 ms (240) e 9 ms (432) amostras.
        let first = l.iter().position(|v| v.abs() > 1e-6).unwrap();
        assert!((240..=432).contains(&first), "eco em {first}");
        assert!(l[..240].iter().all(|v| *v == 0.0));
    }

    #[test]
    fn modulation_varies_delay_over_time_and_channels_differ() {
        let mut c = chorus(ChorusParams {
            rate_hz: 2.0,
            depth_ms: 5.0,
            mix: 1.0,
        });
        let mut l = sine(440.0, 0.5, SR, 48_000, 0.0);
        let mut r = l.clone();
        process_in_blocks(&mut c, &mut l, &mut r, 128);
        let diff: f32 = l.iter().zip(&r).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 100.0, "canais devem diferir (LFO defasado): {diff}");
        assert!(
            peak(&l) <= 0.5 + 1e-3,
            "interpolação não excede o pico de entrada"
        );
    }

    #[test]
    fn zero_mix_is_dry() {
        let mut c = chorus(ChorusParams {
            mix: 0.0,
            ..Default::default()
        });
        let x = sine(300.0, 0.7, SR, 2_000, 0.0);
        let mut l = x.clone();
        let mut r = x.clone();
        c.process(&mut l, &mut r);
        assert_eq!(l, x);
    }
}
