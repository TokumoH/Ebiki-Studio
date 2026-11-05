//! Saturação suave por `tanh` com *drive*, filtro de tom e mix, com compensação de ganho.
//!
//! `y = c · tanh(g·x)`, onde `g` é o *drive* linear e `c = REF / tanh(g·REF)` mantém um sinal de
//! referência de −12 dBFS (`REF = 0,25`) com o mesmo pico, para qualquer *drive*. Como
//! `|tanh| ≤ 1` e `c ≤ 1,03`, a parte processada é limitada a ±1,03 qualquer que seja a entrada.
//! O tom é um passa-baixa de um polo (resposta ao impulso positiva → não ultrapassa o pico).
//!
//! Limitação conhecida: sem *oversampling* (aliasing com *drive* alto em agudos).

use crate::smoothing::ParamSmoother;
use crate::util::{clamp_param, db_to_lin, flush, safe_sr, sanitize};
use crate::Processor;
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SaturationParams {
    pub drive_db: f32,
    /// 0 = escuro (corte em 1,5 kHz), 1 = aberto (corte em 20 kHz).
    pub tone: f32,
    pub mix: f32,
}

impl Default for SaturationParams {
    fn default() -> Self {
        Self {
            drive_db: 6.0,
            tone: 0.5,
            mix: 1.0,
        }
    }
}

impl SaturationParams {
    pub fn clamped(self) -> Self {
        Self {
            drive_db: clamp_param(self.drive_db, 0.0, 36.0, 6.0),
            tone: clamp_param(self.tone, 0.0, 1.0, 0.5),
            mix: clamp_param(self.mix, 0.0, 1.0, 1.0),
        }
    }
}

const REF_LEVEL: f32 = 0.25;
const TONE_MIN_HZ: f32 = 1_500.0;
const TONE_MAX_HZ: f32 = 20_000.0;

/// Limite teórico da parte processada (`max c` ocorre com drive 0 dB).
pub const SATURATION_WET_BOUND: f32 = 1.03;

#[derive(Debug, Clone)]
pub struct Saturation {
    params: SaturationParams,
    sample_rate: f32,
    drive: ParamSmoother,
    comp: ParamSmoother,
    mix: ParamSmoother,
    tone_coef: ParamSmoother,
    lp_l: f32,
    lp_r: f32,
}

impl Saturation {
    pub fn new() -> Self {
        let mut s = Self {
            params: SaturationParams::default(),
            sample_rate: 48_000.0,
            drive: ParamSmoother::new(1.0),
            comp: ParamSmoother::new(1.0),
            mix: ParamSmoother::new(1.0),
            tone_coef: ParamSmoother::new(0.0),
            lp_l: 0.0,
            lp_r: 0.0,
        };
        s.prepare(48_000.0, 0);
        s
    }

    /// Fator de compensação para um *drive* linear `g`.
    pub fn compensation(g: f32) -> f32 {
        REF_LEVEL / (g * REF_LEVEL).tanh()
    }

    pub fn set_params(&mut self, p: SaturationParams) {
        let p = p.clamped();
        self.params = p;
        let g = db_to_lin(p.drive_db);
        self.drive.set_target(g);
        self.comp.set_target(Self::compensation(g));
        self.mix.set_target(p.mix);
        // Interpolação logarítmica da frequência de corte.
        let fc =
            (TONE_MIN_HZ * (TONE_MAX_HZ / TONE_MIN_HZ).powf(p.tone)).min(0.45 * self.sample_rate);
        self.tone_coef
            .set_target((-TAU * fc / self.sample_rate).exp());
    }

    pub fn params(&self) -> SaturationParams {
        self.params
    }
}

impl Default for Saturation {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Saturation {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        for s in [
            &mut self.drive,
            &mut self.comp,
            &mut self.mix,
            &mut self.tone_coef,
        ] {
            s.set_time(15.0, self.sample_rate);
        }
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.lp_l = 0.0;
        self.lp_r = 0.0;
        self.drive.snap();
        self.comp.snap();
        self.mix.snap();
        self.tone_coef.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let (xl, xr) = (sanitize(*l), sanitize(*r));
            let g = self.drive.next_value();
            let c = self.comp.next_value();
            let mix = self.mix.next_value();
            let a = self.tone_coef.next_value();
            let sl = c * (g * xl).tanh();
            let sr = c * (g * xr).tanh();
            self.lp_l = flush(sl * (1.0 - a) + self.lp_l * a);
            self.lp_r = flush(sr * (1.0 - a) + self.lp_r * a);
            *l = xl * (1.0 - mix) + self.lp_l * mix;
            *r = xr * (1.0 - mix) + self.lp_r * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{peak, peak_db, process_in_blocks, sine};

    const SR: f32 = 48_000.0;

    fn sat(p: SaturationParams) -> Saturation {
        let mut s = Saturation::new();
        s.prepare(SR, 128);
        s.set_params(p);
        s.reset();
        s
    }

    #[test]
    fn output_is_bounded_for_hot_input() {
        let mut s = sat(SaturationParams {
            drive_db: 36.0,
            tone: 1.0,
            mix: 1.0,
        });
        let mut l = sine(100.0, 100.0, SR, 48_000, 0.0);
        let mut r = crate::signals::noise(48_000, 1_000.0, 2);
        process_in_blocks(&mut s, &mut l, &mut r, 128);
        assert!(peak(&l).max(peak(&r)) <= SATURATION_WET_BOUND);
        assert!(Saturation::compensation(1.0) <= SATURATION_WET_BOUND);
    }

    #[test]
    fn gain_compensation_keeps_reference_level() {
        for drive_db in [0.0, 12.0, 24.0] {
            let mut s = sat(SaturationParams {
                drive_db,
                tone: 1.0,
                mix: 1.0,
            });
            let mut l = sine(200.0, REF_LEVEL, SR, 24_000, 0.0);
            let mut r = l.clone();
            process_in_blocks(&mut s, &mut l, &mut r, 128);
            let out = peak_db(&l[12_000..]);
            assert!(
                (out - peak_db(&[REF_LEVEL])).abs() < 0.3,
                "drive {drive_db}: {out}"
            );
        }
    }

    #[test]
    fn drive_adds_harmonics() {
        let mut s = sat(SaturationParams {
            drive_db: 24.0,
            tone: 1.0,
            mix: 1.0,
        });
        let mut l = sine(1_000.0, 0.5, SR, 48_000, 0.0);
        let mut r = l.clone();
        s.process(&mut l, &mut r);
        // Projeção no 3º harmônico (3 kHz): tanh gera harmônicos ímpares.
        let h3: f64 = l[24_000..]
            .iter()
            .enumerate()
            .map(|(i, v)| *v as f64 * (std::f64::consts::TAU * 3_000.0 * i as f64 / 48_000.0).sin())
            .sum::<f64>()
            .abs()
            / 12_000.0;
        assert!(h3 > 0.01, "3º harmônico {h3}");
    }

    #[test]
    fn zero_mix_is_dry() {
        let mut s = sat(SaturationParams {
            mix: 0.0,
            ..Default::default()
        });
        let x = sine(500.0, 0.9, SR, 1_000, 0.0);
        let mut l = x.clone();
        let mut r = x.clone();
        s.process(&mut l, &mut r);
        assert_eq!(l, x);
    }

    #[test]
    fn dark_tone_attenuates_highs() {
        let mut s = sat(SaturationParams {
            drive_db: 0.0,
            tone: 0.0,
            mix: 1.0,
        });
        let mut l = sine(10_000.0, 0.1, SR, 24_000, 0.0);
        let mut r = l.clone();
        s.process(&mut l, &mut r);
        assert!(peak_db(&l[12_000..]) < peak_db(&[0.1]) - 10.0);
    }
}
