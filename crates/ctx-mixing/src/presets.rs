//! Presets de cadeia de efeitos por instrumento (foco em voz, violão e guitarra).

use crate::effects::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstrumentPreset {
    LeadVocal,
    BackingVocal,
    Rap,
    AcousticGuitar,
    ElectricGuitarClean,
    ElectricGuitarDistorted,
    Bass,
    Drums,
    Piano,
    MasterBus,
}

impl InstrumentPreset {
    pub const ALL: [InstrumentPreset; 10] = [
        Self::LeadVocal,
        Self::BackingVocal,
        Self::Rap,
        Self::AcousticGuitar,
        Self::ElectricGuitarClean,
        Self::ElectricGuitarDistorted,
        Self::Bass,
        Self::Drums,
        Self::Piano,
        Self::MasterBus,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::LeadVocal => "Voz principal",
            Self::BackingVocal => "Backing vocal",
            Self::Rap => "Voz falada / rap",
            Self::AcousticGuitar => "Violão",
            Self::ElectricGuitarClean => "Guitarra limpa",
            Self::ElectricGuitarDistorted => "Guitarra distorcida",
            Self::Bass => "Baixo",
            Self::Drums => "Bateria",
            Self::Piano => "Piano / teclado",
            Self::MasterBus => "Master",
        }
    }

    /// Cadeia de efeitos recomendada (ponto de partida; o usuário ajusta).
    pub fn chain(self) -> Vec<EffectSlot> {
        use EqBandKind::*;
        let band = |kind, freq_hz, gain_db, q| EqBand {
            kind,
            enabled: true,
            freq_hz,
            gain_db,
            q,
        };
        let eq = |bands: Vec<EqBand>| Effect::Eq(EqParams { bands });
        let comp = |threshold_db, ratio, attack_ms, release_ms, makeup_db| {
            Effect::Compressor(CompressorParams {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db: 6.0,
                makeup_db,
            })
        };
        let effects: Vec<Effect> = match self {
            Self::LeadVocal => vec![
                eq(vec![
                    band(HighPass, 90.0, 0.0, 0.707),
                    band(Peak, 300.0, -2.5, 1.2),
                    band(Peak, 3_000.0, 2.0, 0.9),
                    band(HighShelf, 10_000.0, 2.5, 0.707),
                ]),
                comp(-20.0, 3.5, 5.0, 80.0, 4.0),
                Effect::DeEsser(DeEsserParams::default()),
                Effect::Delay(DelayParams {
                    time_ms: 250.0,
                    feedback: 0.2,
                    high_cut_hz: 5_000.0,
                    mix: 0.08,
                    ping_pong: false,
                }),
                Effect::Reverb(ReverbParams {
                    room_size: 0.55,
                    damping: 0.6,
                    width: 0.9,
                    predelay_ms: 30.0,
                    mix: 0.15,
                }),
            ],
            Self::BackingVocal => vec![
                eq(vec![
                    band(HighPass, 150.0, 0.0, 0.707),
                    band(Peak, 400.0, -3.0, 1.0),
                    band(HighShelf, 9_000.0, 1.5, 0.707),
                ]),
                comp(-24.0, 4.0, 8.0, 100.0, 3.0),
                Effect::DeEsser(DeEsserParams {
                    reduction_db: -10.0,
                    ..Default::default()
                }),
                Effect::Chorus(ChorusParams {
                    rate_hz: 0.5,
                    depth_ms: 2.0,
                    mix: 0.2,
                }),
                Effect::Reverb(ReverbParams {
                    mix: 0.25,
                    ..Default::default()
                }),
            ],
            Self::Rap => vec![
                eq(vec![
                    band(HighPass, 80.0, 0.0, 0.707),
                    band(Peak, 250.0, -2.0, 1.0),
                    band(Peak, 4_000.0, 2.5, 1.0),
                ]),
                comp(-18.0, 4.0, 3.0, 60.0, 4.0),
                Effect::DeEsser(DeEsserParams::default()),
                Effect::Saturation(SaturationParams {
                    drive_db: 3.0,
                    tone: 0.55,
                    mix: 0.4,
                }),
            ],
            Self::AcousticGuitar => vec![
                eq(vec![
                    band(HighPass, 80.0, 0.0, 0.707),
                    band(Peak, 200.0, -3.0, 1.0),
                    band(Peak, 5_000.0, 1.5, 0.8),
                    band(HighShelf, 12_000.0, 2.0, 0.707),
                ]),
                comp(-20.0, 2.5, 15.0, 150.0, 2.0),
                Effect::Reverb(ReverbParams {
                    room_size: 0.45,
                    mix: 0.18,
                    ..Default::default()
                }),
            ],
            Self::ElectricGuitarClean => vec![
                eq(vec![
                    band(HighPass, 90.0, 0.0, 0.707),
                    band(Peak, 2_500.0, 1.5, 1.0),
                    band(LowPass, 12_000.0, 0.0, 0.707),
                ]),
                comp(-22.0, 3.0, 10.0, 120.0, 2.0),
                Effect::Chorus(ChorusParams::default()),
                Effect::Delay(DelayParams {
                    mix: 0.15,
                    ..Default::default()
                }),
                Effect::Reverb(ReverbParams {
                    mix: 0.2,
                    ..Default::default()
                }),
            ],
            Self::ElectricGuitarDistorted => vec![
                Effect::Gate(GateParams {
                    threshold_db: -45.0,
                    ..Default::default()
                }),
                eq(vec![
                    band(HighPass, 100.0, 0.0, 0.707),
                    band(Peak, 500.0, -2.0, 1.2),
                    band(LowPass, 9_000.0, 0.0, 0.707),
                ]),
                Effect::Saturation(SaturationParams {
                    drive_db: 18.0,
                    tone: 0.45,
                    mix: 1.0,
                }),
                comp(-16.0, 2.0, 20.0, 100.0, 0.0),
            ],
            Self::Bass => vec![
                eq(vec![
                    band(HighPass, 35.0, 0.0, 0.707),
                    band(LowShelf, 80.0, 2.0, 0.707),
                    band(Peak, 250.0, -2.5, 1.0),
                    band(LowPass, 6_000.0, 0.0, 0.707),
                ]),
                comp(-20.0, 4.0, 20.0, 150.0, 3.0),
                Effect::Saturation(SaturationParams {
                    drive_db: 4.0,
                    tone: 0.4,
                    mix: 0.3,
                }),
            ],
            Self::Drums => vec![
                eq(vec![
                    band(HighPass, 30.0, 0.0, 0.707),
                    band(Peak, 400.0, -3.0, 1.0),
                    band(HighShelf, 8_000.0, 2.0, 0.707),
                ]),
                comp(-18.0, 4.0, 20.0, 100.0, 3.0),
            ],
            Self::Piano => vec![
                eq(vec![
                    band(HighPass, 40.0, 0.0, 0.707),
                    band(Peak, 300.0, -1.5, 1.0),
                    band(HighShelf, 10_000.0, 1.5, 0.707),
                ]),
                comp(-22.0, 2.0, 20.0, 200.0, 1.5),
                Effect::Reverb(ReverbParams {
                    room_size: 0.7,
                    mix: 0.2,
                    ..Default::default()
                }),
            ],
            Self::MasterBus => vec![
                eq(vec![band(HighPass, 25.0, 0.0, 0.707)]),
                comp(-14.0, 1.5, 30.0, 200.0, 1.0),
                Effect::Limiter(LimiterParams {
                    ceiling_db: -1.0,
                    input_gain_db: 2.0,
                    release_ms: 60.0,
                }),
            ],
        };
        effects.into_iter().map(EffectSlot::new).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MAX_INSERTS;

    #[test]
    fn every_preset_is_valid_and_within_limits() {
        for p in InstrumentPreset::ALL {
            let chain = p.chain();
            assert!(!chain.is_empty() && chain.len() <= MAX_INSERTS, "{p:?}");
            for slot in &chain {
                assert_eq!(
                    slot.effect,
                    slot.effect.clamped(),
                    "{p:?} tem parâmetro fora da faixa"
                );
            }
        }
    }

    #[test]
    fn vocal_chain_has_deesser_and_master_ends_with_limiter() {
        assert!(InstrumentPreset::LeadVocal
            .chain()
            .iter()
            .any(|s| matches!(s.effect, Effect::DeEsser(_))));
        assert!(matches!(
            InstrumentPreset::MasterBus.chain().last().unwrap().effect,
            Effect::Limiter(_)
        ));
    }
}
