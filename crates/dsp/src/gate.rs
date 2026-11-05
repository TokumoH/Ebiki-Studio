//! Noise gate com limiar, ataque, *hold*, release e faixa (atenuação máxima).
//!
//! Detector de pico estéreo vinculado (ataque instantâneo, release de 10 ms) com histerese de
//! 3 dB; máquina de estados Aberto → Hold → Fechando; ganho suavizado por um polo.

use crate::util::{clamp_param, db_to_lin, safe_sr, sanitize, time_coef};
use crate::Processor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateParams {
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
    /// Atenuação quando fechado (−90 = praticamente mudo, 0 = gate sem efeito).
    pub range_db: f32,
}

impl Default for GateParams {
    fn default() -> Self {
        Self {
            threshold_db: -50.0,
            attack_ms: 1.0,
            hold_ms: 30.0,
            release_ms: 150.0,
            range_db: -60.0,
        }
    }
}

impl GateParams {
    pub fn clamped(self) -> Self {
        Self {
            threshold_db: clamp_param(self.threshold_db, -90.0, 0.0, -50.0),
            attack_ms: clamp_param(self.attack_ms, 0.05, 50.0, 1.0),
            hold_ms: clamp_param(self.hold_ms, 0.0, 500.0, 30.0),
            release_ms: clamp_param(self.release_ms, 5.0, 2_000.0, 150.0),
            range_db: clamp_param(self.range_db, -90.0, 0.0, -60.0),
        }
    }
}

/// Histerese: fecha somente quando o nível cai este tanto abaixo do limiar.
const HYSTERESIS_DB: f32 = 3.0;
const DETECTOR_RELEASE_MS: f32 = 10.0;

#[derive(Debug, Clone)]
pub struct Gate {
    params: GateParams,
    sample_rate: f32,
    open_thr: f32,
    close_thr: f32,
    floor: f32,
    att: f32,
    rel: f32,
    det_rel: f32,
    hold_samples: u32,
    env: f32,
    hold_left: u32,
    open: bool,
    gain: f32,
}

impl Gate {
    pub fn new() -> Self {
        let mut g = Self {
            params: GateParams::default(),
            sample_rate: 48_000.0,
            open_thr: 0.0,
            close_thr: 0.0,
            floor: 0.0,
            att: 0.0,
            rel: 0.0,
            det_rel: 0.0,
            hold_samples: 0,
            env: 0.0,
            hold_left: 0,
            open: false,
            gain: 0.0,
        };
        g.prepare(48_000.0, 0);
        g
    }

    pub fn set_params(&mut self, p: GateParams) {
        let p = p.clamped();
        self.params = p;
        let sr = self.sample_rate;
        self.open_thr = db_to_lin(p.threshold_db);
        self.close_thr = db_to_lin(p.threshold_db - HYSTERESIS_DB);
        self.floor = db_to_lin(p.range_db);
        self.att = time_coef(p.attack_ms, sr);
        self.rel = time_coef(p.release_ms, sr);
        self.det_rel = time_coef(DETECTOR_RELEASE_MS, sr);
        self.hold_samples = (p.hold_ms * 0.001 * sr) as u32;
    }

    pub fn params(&self) -> GateParams {
        self.params
    }

    /// `true` se o gate está aberto (sinal acima do limiar ou em *hold*).
    pub fn is_open(&self) -> bool {
        self.open || self.hold_left > 0
    }

    /// Ganho atual aplicado (linear, entre `range` e 1).
    pub fn current_gain(&self) -> f32 {
        self.gain
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Gate {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.env = 0.0;
        self.hold_left = 0;
        self.open = false;
        self.gain = self.floor;
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let (xl, xr) = (sanitize(*l), sanitize(*r));
            let level = xl.abs().max(xr.abs());
            self.env = if level > self.env {
                level
            } else {
                self.det_rel * self.env + (1.0 - self.det_rel) * level
            };
            if self.env >= self.open_thr {
                self.open = true;
                self.hold_left = self.hold_samples;
            } else if self.open && self.env < self.close_thr {
                self.open = false;
            } else if !self.open && self.hold_left > 0 {
                self.hold_left -= 1;
            }
            let (target, coef) = if self.is_open() {
                (1.0, self.att)
            } else {
                (self.floor, self.rel)
            };
            self.gain = target + coef * (self.gain - target);
            *l = xl * self.gain;
            *r = xr * self.gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{peak, peak_db, sine};

    const SR: f32 = 48_000.0;

    fn gate(p: GateParams) -> Gate {
        let mut g = Gate::new();
        g.prepare(SR, 128);
        g.set_params(p);
        g.reset();
        g
    }

    #[test]
    fn closes_below_threshold() {
        let mut g = gate(GateParams {
            threshold_db: -40.0,
            range_db: -80.0,
            ..Default::default()
        });
        let mut l = sine(200.0, db_to_lin(-60.0), SR, 48_000, 0.0);
        let mut r = l.clone();
        g.process(&mut l, &mut r);
        assert!(!g.is_open());
        // −60 dBFS atenuado em ~80 dB.
        assert!(peak_db(&l[24_000..]) < -130.0, "{}", peak_db(&l[24_000..]));
    }

    #[test]
    fn opens_above_threshold_and_passes_signal() {
        let mut g = gate(GateParams {
            threshold_db: -40.0,
            ..Default::default()
        });
        let mut l = sine(200.0, 0.5, SR, 24_000, 0.0);
        let mut r = l.clone();
        g.process(&mut l, &mut r);
        assert!(g.is_open());
        assert!((peak(&l[12_000..]) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn hold_keeps_gate_open_then_releases_to_range() {
        let mut g = gate(GateParams {
            threshold_db: -30.0,
            attack_ms: 0.1,
            hold_ms: 50.0,
            release_ms: 10.0,
            range_db: -40.0,
        });
        let mut loud = vec![0.5f32; 4_800];
        let mut loud_r = loud.clone();
        g.process(&mut loud, &mut loud_r);
        // Sinal baixo constante (−50 dBFS) após o trecho forte.
        let quiet_amp = db_to_lin(-50.0);
        let mut q = vec![quiet_amp; 48_000];
        let mut q_r = q.clone();
        g.process(&mut q, &mut q_r);
        // Detector leva ~10 ms × ln(0,497/0,0193) ≈ 32 ms para cair abaixo do limiar de fechamento
        // (−33 dBFS); com hold de 50 ms, ainda aberto a ~73 ms.
        assert!(
            (q[3_500] / quiet_amp - 1.0).abs() < 0.01,
            "ainda aberto no hold"
        );
        assert!((crate::util::lin_to_db(q[47_999] / quiet_amp) + 40.0).abs() < 0.1);
        assert!((g.current_gain() - db_to_lin(-40.0)).abs() < 1e-4);
    }

    #[test]
    fn range_zero_is_transparent() {
        let mut g = gate(GateParams {
            threshold_db: 0.0,
            range_db: 0.0,
            ..Default::default()
        });
        let mut l = sine(100.0, 0.01, SR, 4_800, 0.0);
        let orig = l.clone();
        let mut r = l.clone();
        g.process(&mut l, &mut r);
        assert!(l.iter().zip(&orig).all(|(a, b)| (a - b).abs() < 1e-6));
    }
}
