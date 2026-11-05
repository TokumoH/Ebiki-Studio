//! Compressor *feed-forward* com joelho suave, envelope de ataque/release no domínio do ganho
//! (detector "smooth decoupled peak" de Giannoulis, Massberg & Reiss, JAES 2012), *makeup* e
//! detecção estéreo vinculada (máximo dos dois canais → mesma redução nos dois).

use crate::smoothing::GainSmoother;
use crate::util::{clamp_param, db_to_lin, safe_sr, sanitize, time_coef};
use crate::Processor;

/// ln(10)/20: converte dB em expoente natural.
const DB_TO_NEPER: f32 = std::f32::consts::LN_10 / 20.0;
/// 20/ln(10): converte logaritmo natural de amplitude em dB.
const NEPER_TO_DB: f32 = 20.0 / std::f32::consts::LN_10;

/// Parâmetros do compressor (faixas iguais às do contexto Mixing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_db: f32,
    pub makeup_db: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            knee_db: 6.0,
            makeup_db: 0.0,
        }
    }
}

impl CompressorParams {
    pub fn clamped(self) -> Self {
        Self {
            threshold_db: clamp_param(self.threshold_db, -60.0, 0.0, -18.0),
            ratio: clamp_param(self.ratio, 1.0, 20.0, 3.0),
            attack_ms: clamp_param(self.attack_ms, 0.1, 200.0, 10.0),
            release_ms: clamp_param(self.release_ms, 5.0, 2_000.0, 120.0),
            knee_db: clamp_param(self.knee_db, 0.0, 24.0, 6.0),
            makeup_db: clamp_param(self.makeup_db, 0.0, 24.0, 0.0),
        }
    }
}

/// Curva estática de compressão: redução de ganho (dB, ≥ 0) para um nível de entrada `x_db`.
///
/// Joelho suave quadrático de largura `knee` centrado no limiar (Giannoulis et al., eq. 4).
#[inline]
pub fn gain_computer_db(x_db: f32, threshold: f32, ratio: f32, knee: f32) -> f32 {
    let slope = 1.0 - 1.0 / ratio;
    let over = x_db - threshold;
    if knee > 0.0 && 2.0 * over.abs() <= knee {
        let t = over + knee / 2.0;
        slope * t * t / (2.0 * knee)
    } else if over > 0.0 {
        slope * over
    } else {
        0.0
    }
}

/// Compressor estéreo vinculado.
#[derive(Debug, Clone)]
pub struct Compressor {
    params: CompressorParams,
    sample_rate: f32,
    att: f32,
    rel: f32,
    /// Estágio de release (pico com decaimento) da redução, em dB.
    y1: f32,
    /// Saída suavizada pelo ataque, em dB (≥ 0).
    gr_db: f32,
    makeup: GainSmoother,
}

impl Compressor {
    pub fn new() -> Self {
        let mut c = Self {
            params: CompressorParams::default(),
            sample_rate: 48_000.0,
            att: 0.0,
            rel: 0.0,
            y1: 0.0,
            gr_db: 0.0,
            makeup: GainSmoother::new(1.0),
        };
        c.prepare(48_000.0, 0);
        c
    }

    /// Altera parâmetros sem alocação. Limiar/razão agem sobre a redução, que já é suavizada pelo
    /// envelope; o *makeup* tem rampa linear de 20 ms.
    pub fn set_params(&mut self, p: CompressorParams) {
        self.params = p.clamped();
        self.att = time_coef(self.params.attack_ms, self.sample_rate);
        self.rel = time_coef(self.params.release_ms, self.sample_rate);
        self.makeup.set_target(db_to_lin(self.params.makeup_db));
    }

    pub fn params(&self) -> CompressorParams {
        self.params
    }

    /// Redução de ganho atual em dB (valor ≥ 0; 6,0 significa −6 dB aplicados).
    pub fn gain_reduction_db(&self) -> f32 {
        self.gr_db
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Compressor {
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        self.makeup.prepare(self.sample_rate, max_block);
        self.set_params(self.params);
        self.reset();
    }

    fn reset(&mut self) {
        self.y1 = 0.0;
        self.gr_db = 0.0;
        self.makeup.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let CompressorParams {
            threshold_db,
            ratio,
            knee_db,
            ..
        } = self.params;
        // Abaixo deste nível linear a redução é sempre 0: evita log10 em silêncio.
        let floor_lin = db_to_lin(threshold_db - knee_db / 2.0 - 1.0);
        let (att, rel) = (self.att, self.rel);
        for (l, r) in left[..n].iter_mut().zip(right[..n].iter_mut()) {
            let (xl, xr) = (sanitize(*l), sanitize(*r));
            let level = xl.abs().max(xr.abs());
            let xg = if level > floor_lin {
                gain_computer_db(NEPER_TO_DB * level.ln(), threshold_db, ratio, knee_db)
            } else {
                0.0
            };
            // Smooth decoupled peak: release no pico, depois ataque.
            self.y1 = xg.max(rel * self.y1 + (1.0 - rel) * xg);
            self.gr_db = att * self.gr_db + (1.0 - att) * self.y1;
            if self.gr_db < 1.0e-6 {
                self.gr_db = 0.0;
            }
            // 10^(−gr/20) = e^(−gr·ln10/20); ganho 1 exato quando não há redução.
            let g_red = if self.gr_db == 0.0 {
                1.0
            } else {
                (-self.gr_db * DB_TO_NEPER).exp()
            };
            let g = g_red * self.makeup.next_value();
            *l = xl * g;
            *r = xr * g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::{peak_db, process_in_blocks, sine};

    const SR: f32 = 48_000.0;

    fn comp(p: CompressorParams) -> Compressor {
        let mut c = Compressor::new();
        c.prepare(SR, 128);
        c.set_params(p);
        c.reset();
        c
    }

    #[test]
    fn static_curve_hard_and_soft_knee() {
        assert_eq!(gain_computer_db(-30.0, -20.0, 4.0, 0.0), 0.0);
        assert!((gain_computer_db(-4.0, -20.0, 4.0, 0.0) - 12.0).abs() < 1e-5);
        // No limiar com joelho de 6 dB: slope·(k/2)²/(2k) = 0,75·9/12 = 0,5625.
        assert!((gain_computer_db(-20.0, -20.0, 4.0, 6.0) - 0.5625).abs() < 1e-5);
        // Contínua nas bordas do joelho.
        let a = gain_computer_db(-17.0001, -20.0, 4.0, 6.0);
        let b = gain_computer_db(-16.9999, -20.0, 4.0, 6.0);
        assert!((a - b).abs() < 1e-3);
    }

    #[test]
    fn ratio_4_to_1_steady_state_output_level() {
        // Seno a −4 dBFS, limiar −20, 4:1, sem joelho: 16 dB acima → sai 4 dB acima = −16 dBFS.
        let mut c = comp(CompressorParams {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 200.0,
            knee_db: 0.0,
            makeup_db: 0.0,
        });
        let amp = db_to_lin(-4.0);
        let len = 96_000;
        let mut l = sine(1_000.0, amp, SR, len, 0.0);
        let mut r = l.clone();
        process_in_blocks(&mut c, &mut l, &mut r, 128);
        let out = peak_db(&l[len - 4_800..]);
        assert!((out + 16.0).abs() < 0.5, "saída {out} dBFS");
        assert!((c.gain_reduction_db() - 12.0).abs() < 0.5);
        assert_eq!(peak_db(&l[len - 4_800..]), peak_db(&r[len - 4_800..]));
    }

    #[test]
    fn below_threshold_is_untouched_and_makeup_applies() {
        let mut c = comp(CompressorParams {
            threshold_db: -10.0,
            ratio: 8.0,
            knee_db: 0.0,
            makeup_db: 6.0,
            ..Default::default()
        });
        let mut l = sine(500.0, 0.1, SR, 24_000, 0.0);
        let mut r = l.clone();
        process_in_blocks(&mut c, &mut l, &mut r, 64);
        assert_eq!(c.gain_reduction_db(), 0.0);
        let out = peak_db(&l[20_000..]);
        assert!((out - (-20.0 + 6.0)).abs() < 0.05, "{out}");
    }

    #[test]
    fn stereo_link_applies_same_gain_to_both_channels() {
        let mut c = comp(CompressorParams {
            threshold_db: -20.0,
            ratio: 10.0,
            knee_db: 0.0,
            ..Default::default()
        });
        let mut l = sine(1_000.0, 1.0, SR, 48_000, 0.0);
        let mut r = vec![0.01f32; 48_000];
        process_in_blocks(&mut c, &mut l, &mut r, 128);
        // O canal direito (baixo) recebe a mesma redução causada pelo esquerdo.
        let gr = c.gain_reduction_db();
        assert!(gr > 15.0);
        assert!((crate::util::lin_to_db(r[47_999] / 0.01) + gr).abs() < 0.5);
    }

    #[test]
    fn attack_and_release_shape_gain_reduction() {
        let mut c = comp(CompressorParams {
            threshold_db: -30.0,
            ratio: 20.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 0.0,
            makeup_db: 0.0,
        });
        let mut l = vec![0.5f32; 4_800];
        let mut r = l.clone();
        c.process(&mut l[..48], &mut r[..48]);
        let early = c.gain_reduction_db();
        c.process(&mut l[48..], &mut r[48..]);
        let full = c.gain_reduction_db();
        assert!(
            early < full * 0.2,
            "ataque não é instantâneo: {early} vs {full}"
        );
        let mut z = vec![0.0f32; 4_800];
        let mut z2 = z.clone();
        c.process(&mut z, &mut z2);
        let after = c.gain_reduction_db();
        assert!(
            after < full * 0.5 && after > 0.0,
            "release gradual: {after}"
        );
    }

    #[test]
    fn nan_params_fall_back_to_defaults() {
        let mut c = comp(CompressorParams {
            ratio: f32::NAN,
            attack_ms: -1.0,
            ..Default::default()
        });
        assert_eq!(c.params().ratio, 3.0);
        assert_eq!(c.params().attack_ms, 0.1);
        let mut l = [f32::NAN; 8];
        let mut r = [1.0e6f32; 8];
        c.process(&mut l, &mut r);
        assert!(l.iter().chain(r.iter()).all(|v| v.is_finite()));
    }
}
