//! Efeitos nativos e seus parâmetros. Todo parâmetro é limitado (`clamped`) a uma faixa segura.

use serde::{Deserialize, Serialize};
use shared_kernel::define_id;

define_id!(
    /// Identificador de efeito inserido.
    EffectId
);

fn c(v: f32, lo: f32, hi: f32, default: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        default
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EqBandKind {
    HighPass,
    LowShelf,
    Peak,
    HighShelf,
    LowPass,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqBand {
    pub kind: EqBandKind,
    pub enabled: bool,
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl EqBand {
    pub fn clamped(self) -> Self {
        Self {
            freq_hz: c(self.freq_hz, 20.0, 20_000.0, 1_000.0),
            gain_db: c(self.gain_db, -24.0, 24.0, 0.0),
            q: c(self.q, 0.1, 18.0, 0.707),
            ..self
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqParams {
    pub bands: Vec<EqBand>,
}

pub const MAX_EQ_BANDS: usize = 8;

impl Default for EqParams {
    fn default() -> Self {
        use EqBandKind::*;
        let b = |kind, freq_hz, q| EqBand {
            kind,
            enabled: kind != HighPass,
            freq_hz,
            gain_db: 0.0,
            q,
        };
        Self {
            bands: vec![
                b(HighPass, 80.0, 0.707),
                b(LowShelf, 120.0, 0.707),
                b(Peak, 1_000.0, 1.0),
                b(HighShelf, 8_000.0, 0.707),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateParams {
    pub threshold_db: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimiterParams {
    pub ceiling_db: f32,
    pub input_gain_db: f32,
    pub release_ms: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self {
            ceiling_db: -1.0,
            input_gain_db: 0.0,
            release_ms: 80.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaturationParams {
    pub drive_db: f32,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeEsserParams {
    pub freq_hz: f32,
    pub threshold_db: f32,
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

/// Efeito com seus parâmetros.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "params", rename_all = "camelCase")]
pub enum Effect {
    Eq(EqParams),
    Compressor(CompressorParams),
    Gate(GateParams),
    Limiter(LimiterParams),
    Reverb(ReverbParams),
    Delay(DelayParams),
    Saturation(SaturationParams),
    Chorus(ChorusParams),
    DeEsser(DeEsserParams),
}

impl Effect {
    /// Retorna uma cópia com todos os parâmetros dentro das faixas seguras.
    pub fn clamped(&self) -> Effect {
        match self {
            Effect::Eq(p) => Effect::Eq(EqParams {
                bands: p
                    .bands
                    .iter()
                    .take(MAX_EQ_BANDS)
                    .map(|b| b.clamped())
                    .collect(),
            }),
            Effect::Compressor(p) => Effect::Compressor(CompressorParams {
                threshold_db: c(p.threshold_db, -60.0, 0.0, -18.0),
                ratio: c(p.ratio, 1.0, 20.0, 3.0),
                attack_ms: c(p.attack_ms, 0.1, 200.0, 10.0),
                release_ms: c(p.release_ms, 5.0, 2_000.0, 120.0),
                knee_db: c(p.knee_db, 0.0, 24.0, 6.0),
                makeup_db: c(p.makeup_db, 0.0, 24.0, 0.0),
            }),
            Effect::Gate(p) => Effect::Gate(GateParams {
                threshold_db: c(p.threshold_db, -90.0, 0.0, -50.0),
                attack_ms: c(p.attack_ms, 0.05, 50.0, 1.0),
                hold_ms: c(p.hold_ms, 0.0, 500.0, 30.0),
                release_ms: c(p.release_ms, 5.0, 2_000.0, 150.0),
                range_db: c(p.range_db, -90.0, 0.0, -60.0),
            }),
            Effect::Limiter(p) => Effect::Limiter(LimiterParams {
                ceiling_db: c(p.ceiling_db, -24.0, 0.0, -1.0),
                input_gain_db: c(p.input_gain_db, 0.0, 24.0, 0.0),
                release_ms: c(p.release_ms, 1.0, 1_000.0, 80.0),
            }),
            Effect::Reverb(p) => Effect::Reverb(ReverbParams {
                room_size: c(p.room_size, 0.0, 1.0, 0.6),
                damping: c(p.damping, 0.0, 1.0, 0.5),
                width: c(p.width, 0.0, 1.0, 1.0),
                predelay_ms: c(p.predelay_ms, 0.0, 200.0, 20.0),
                mix: c(p.mix, 0.0, 1.0, 0.25),
            }),
            Effect::Delay(p) => Effect::Delay(DelayParams {
                time_ms: c(p.time_ms, 1.0, 2_000.0, 375.0),
                feedback: c(p.feedback, 0.0, 0.95, 0.35),
                high_cut_hz: c(p.high_cut_hz, 500.0, 20_000.0, 6_000.0),
                mix: c(p.mix, 0.0, 1.0, 0.2),
                ping_pong: p.ping_pong,
            }),
            Effect::Saturation(p) => Effect::Saturation(SaturationParams {
                drive_db: c(p.drive_db, 0.0, 36.0, 6.0),
                tone: c(p.tone, 0.0, 1.0, 0.5),
                mix: c(p.mix, 0.0, 1.0, 1.0),
            }),
            Effect::Chorus(p) => Effect::Chorus(ChorusParams {
                rate_hz: c(p.rate_hz, 0.05, 10.0, 0.8),
                depth_ms: c(p.depth_ms, 0.1, 15.0, 3.0),
                mix: c(p.mix, 0.0, 1.0, 0.4),
            }),
            Effect::DeEsser(p) => Effect::DeEsser(DeEsserParams {
                freq_hz: c(p.freq_hz, 2_000.0, 12_000.0, 6_500.0),
                threshold_db: c(p.threshold_db, -60.0, 0.0, -28.0),
                reduction_db: c(p.reduction_db, -24.0, 0.0, -8.0),
            }),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Effect::Eq(_) => "Equalizador",
            Effect::Compressor(_) => "Compressor",
            Effect::Gate(_) => "Gate",
            Effect::Limiter(_) => "Limitador",
            Effect::Reverb(_) => "Reverb",
            Effect::Delay(_) => "Delay",
            Effect::Saturation(_) => "Saturação",
            Effect::Chorus(_) => "Chorus",
            Effect::DeEsser(_) => "De-esser",
        }
    }
}

/// Efeito inserido num canal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectSlot {
    pub id: EffectId,
    pub bypass: bool,
    pub effect: Effect,
}

impl EffectSlot {
    pub fn new(effect: Effect) -> Self {
        Self {
            id: EffectId::new(),
            bypass: false,
            effect: effect.clamped(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_unsafe_values_and_nan() {
        let e = Effect::Compressor(CompressorParams {
            ratio: 1000.0,
            attack_ms: f32::NAN,
            ..Default::default()
        })
        .clamped();
        let Effect::Compressor(p) = e else { panic!() };
        assert_eq!(p.ratio, 20.0);
        assert_eq!(p.attack_ms, 10.0);
        let Effect::Delay(d) = Effect::Delay(DelayParams {
            feedback: 1.5,
            ..Default::default()
        })
        .clamped() else {
            panic!()
        };
        assert!(d.feedback < 1.0, "realimentação ≥ 1 explodiria o sinal");
    }

    #[test]
    fn eq_band_count_is_limited() {
        let bands = vec![EqParams::default().bands[0]; 20];
        let Effect::Eq(p) = Effect::Eq(EqParams { bands }).clamped() else {
            panic!()
        };
        assert_eq!(p.bands.len(), MAX_EQ_BANDS);
    }

    #[test]
    fn effect_serializes_with_type_tag() {
        let json = serde_json::to_value(Effect::Gate(GateParams::default())).unwrap();
        assert_eq!(json["type"], "gate");
        assert!(json["params"]["thresholdDb"].is_number());
    }
}
