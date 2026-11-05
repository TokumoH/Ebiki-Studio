//! Filtros biquad (RBJ Audio EQ Cookbook) e equalizador paramétrico de até 8 bandas.
//!
//! Coeficientes e estados em `f64` (precisão em frequências graves); estrutura Transposta
//! Direta II (TDF-II), estável numericamente e barata.

use crate::smoothing::{GainSmoother, ParamSmoother};
use crate::util::{clamp_param, flush64, safe_sr, sanitize};
use crate::Processor;
use std::f64::consts::PI;

/// Tipo de resposta do filtro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterKind {
    HighPass,
    LowPass,
    #[default]
    Peak,
    LowShelf,
    HighShelf,
    /// Passa-faixa de ganho de pico 0 dB (usado na detecção do de-esser).
    BandPass,
}

/// Coeficientes normalizados (a0 = 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl BiquadCoeffs {
    /// Filtro identidade (passa tudo sem alteração).
    pub const IDENTITY: BiquadCoeffs = BiquadCoeffs {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    /// Projeta pelo RBJ Cookbook. Parâmetros fora de faixa são limitados:
    /// `freq` em [10 Hz, 0,49·fs], `q` em [0,05, 40], `gain_db` em [−48, +48].
    pub fn design(kind: FilterKind, freq_hz: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let sr = safe_sr(sample_rate) as f64;
        let f = clamp_param(freq_hz, 10.0, 1.0e6, 1_000.0) as f64;
        let f = f.min(0.49 * sr);
        let q = clamp_param(q, 0.05, 40.0, 0.707) as f64;
        let g = clamp_param(gain_db, -48.0, 48.0, 0.0) as f64;
        let a = 10f64.powf(g / 40.0);
        let w0 = 2.0 * PI * f / sr;
        let (sn, cs) = w0.sin_cos();
        let alpha = sn / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            FilterKind::LowPass => (
                (1.0 - cs) / 2.0,
                1.0 - cs,
                (1.0 - cs) / 2.0,
                1.0 + alpha,
                -2.0 * cs,
                1.0 - alpha,
            ),
            FilterKind::HighPass => (
                (1.0 + cs) / 2.0,
                -(1.0 + cs),
                (1.0 + cs) / 2.0,
                1.0 + alpha,
                -2.0 * cs,
                1.0 - alpha,
            ),
            FilterKind::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cs, 1.0 - alpha),
            FilterKind::Peak => (
                1.0 + alpha * a,
                -2.0 * cs,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cs,
                1.0 - alpha / a,
            ),
            FilterKind::LowShelf => {
                let sa = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cs + sa),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cs),
                    a * ((a + 1.0) - (a - 1.0) * cs - sa),
                    (a + 1.0) + (a - 1.0) * cs + sa,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cs),
                    (a + 1.0) + (a - 1.0) * cs - sa,
                )
            }
            FilterKind::HighShelf => {
                let sa = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cs + sa),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cs),
                    a * ((a + 1.0) + (a - 1.0) * cs - sa),
                    (a + 1.0) - (a - 1.0) * cs + sa,
                    2.0 * ((a - 1.0) - (a + 1.0) * cs),
                    (a + 1.0) - (a - 1.0) * cs - sa,
                )
            }
        };
        BiquadCoeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Magnitude analítica da resposta em `freq_hz`, em dB.
    pub fn magnitude_db(&self, freq_hz: f32, sample_rate: f32) -> f32 {
        let w = 2.0 * PI * freq_hz as f64 / sample_rate as f64;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = -(self.b1 * s1 + self.b2 * s2);
        let dr = 1.0 + self.a1 * c1 + self.a2 * c2;
        let di = -(self.a1 * s1 + self.a2 * s2);
        let mag2 = (nr * nr + ni * ni) / (dr * dr + di * di);
        (10.0 * mag2.log10()) as f32
    }
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Estado de um biquad de um canal (TDF-II).
#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    s1: f64,
    s2: f64,
}

impl BiquadState {
    #[inline]
    pub fn tick(&mut self, c: &BiquadCoeffs, x: f64) -> f64 {
        let y = c.b0 * x + self.s1;
        self.s1 = c.b1 * x - c.a1 * y + self.s2;
        self.s2 = c.b2 * x - c.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    /// Zera estados desnormalizados (chamar uma vez por bloco).
    pub fn flush(&mut self) {
        self.s1 = flush64(self.s1);
        self.s2 = flush64(self.s2);
        if !self.s1.is_finite() || !self.s2.is_finite() {
            self.reset();
        }
    }
}

/// Biquad estéreo com coeficientes fixos (sem suavização). Para mudanças em tempo real use [`Eq`].
#[derive(Debug, Clone, Default)]
pub struct Biquad {
    coeffs: BiquadCoeffs,
    l: BiquadState,
    r: BiquadState,
}

impl Biquad {
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            ..Default::default()
        }
    }

    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    pub fn coeffs(&self) -> BiquadCoeffs {
        self.coeffs
    }
}

impl Processor for Biquad {
    fn prepare(&mut self, _sample_rate: f32, _max_block: usize) {
        self.reset();
    }

    fn reset(&mut self) {
        self.l.reset();
        self.r.reset();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let c = self.coeffs;
        for x in left.iter_mut() {
            *x = self.l.tick(&c, sanitize(*x) as f64) as f32;
        }
        for x in right.iter_mut() {
            *x = self.r.tick(&c, sanitize(*x) as f64) as f32;
        }
        self.l.flush();
        self.r.flush();
    }
}

/// Número máximo de bandas do equalizador.
pub const MAX_EQ_BANDS: usize = 8;

/// Parâmetros de uma banda do EQ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBandParams {
    pub kind: FilterKind,
    pub enabled: bool,
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl Default for EqBandParams {
    fn default() -> Self {
        Self {
            kind: FilterKind::Peak,
            enabled: false,
            freq_hz: 1_000.0,
            gain_db: 0.0,
            q: 0.707,
        }
    }
}

impl EqBandParams {
    /// Limita às faixas seguras (mesmas do contexto Mixing).
    pub fn clamped(self) -> Self {
        Self {
            freq_hz: clamp_param(self.freq_hz, 20.0, 20_000.0, 1_000.0),
            gain_db: clamp_param(self.gain_db, -24.0, 24.0, 0.0),
            q: clamp_param(self.q, 0.1, 18.0, 0.707),
            ..self
        }
    }
}

/// Amostras entre recálculos de coeficientes enquanto um parâmetro está sendo suavizado.
const EQ_SUBBLOCK: usize = 16;

#[derive(Debug, Clone)]
struct EqBand {
    params: EqBandParams,
    /// log2 da frequência (suavização perceptualmente uniforme).
    log_freq: ParamSmoother,
    gain_db: ParamSmoother,
    q: ParamSmoother,
    /// Mistura 0..1 entre seco e filtrado: liga/desliga a banda sem estalo.
    wet: GainSmoother,
    coeffs: BiquadCoeffs,
    l: BiquadState,
    r: BiquadState,
}

impl EqBand {
    fn new() -> Self {
        let p = EqBandParams::default();
        Self {
            params: p,
            log_freq: ParamSmoother::new(p.freq_hz.log2()),
            gain_db: ParamSmoother::new(p.gain_db),
            q: ParamSmoother::new(p.q),
            wet: GainSmoother::new(0.0),
            coeffs: BiquadCoeffs::IDENTITY,
            l: BiquadState::default(),
            r: BiquadState::default(),
        }
    }

    fn update_coeffs(&mut self, sr: f32) {
        self.coeffs = BiquadCoeffs::design(
            self.params.kind,
            self.log_freq.current().exp2(),
            self.q.current(),
            self.gain_db.current(),
            sr,
        );
    }

    fn smoothing(&self) -> bool {
        !(self.log_freq.is_settled() && self.gain_db.is_settled() && self.q.is_settled())
    }
}

/// Equalizador paramétrico estéreo de até [`MAX_EQ_BANDS`] bandas em série.
///
/// Mudanças de frequência/ganho/Q são suavizadas (≈ 10 ms) com recálculo de coeficientes a cada
/// 16 amostras; ligar/desligar banda faz *crossfade* linear de 10 ms.
#[derive(Debug, Clone)]
pub struct Eq {
    bands: [EqBand; MAX_EQ_BANDS],
    sample_rate: f32,
}

impl Eq {
    pub fn new() -> Self {
        let mut eq = Self {
            bands: std::array::from_fn(|_| EqBand::new()),
            sample_rate: 48_000.0,
        };
        eq.prepare(48_000.0, 0);
        eq
    }

    /// Define uma banda (índice ≥ 8 é ignorado). Sem alocação.
    pub fn set_band(&mut self, index: usize, params: EqBandParams) {
        let sr = self.sample_rate;
        let Some(b) = self.bands.get_mut(index) else {
            return;
        };
        let p = params.clamped();
        let kind_changed = p.kind != b.params.kind;
        b.params = p;
        b.log_freq.set_target(p.freq_hz.log2());
        b.gain_db.set_target(p.gain_db);
        b.q.set_target(p.q);
        b.wet.set_target(if p.enabled { 1.0 } else { 0.0 });
        if kind_changed {
            b.update_coeffs(sr);
        }
    }

    /// Define todas as bandas de uma vez; bandas além de `bands.len()` são desligadas.
    pub fn set_bands(&mut self, bands: &[EqBandParams]) {
        for i in 0..MAX_EQ_BANDS {
            match bands.get(i) {
                Some(p) => self.set_band(i, *p),
                None => {
                    let mut p = self.bands[i].params;
                    p.enabled = false;
                    self.set_band(i, p);
                }
            }
        }
    }

    pub fn band(&self, index: usize) -> Option<EqBandParams> {
        self.bands.get(index).map(|b| b.params)
    }

    /// Resposta analítica (com os parâmetros-alvo) em `freq_hz`, em dB.
    pub fn magnitude_db(&self, freq_hz: f32) -> f32 {
        self.bands
            .iter()
            .filter(|b| b.params.enabled)
            .map(|b| {
                let p = b.params;
                BiquadCoeffs::design(p.kind, p.freq_hz, p.q, p.gain_db, self.sample_rate)
                    .magnitude_db(freq_hz, self.sample_rate)
            })
            .sum()
    }
}

impl Default for Eq {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Eq {
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        let sr = self.sample_rate;
        for b in &mut self.bands {
            b.log_freq.set_time(10.0, sr);
            b.gain_db.set_time(10.0, sr);
            b.q.set_time(10.0, sr);
            b.wet.set_ramp_ms(10.0, sr);
            b.wet.prepare(sr, max_block);
        }
        self.reset();
    }

    fn reset(&mut self) {
        let sr = self.sample_rate;
        for b in &mut self.bands {
            b.log_freq.snap();
            b.gain_db.snap();
            b.q.snap();
            b.wet.snap();
            b.l.reset();
            b.r.reset();
            b.update_coeffs(sr);
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let sr = self.sample_rate;
        for x in left[..n].iter_mut().chain(right[..n].iter_mut()) {
            *x = sanitize(*x);
        }
        for b in &mut self.bands {
            if b.wet.current() == 0.0 && !b.wet.is_smoothing() {
                continue;
            }
            let mut start = 0;
            while start < n {
                let end = (start + EQ_SUBBLOCK).min(n);
                if b.smoothing() {
                    let len = end - start;
                    b.log_freq.advance(len);
                    b.gain_db.advance(len);
                    b.q.advance(len);
                    b.update_coeffs(sr);
                }
                let c = b.coeffs;
                let full = b.wet.current() == 1.0 && !b.wet.is_smoothing();
                for i in start..end {
                    let (xl, xr) = (left[i] as f64, right[i] as f64);
                    let yl = b.l.tick(&c, xl);
                    let yr = b.r.tick(&c, xr);
                    if full {
                        left[i] = yl as f32;
                        right[i] = yr as f32;
                    } else {
                        let w = b.wet.next_value() as f64;
                        left[i] = (xl + w * (yl - xl)) as f32;
                        right[i] = (xr + w * (yr - xr)) as f32;
                    }
                }
                start = end;
            }
            b.l.flush();
            b.r.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{peak_db, sine};

    const SR: f32 = 48_000.0;

    /// Mede o ganho em regime de um seno passando pelo processador (descarta o transitório).
    fn measure_gain_db(p: &mut dyn Processor, freq: f32) -> f32 {
        let len = 48_000;
        let mut l = sine(freq, 0.25, SR, len, 0.0);
        let mut r = l.clone();
        crate::signals::process_in_blocks(p, &mut l, &mut r, 128);
        peak_db(&l[len / 2..]) - crate::util::lin_to_db(0.25)
    }

    fn eq_with(bands: &[EqBandParams]) -> Eq {
        let mut eq = Eq::new();
        eq.prepare(SR, 128);
        eq.set_bands(bands);
        eq.reset();
        eq
    }

    #[test]
    fn peak_plus_6db_measures_6db_at_center() {
        let mut eq = eq_with(&[EqBandParams {
            kind: FilterKind::Peak,
            enabled: true,
            freq_hz: 1_000.0,
            gain_db: 6.0,
            q: 1.0,
        }]);
        let g = measure_gain_db(&mut eq, 1_000.0);
        assert!((g - 6.0).abs() < 0.5, "ganho medido {g} dB");
        // Longe do centro, praticamente inalterado.
        let g_far = measure_gain_db(&mut eq, 100.0);
        assert!(g_far.abs() < 0.5, "100 Hz: {g_far} dB");
        assert!((eq.magnitude_db(1_000.0) - 6.0).abs() < 0.01);
    }

    #[test]
    fn highpass_kills_30hz_and_passes_1khz() {
        let mut eq = eq_with(&[EqBandParams {
            kind: FilterKind::HighPass,
            enabled: true,
            freq_hz: 120.0,
            gain_db: 0.0,
            q: 0.707,
        }]);
        let low = measure_gain_db(&mut eq, 30.0);
        eq.reset();
        let mid = measure_gain_db(&mut eq, 1_000.0);
        // 2ª ordem: −12 dB/oitava → 2 oitavas abaixo ≈ −24 dB.
        assert!(low < -20.0, "30 Hz: {low} dB");
        assert!(mid.abs() < 0.2, "1 kHz: {mid} dB");
    }

    #[test]
    fn lowpass_and_shelves_have_expected_response() {
        let lp = BiquadCoeffs::design(FilterKind::LowPass, 1_000.0, 0.707, 0.0, SR);
        assert!((lp.magnitude_db(1_000.0, SR) + 3.01).abs() < 0.05);
        assert!(lp.magnitude_db(8_000.0, SR) < -30.0);
        let ls = BiquadCoeffs::design(FilterKind::LowShelf, 200.0, 0.707, 6.0, SR);
        assert!((ls.magnitude_db(20.0, SR) - 6.0).abs() < 0.2);
        assert!(ls.magnitude_db(5_000.0, SR).abs() < 0.2);
        let hs = BiquadCoeffs::design(FilterKind::HighShelf, 5_000.0, 0.707, -6.0, SR);
        assert!((hs.magnitude_db(20_000.0, SR) + 6.0).abs() < 0.3);
        assert!(hs.magnitude_db(100.0, SR).abs() < 0.1);
        let bp = BiquadCoeffs::design(FilterKind::BandPass, 6_000.0, 2.0, 0.0, SR);
        assert!(bp.magnitude_db(6_000.0, SR).abs() < 0.01);
        assert!(bp.magnitude_db(500.0, SR) < -20.0);
    }

    #[test]
    fn biquad_processor_matches_analytic_response() {
        let mut bq = Biquad::new(BiquadCoeffs::design(
            FilterKind::Peak,
            2_000.0,
            2.0,
            -9.0,
            SR,
        ));
        let g = measure_gain_db(&mut bq, 2_000.0);
        assert!((g + 9.0).abs() < 0.3, "{g}");
    }

    #[test]
    fn disabled_eq_is_transparent() {
        let mut eq = eq_with(&[]);
        let mut l = sine(440.0, 0.5, SR, 1_000, 0.0);
        let orig = l.clone();
        let mut r = l.clone();
        eq.process(&mut l, &mut r);
        assert_eq!(l, orig);
    }

    #[test]
    fn parameter_change_is_smoothed_without_clicks() {
        let band = EqBandParams {
            kind: FilterKind::Peak,
            enabled: true,
            freq_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        };
        let mut eq = eq_with(&[band]);
        let len = 9_600;
        let mut l = sine(1_000.0, 0.5, SR, len, 0.0);
        let mut r = l.clone();
        eq.process(&mut l[..4_800], &mut r[..4_800]);
        eq.set_band(
            0,
            EqBandParams {
                gain_db: 18.0,
                ..band
            },
        );
        eq.process(&mut l[4_800..], &mut r[4_800..]);
        // A diferença entre amostras consecutivas nunca excede muito a de um seno de 1 kHz
        // com o ganho final (+18 dB ≈ ×7,9): 2π·1000/48000·0,5·7,9 ≈ 0,52.
        let max_step = l
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(max_step < 0.6, "salto {max_step}");
        assert!(peak_db(&l[8_000..]) > peak_db(&l[..4_800]) + 17.0);
    }

    #[test]
    fn enabling_band_crossfades() {
        let mut eq = eq_with(&[]);
        let mut l = vec![1.0f32; 2_000];
        let mut r = vec![1.0f32; 2_000];
        eq.set_band(
            0,
            EqBandParams {
                kind: FilterKind::HighPass,
                enabled: true,
                freq_hz: 1_000.0,
                gain_db: 0.0,
                q: 0.707,
            },
        );
        eq.process(&mut l, &mut r);
        assert!(l[0] > 0.99, "começa seco: {}", l[0]);
        assert!(l[1_999].abs() < 0.01, "DC removido ao fim: {}", l[1_999]);
    }

    #[test]
    fn invalid_index_and_nan_input_are_safe() {
        let mut eq = eq_with(&[EqBandParams {
            enabled: true,
            gain_db: 6.0,
            ..Default::default()
        }]);
        eq.set_band(99, EqBandParams::default());
        let mut l = [f32::NAN, 1.0, f32::INFINITY, 0.5];
        let mut r = [0.0; 4];
        eq.process(&mut l, &mut r);
        assert!(l.iter().all(|v| v.is_finite()));
    }
}
