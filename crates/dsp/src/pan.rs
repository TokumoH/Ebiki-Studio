//! Lei de panorâmica (potência constante, −3 dB no centro), coerente com `shared_kernel::Pan`.

use crate::smoothing::GainSmoother;
use crate::Processor;
use shared_kernel::Pan;

/// Lei de panorâmica usada pela engine. Delegada a [`Pan::gains`] para garantir uma única fonte
/// da verdade entre domínio (Mixing) e DSP.
#[derive(Debug, Clone, Copy, Default)]
pub struct PanLaw;

impl PanLaw {
    /// Ganhos (esquerdo, direito) para `pan` em −1..+1 (NaN → centro).
    #[inline]
    pub fn gains(pan: f32) -> (f32, f32) {
        Pan(pan).gains()
    }
}

/// Panner estéreo com ganho de canal: aplica `gain * PanLaw::gains(pan)` com rampa linear.
///
/// Para sinais mono duplicados em L/R, o resultado é exatamente a lei de potência constante.
#[derive(Debug, Clone)]
pub struct Panner {
    left: GainSmoother,
    right: GainSmoother,
    sample_rate: f32,
}

impl Panner {
    pub fn new() -> Self {
        let (l, r) = PanLaw::gains(0.0);
        Self {
            left: GainSmoother::new(l),
            right: GainSmoother::new(r),
            sample_rate: 48_000.0,
        }
    }

    /// Define panorâmica e ganho linear (sem alocação; suavizado).
    pub fn set_params(&mut self, pan: f32, gain: f32) {
        let (l, r) = PanLaw::gains(pan);
        let g = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
        self.left.set_target(l * g);
        self.right.set_target(r * g);
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}

impl Default for Panner {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Panner {
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        self.sample_rate = crate::util::safe_sr(sample_rate);
        self.left.prepare(self.sample_rate, max_block);
        self.right.prepare(self.sample_rate, max_block);
    }

    fn reset(&mut self) {
        self.left.snap();
        self.right.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            *l *= self.left.next_value();
            *r *= self.right.next_value();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn center_is_minus_3db() {
        let (l, r) = PanLaw::gains(0.0);
        assert!((crate::util::lin_to_db(l) + 3.0103).abs() < 1e-3);
        assert_eq!(l, r);
        assert_eq!(PanLaw::gains(f32::NAN), PanLaw::gains(0.0));
    }

    #[test]
    fn panner_hard_left_silences_right() {
        let mut p = Panner::new();
        p.prepare(48_000.0, 64);
        p.set_params(-1.0, 1.0);
        p.reset();
        let mut l = [1.0f32; 16];
        let mut r = [1.0f32; 16];
        p.process(&mut l, &mut r);
        assert!(l.iter().all(|v| (*v - 1.0).abs() < 1e-6));
        assert!(r.iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn panner_change_is_ramped() {
        let mut p = Panner::new();
        p.prepare(48_000.0, 64);
        p.set_params(-1.0, 1.0);
        let mut l = vec![1.0f32; 2_000];
        let mut r = vec![1.0f32; 2_000];
        p.process(&mut l, &mut r);
        let max_jump = r
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(max_jump < 0.01, "salto máximo {max_jump}");
        assert!(r[1_999].abs() < 1e-6);
    }

    proptest! {
        #[test]
        fn pan_law_matches_shared_kernel(p in -1.0f32..=1.0) {
            prop_assert_eq!(PanLaw::gains(p), Pan(p).gains());
        }
    }
}
