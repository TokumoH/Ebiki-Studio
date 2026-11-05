//! Reverb algorítmico estilo Freeverb (Jezar, domínio público): 8 filtros pente com
//! amortecimento passa-baixa + 4 passa-tudo em série por canal, espalhamento estéreo de 23
//! amostras, pré-delay de até 200 ms, largura e mix.
//!
//! Os comprimentos originais (definidos a 44,1 kHz) são escalados para a taxa de amostragem.

use crate::smoothing::ParamSmoother;
use crate::util::{clamp_param, flush, safe_sr, sanitize};
use crate::Processor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParams {
    pub room_size: f32,
    pub damping: f32,
    pub width: f32,
    pub predelay_ms: f32,
    pub mix: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.6,
            damping: 0.5,
            width: 1.0,
            predelay_ms: 20.0,
            mix: 0.25,
        }
    }
}

impl ReverbParams {
    pub fn clamped(self) -> Self {
        Self {
            room_size: clamp_param(self.room_size, 0.0, 1.0, 0.6),
            damping: clamp_param(self.damping, 0.0, 1.0, 0.5),
            width: clamp_param(self.width, 0.0, 1.0, 1.0),
            predelay_ms: clamp_param(self.predelay_ms, 0.0, MAX_PREDELAY_MS, 20.0),
            mix: clamp_param(self.mix, 0.0, 1.0, 0.25),
        }
    }
}

pub const MAX_PREDELAY_MS: f32 = 200.0;

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const FIXED_GAIN: f32 = 0.015;
const SCALE_WET: f32 = 3.0;
const SCALE_DAMP: f32 = 0.4;
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;
const ALLPASS_FEEDBACK: f32 = 0.5;

#[derive(Debug, Clone, Default)]
struct Comb {
    buf: Vec<f32>,
    idx: usize,
    store: f32,
}

impl Comb {
    #[inline]
    fn tick(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let Some(out) = self.buf.get(self.idx).copied() else {
            return 0.0;
        };
        self.store = flush(out * (1.0 - damp) + self.store * damp);
        self.buf[self.idx] = x + self.store * feedback;
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }

    fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
        self.idx = 0;
        self.store = 0.0;
    }
}

#[derive(Debug, Clone, Default)]
struct Allpass {
    buf: Vec<f32>,
    idx: usize,
}

impl Allpass {
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let Some(b) = self.buf.get(self.idx).copied() else {
            return x;
        };
        let out = b - x;
        self.buf[self.idx] = flush(x + b * ALLPASS_FEEDBACK);
        self.idx += 1;
        if self.idx >= self.buf.len() {
            self.idx = 0;
        }
        out
    }

    fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
        self.idx = 0;
    }
}

#[derive(Debug, Clone)]
pub struct Reverb {
    params: ReverbParams,
    sample_rate: f32,
    comb_l: [Comb; 8],
    comb_r: [Comb; 8],
    ap_l: [Allpass; 4],
    ap_r: [Allpass; 4],
    predelay: Vec<f32>,
    pd_write: usize,
    pd_samples: usize,
    feedback: ParamSmoother,
    damp: ParamSmoother,
    width: ParamSmoother,
    mix: ParamSmoother,
}

impl Reverb {
    pub fn new() -> Self {
        let mut r = Self {
            params: ReverbParams::default(),
            sample_rate: 48_000.0,
            comb_l: Default::default(),
            comb_r: Default::default(),
            ap_l: Default::default(),
            ap_r: Default::default(),
            predelay: Vec::new(),
            pd_write: 0,
            pd_samples: 0,
            feedback: ParamSmoother::new(0.0),
            damp: ParamSmoother::new(0.0),
            width: ParamSmoother::new(1.0),
            mix: ParamSmoother::new(0.0),
        };
        r.prepare(48_000.0, 0);
        r
    }

    /// Sem alocação: o buffer de pré-delay já comporta 200 ms.
    pub fn set_params(&mut self, p: ReverbParams) {
        let p = p.clamped();
        self.params = p;
        self.feedback
            .set_target(p.room_size * SCALE_ROOM + OFFSET_ROOM);
        self.damp.set_target(p.damping * SCALE_DAMP);
        self.width.set_target(p.width);
        self.mix.set_target(p.mix);
        let max = self.predelay.len().saturating_sub(1);
        self.pd_samples = ((p.predelay_ms * 0.001 * self.sample_rate) as usize).min(max);
    }

    pub fn params(&self) -> ReverbParams {
        self.params
    }
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Reverb {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        let scale = self.sample_rate as f64 / 44_100.0;
        let len = |n: usize| ((n as f64 * scale).round() as usize).max(1);
        for (i, t) in COMB_TUNING.iter().enumerate() {
            self.comb_l[i].buf = vec![0.0; len(*t)];
            self.comb_r[i].buf = vec![0.0; len(*t + STEREO_SPREAD)];
        }
        for (i, t) in ALLPASS_TUNING.iter().enumerate() {
            self.ap_l[i].buf = vec![0.0; len(*t)];
            self.ap_r[i].buf = vec![0.0; len(*t + STEREO_SPREAD)];
        }
        self.predelay = vec![0.0; (MAX_PREDELAY_MS * 0.001 * self.sample_rate) as usize + 1];
        for s in [
            &mut self.feedback,
            &mut self.damp,
            &mut self.width,
            &mut self.mix,
        ] {
            s.set_time(20.0, self.sample_rate);
        }
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.comb_l
            .iter_mut()
            .chain(self.comb_r.iter_mut())
            .for_each(Comb::clear);
        self.ap_l
            .iter_mut()
            .chain(self.ap_r.iter_mut())
            .for_each(Allpass::clear);
        self.predelay.iter_mut().for_each(|v| *v = 0.0);
        self.pd_write = 0;
        self.feedback.snap();
        self.damp.snap();
        self.width.snap();
        self.mix.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let pd_len = self.predelay.len();
        if pd_len == 0 {
            return;
        }
        let n = left.len().min(right.len());
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let (xl, xr) = (sanitize(*l), sanitize(*r));
            let fb = self.feedback.next_value();
            let damp = self.damp.next_value();
            let width = self.width.next_value();
            let mix = self.mix.next_value();

            // Pré-delay da soma mono (como no Freeverb, a entrada dos pentes é mono).
            self.predelay[self.pd_write] = (xl + xr) * FIXED_GAIN;
            let read = (self.pd_write + pd_len - self.pd_samples) % pd_len;
            let input = self.predelay[read];
            self.pd_write += 1;
            if self.pd_write >= pd_len {
                self.pd_write = 0;
            }

            let mut out_l = 0.0;
            let mut out_r = 0.0;
            for c in &mut self.comb_l {
                out_l += c.tick(input, fb, damp);
            }
            for c in &mut self.comb_r {
                out_r += c.tick(input, fb, damp);
            }
            for a in &mut self.ap_l {
                out_l = a.tick(out_l);
            }
            for a in &mut self.ap_r {
                out_r = a.tick(out_r);
            }
            let wet1 = SCALE_WET * (width / 2.0 + 0.5);
            let wet2 = SCALE_WET * ((1.0 - width) / 2.0);
            let wl = out_l * wet1 + out_r * wet2;
            let wr = out_r * wet1 + out_l * wet2;
            *l = xl * (1.0 - mix) + wl * mix;
            *r = xr * (1.0 - mix) + wr * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{impulse, peak, rms};

    const SR: f32 = 48_000.0;

    fn reverb(p: ReverbParams) -> Reverb {
        let mut r = Reverb::new();
        r.prepare(SR, 128);
        r.set_params(p);
        r.reset();
        r
    }

    #[test]
    fn impulse_tail_decays_and_is_finite() {
        let mut rv = reverb(ReverbParams {
            room_size: 0.8,
            mix: 1.0,
            predelay_ms: 0.0,
            ..Default::default()
        });
        let len = 5 * 48_000;
        let mut l = impulse(len, 0, 1.0);
        let mut r = l.clone();
        crate::signals::process_in_blocks(&mut rv, &mut l, &mut r, 128);
        assert!(l.iter().chain(r.iter()).all(|v| v.is_finite()));
        let early = rms(&l[2_400..24_000]);
        let late = rms(&l[4 * 48_000..]);
        assert!(early > 1e-3, "há cauda: {early}");
        assert!(late < early * 0.01, "cauda decai: {early} → {late}");
    }

    #[test]
    fn predelay_shifts_onset() {
        let mut rv = reverb(ReverbParams {
            predelay_ms: 50.0,
            mix: 1.0,
            ..Default::default()
        });
        let mut l = impulse(9_600, 0, 1.0);
        let mut r = l.clone();
        rv.process(&mut l, &mut r);
        // 50 ms = 2400 amostras + menor pente (1116·48/44,1 ≈ 1215).
        assert!(peak(&l[..2_400 + 1_200]) == 0.0);
        assert!(peak(&l[2_400 + 1_200..]) > 0.0);
    }

    #[test]
    fn zero_mix_is_dry_and_width_zero_is_mono() {
        let mut rv = reverb(ReverbParams {
            mix: 0.0,
            ..Default::default()
        });
        let x = crate::signals::noise(4_800, 0.5, 9);
        let mut l = x.clone();
        let mut r = x.clone();
        rv.process(&mut l, &mut r);
        assert_eq!(l, x);

        let mut mono = reverb(ReverbParams {
            width: 0.0,
            mix: 1.0,
            ..Default::default()
        });
        let mut l = impulse(9_600, 0, 1.0);
        let mut r = l.clone();
        mono.process(&mut l, &mut r);
        assert!(l.iter().zip(&r).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn max_room_with_loud_noise_stays_bounded() {
        let mut rv = reverb(ReverbParams {
            room_size: 1.0,
            damping: 0.0,
            mix: 1.0,
            ..Default::default()
        });
        let mut l = crate::signals::noise(10 * 48_000, 1.0, 5);
        let mut r = crate::signals::noise(10 * 48_000, 1.0, 6);
        crate::signals::process_in_blocks(&mut rv, &mut l, &mut r, 256);
        let p = peak(&l).max(peak(&r));
        assert!(p.is_finite() && p < 20.0, "pico {p}");
    }
}
