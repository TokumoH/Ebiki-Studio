//! De-esser de banda dividida.
//!
//! - **Detecção:** passa-faixa (RBJ, Q = 1,5) centrado em `freq_hz` sobre a soma mono; envelope
//!   de pico (ataque 1 ms, release 60 ms).
//! - **Redução:** *shelf* agudo dinâmico (RBJ, ponto médio uma oitava abaixo do centro) cujo
//!   ganho é o excesso do envelope acima do limiar, limitado a `reduction_db`. Os coeficientes são
//!   recalculados a cada 16 amostras apenas quando a redução muda (> 0,01 dB); com redução 0 o
//!   filtro é a identidade exata (sinal intocado).
//!
//! Alternativa descartada: divisão complementar `agudo = x − grave` com filtro de um polo — a
//! rotação de fase entre as bandas reduzia só ~4 dB dos 8 dB pedidos no centro da sibilância.

use crate::biquad::{BiquadCoeffs, BiquadState, FilterKind};
use crate::util::{clamp_param, db_to_lin, flush, safe_sr, sanitize, time_coef};
use crate::Processor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeEsserParams {
    pub freq_hz: f32,
    pub threshold_db: f32,
    /// Redução máxima (negativa, ex.: −8 dB).
    pub reduction_db: f32,
}

impl Default for DeEsserParams {
    fn default() -> Self {
        Self {
            freq_hz: 6_500.0,
            threshold_db: -28.0,
            reduction_db: -8.0,
        }
    }
}

impl DeEsserParams {
    pub fn clamped(self) -> Self {
        Self {
            freq_hz: clamp_param(self.freq_hz, 2_000.0, 12_000.0, 6_500.0),
            threshold_db: clamp_param(self.threshold_db, -60.0, 0.0, -28.0),
            reduction_db: clamp_param(self.reduction_db, -24.0, 0.0, -8.0),
        }
    }
}

const DETECT_Q: f32 = 1.5;
/// O *shelf* de redução fica uma oitava abaixo do centro de detecção.
const SHELF_RATIO: f32 = 0.5;
const SHELF_Q: f32 = 0.707;
const SUBBLOCK: usize = 16;

#[derive(Debug, Clone)]
pub struct DeEsser {
    params: DeEsserParams,
    sample_rate: f32,
    detect: BiquadCoeffs,
    det_state: BiquadState,
    shelf: BiquadCoeffs,
    shelf_gr_db: f32,
    shelf_l: BiquadState,
    shelf_r: BiquadState,
    env: f32,
    att: f32,
    rel: f32,
    thr_lin: f32,
    gr_db: f32,
}

impl DeEsser {
    pub fn new() -> Self {
        let mut d = Self {
            params: DeEsserParams::default(),
            sample_rate: 48_000.0,
            detect: BiquadCoeffs::IDENTITY,
            det_state: BiquadState::default(),
            shelf: BiquadCoeffs::IDENTITY,
            shelf_gr_db: 0.0,
            shelf_l: BiquadState::default(),
            shelf_r: BiquadState::default(),
            env: 0.0,
            att: 0.0,
            rel: 0.0,
            thr_lin: 1.0,
            gr_db: 0.0,
        };
        d.prepare(48_000.0, 0);
        d
    }

    pub fn set_params(&mut self, p: DeEsserParams) {
        let p = p.clamped();
        self.params = p;
        let sr = self.sample_rate;
        self.detect = BiquadCoeffs::design(FilterKind::BandPass, p.freq_hz, DETECT_Q, 0.0, sr);
        self.update_shelf(self.shelf_gr_db, true);
        self.thr_lin = db_to_lin(p.threshold_db);
    }

    pub fn params(&self) -> DeEsserParams {
        self.params
    }

    fn update_shelf(&mut self, gr_db: f32, force: bool) {
        if !force && (gr_db - self.shelf_gr_db).abs() < 0.01 {
            return;
        }
        self.shelf_gr_db = gr_db;
        self.shelf = if gr_db <= 0.0 {
            BiquadCoeffs::IDENTITY
        } else {
            BiquadCoeffs::design(
                FilterKind::HighShelf,
                self.params.freq_hz * SHELF_RATIO,
                SHELF_Q,
                -gr_db,
                self.sample_rate,
            )
        };
    }

    /// Redução atual aplicada à banda aguda, em dB (≥ 0).
    pub fn gain_reduction_db(&self) -> f32 {
        self.gr_db
    }
}

impl Default for DeEsser {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for DeEsser {
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        self.att = time_coef(1.0, self.sample_rate);
        self.rel = time_coef(60.0, self.sample_rate);
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.det_state.reset();
        self.shelf_l.reset();
        self.shelf_r.reset();
        self.env = 0.0;
        self.gr_db = 0.0;
        self.update_shelf(0.0, true);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let max_red = -self.params.reduction_db;
        let thr_db = self.params.threshold_db;
        let mut start = 0;
        while start < n {
            let end = (start + SUBBLOCK).min(n);
            self.update_shelf(self.gr_db, false);
            let c = self.shelf;
            for i in start..end {
                let (xl, xr) = (sanitize(left[i]), sanitize(right[i]));
                let band = self
                    .det_state
                    .tick(&self.detect, 0.5 * (xl + xr) as f64)
                    .abs() as f32;
                let coef = if band > self.env { self.att } else { self.rel };
                self.env = flush(coef * self.env + (1.0 - coef) * band);
                left[i] = self.shelf_l.tick(&c, xl as f64) as f32;
                right[i] = self.shelf_r.tick(&c, xr as f64) as f32;
            }
            self.gr_db = if self.env > self.thr_lin {
                (20.0 * self.env.log10() - thr_db).min(max_red)
            } else {
                0.0
            };
            start = end;
        }
        self.det_state.flush();
        self.shelf_l.flush();
        self.shelf_r.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{peak_db, process_in_blocks, sine};

    const SR: f32 = 48_000.0;

    fn deesser(p: DeEsserParams) -> DeEsser {
        let mut d = DeEsser::new();
        d.prepare(SR, 128);
        d.set_params(p);
        d.reset();
        d
    }

    fn run(d: &mut DeEsser, freq: f32, amp: f32) -> f32 {
        let mut l = sine(freq, amp, SR, 24_000, 0.0);
        let mut r = l.clone();
        process_in_blocks(d, &mut l, &mut r, 128);
        peak_db(&l[12_000..]) - peak_db(&[amp])
    }

    #[test]
    fn loud_sibilance_is_reduced_up_to_max() {
        let mut d = deesser(DeEsserParams {
            freq_hz: 6_500.0,
            threshold_db: -30.0,
            reduction_db: -8.0,
        });
        let change = run(&mut d, 6_500.0, 0.5);
        // Shelf com ponto médio em 3,25 kHz: −7,56 dB analíticos em 6,5 kHz.
        assert!((change + 7.56).abs() < 0.3, "redução {change} dB");
        assert!((d.gain_reduction_db() - 8.0).abs() < 0.01);
    }

    #[test]
    fn quiet_sibilance_and_low_frequencies_pass() {
        let mut d = deesser(DeEsserParams {
            threshold_db: -20.0,
            ..Default::default()
        });
        let change = run(&mut d, 6_500.0, 0.01);
        assert!(change.abs() < 0.05, "abaixo do limiar: {change}");
        d.reset();
        // Grave forte: fora da banda de detecção → praticamente sem redução.
        let change = run(&mut d, 150.0, 0.8);
        assert!(change.abs() < 0.3, "grave: {change}");
    }

    #[test]
    fn below_threshold_is_bit_transparent() {
        let mut d = deesser(DeEsserParams {
            threshold_db: 0.0,
            ..Default::default()
        });
        let x = crate::signals::noise(4_800, 0.5, 4);
        let mut l = x.clone();
        let mut r = x.clone();
        d.process(&mut l, &mut r);
        assert!(l.iter().zip(&x).all(|(a, b)| (a - b).abs() < 1e-6));
    }
}
