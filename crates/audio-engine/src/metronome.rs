//! Clique de metrônomo sintetizado (sem alocação).

const CLICK_SECONDS: f32 = 0.03;

/// Gerador de clique: seno com envelope exponencial.
#[derive(Debug, Clone)]
pub struct Click {
    sample_rate: f32,
    phase: f32,
    freq: f32,
    remaining: u32,
    total: u32,
    gain: f32,
}

impl Click {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            phase: 0.0,
            freq: 1000.0,
            remaining: 0,
            total: 1,
            gain: 0.0,
        }
    }

    pub fn trigger(&mut self, accent: bool, volume: f32) {
        self.freq = if accent { 1600.0 } else { 1000.0 };
        self.total = ((self.sample_rate * CLICK_SECONDS) as u32).max(1);
        self.remaining = self.total;
        self.phase = 0.0;
        self.gain = volume.clamp(0.0, 1.0) * if accent { 1.0 } else { 0.7 };
    }

    /// Soma o clique em `out_l`/`out_r` a partir de `offset`.
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32], offset: usize) {
        let step = std::f32::consts::TAU * self.freq / self.sample_rate;
        for i in offset..out_l.len() {
            if self.remaining == 0 {
                break;
            }
            let env = self.remaining as f32 / self.total as f32;
            let s = self.phase.sin() * env * env * self.gain;
            out_l[i] += s;
            out_r[i] += s;
            self.phase += step;
            if self.phase > std::f32::consts::TAU {
                self.phase -= std::f32::consts::TAU;
            }
            self.remaining -= 1;
        }
    }

    pub fn is_active(&self) -> bool {
        self.remaining > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_is_short_and_bounded() {
        let mut c = Click::new(48_000);
        c.trigger(true, 1.0);
        let mut l = vec![0.0; 4096];
        let mut r = vec![0.0; 4096];
        c.render(&mut l, &mut r, 10);
        assert_eq!(
            l[..10].iter().filter(|x| **x != 0.0).count(),
            0,
            "nada antes do offset"
        );
        let nonzero = l.iter().filter(|x| x.abs() > 0.0).count();
        assert!(
            nonzero > 1000 && nonzero <= 1440 + 1,
            "≈30 ms a 48 kHz, obtido {nonzero}"
        );
        assert!(l.iter().all(|x| x.abs() <= 1.0));
        assert!(!c.is_active());
    }
}
