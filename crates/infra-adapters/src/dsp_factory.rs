//! `ProcessorFactory` sobre o crate `dsp`. Parâmetros são achatados em `Vec<f32>` numa ordem fixa por
//! tipo; o processador reconstrói o struct em `set_params` sem alocar.

use app_core::ports::ProcessorFactory;
use audio_engine::AudioProcessor;
use ctx_mixing::{Effect, EqBandKind};
use dsp::Processor;

const MAX_BLOCK: usize = audio_engine::MAX_BLOCK;

#[derive(Debug, Default, Clone, Copy)]
pub struct DspProcessorFactory;

fn kind_code(k: EqBandKind) -> f32 {
    match k {
        EqBandKind::HighPass => 0.0,
        EqBandKind::LowShelf => 1.0,
        EqBandKind::Peak => 2.0,
        EqBandKind::HighShelf => 3.0,
        EqBandKind::LowPass => 4.0,
    }
}

fn filter_kind(code: f32) -> dsp::FilterKind {
    match code as i32 {
        0 => dsp::FilterKind::HighPass,
        1 => dsp::FilterKind::LowShelf,
        3 => dsp::FilterKind::HighShelf,
        4 => dsp::FilterKind::LowPass,
        _ => dsp::FilterKind::Peak,
    }
}

fn b(x: bool) -> f32 {
    if x {
        1.0
    } else {
        0.0
    }
}

fn g(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// Adapta qualquer `dsp::Processor` + função de parâmetros ao `AudioProcessor` da engine.
struct Adapter<P: Processor + Send> {
    inner: P,
    apply: fn(&mut P, &[f32]),
}

impl<P: Processor + Send> AudioProcessor for Adapter<P> {
    fn set_params(&mut self, params: &[f32]) {
        (self.apply)(&mut self.inner, params);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.inner.process(left, right);
    }
    fn reset(&mut self) {
        self.inner.reset();
    }
    fn latency_samples(&self) -> usize {
        self.inner.latency_samples()
    }
}

fn boxed<P: Processor + Send + 'static>(
    mut inner: P,
    sr: u32,
    apply: fn(&mut P, &[f32]),
    params: &[f32],
) -> Box<dyn AudioProcessor> {
    inner.prepare(sr as f32, MAX_BLOCK);
    apply(&mut inner, params);
    inner.reset(); // parâmetros iniciais sem rampa
    Box::new(Adapter { inner, apply })
}

impl ProcessorFactory for DspProcessorFactory {
    fn type_tag(&self, effect: &Effect) -> u32 {
        match effect {
            Effect::Eq(_) => 1,
            Effect::Compressor(_) => 2,
            Effect::Gate(_) => 3,
            Effect::Limiter(_) => 4,
            Effect::Reverb(_) => 5,
            Effect::Delay(_) => 6,
            Effect::Saturation(_) => 7,
            Effect::Chorus(_) => 8,
            Effect::DeEsser(_) => 9,
        }
    }

    fn params(&self, effect: &Effect) -> Vec<f32> {
        match effect.clamped() {
            Effect::Eq(p) => {
                let mut v = vec![p.bands.len() as f32];
                for band in &p.bands {
                    v.extend([
                        kind_code(band.kind),
                        b(band.enabled),
                        band.freq_hz,
                        band.gain_db,
                        band.q,
                    ]);
                }
                v
            }
            Effect::Compressor(p) => vec![
                p.threshold_db,
                p.ratio,
                p.attack_ms,
                p.release_ms,
                p.knee_db,
                p.makeup_db,
            ],
            Effect::Gate(p) => vec![
                p.threshold_db,
                p.attack_ms,
                p.hold_ms,
                p.release_ms,
                p.range_db,
            ],
            Effect::Limiter(p) => vec![p.ceiling_db, p.input_gain_db, p.release_ms],
            Effect::Reverb(p) => vec![p.room_size, p.damping, p.width, p.predelay_ms, p.mix],
            Effect::Delay(p) => vec![p.time_ms, p.feedback, p.high_cut_hz, p.mix, b(p.ping_pong)],
            Effect::Saturation(p) => vec![p.drive_db, p.tone, p.mix],
            Effect::Chorus(p) => vec![p.rate_hz, p.depth_ms, p.mix],
            Effect::DeEsser(p) => vec![p.freq_hz, p.threshold_db, p.reduction_db],
        }
    }

    fn create(&self, effect: &Effect, sr: u32) -> Box<dyn AudioProcessor> {
        let params = self.params(effect);
        match effect {
            Effect::Eq(_) => boxed(dsp::Eq::new(), sr, apply_eq, &params),
            Effect::Compressor(_) => boxed(
                dsp::Compressor::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::CompressorParams {
                        threshold_db: g(p, 0, -18.0),
                        ratio: g(p, 1, 3.0),
                        attack_ms: g(p, 2, 10.0),
                        release_ms: g(p, 3, 120.0),
                        knee_db: g(p, 4, 6.0),
                        makeup_db: g(p, 5, 0.0),
                    })
                },
                &params,
            ),
            Effect::Gate(_) => boxed(
                dsp::Gate::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::GateParams {
                        threshold_db: g(p, 0, -50.0),
                        attack_ms: g(p, 1, 1.0),
                        hold_ms: g(p, 2, 30.0),
                        release_ms: g(p, 3, 150.0),
                        range_db: g(p, 4, -60.0),
                    })
                },
                &params,
            ),
            Effect::Limiter(_) => boxed(
                dsp::Limiter::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::LimiterParams {
                        ceiling_db: g(p, 0, -1.0),
                        input_gain_db: g(p, 1, 0.0),
                        release_ms: g(p, 2, 80.0),
                    })
                },
                &params,
            ),
            Effect::Reverb(_) => boxed(
                dsp::Reverb::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::ReverbParams {
                        room_size: g(p, 0, 0.6),
                        damping: g(p, 1, 0.5),
                        width: g(p, 2, 1.0),
                        predelay_ms: g(p, 3, 20.0),
                        mix: g(p, 4, 0.25),
                    })
                },
                &params,
            ),
            Effect::Delay(_) => boxed(
                dsp::Delay::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::DelayParams {
                        time_ms: g(p, 0, 375.0),
                        feedback: g(p, 1, 0.35),
                        high_cut_hz: g(p, 2, 6000.0),
                        mix: g(p, 3, 0.2),
                        ping_pong: g(p, 4, 0.0) > 0.5,
                    })
                },
                &params,
            ),
            Effect::Saturation(_) => boxed(
                dsp::Saturation::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::SaturationParams {
                        drive_db: g(p, 0, 6.0),
                        tone: g(p, 1, 0.5),
                        mix: g(p, 2, 1.0),
                    })
                },
                &params,
            ),
            Effect::Chorus(_) => boxed(
                dsp::Chorus::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::ChorusParams {
                        rate_hz: g(p, 0, 0.8),
                        depth_ms: g(p, 1, 3.0),
                        mix: g(p, 2, 0.4),
                    })
                },
                &params,
            ),
            Effect::DeEsser(_) => boxed(
                dsp::DeEsser::new(),
                sr,
                |c, p| {
                    c.set_params(dsp::DeEsserParams {
                        freq_hz: g(p, 0, 6500.0),
                        threshold_db: g(p, 1, -28.0),
                        reduction_db: g(p, 2, -8.0),
                    })
                },
                &params,
            ),
        }
    }
}

fn apply_eq(eq: &mut dsp::Eq, p: &[f32]) {
    let n = (g(p, 0, 0.0).max(0.0) as usize).min(dsp::MAX_EQ_BANDS);
    for i in 0..dsp::MAX_EQ_BANDS {
        let base = 1 + i * 5;
        let params = if i < n {
            dsp::EqBandParams {
                kind: filter_kind(g(p, base, 2.0)),
                enabled: g(p, base + 1, 0.0) > 0.5,
                freq_hz: g(p, base + 2, 1000.0),
                gain_db: g(p, base + 3, 0.0),
                q: g(p, base + 4, 0.707),
            }
        } else {
            dsp::EqBandParams {
                enabled: false,
                ..Default::default()
            }
        };
        eq.set_band(i, params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctx_mixing::presets::InstrumentPreset;
    use ctx_mixing::*;

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    fn sine(freq: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn every_effect_type_has_distinct_tag_and_runs_finite() {
        let f = DspProcessorFactory;
        let mut tags = std::collections::HashSet::new();
        for preset in InstrumentPreset::ALL {
            for slot in preset.chain() {
                tags.insert(f.type_tag(&slot.effect));
                let mut p = f.create(&slot.effect, 48_000);
                let mut l = sine(440.0, 0.5, 4096);
                let mut r = l.clone();
                p.process(&mut l, &mut r);
                assert!(
                    l.iter().chain(&r).all(|x| x.is_finite()),
                    "{:?}",
                    slot.effect.name()
                );
            }
        }
        assert_eq!(
            tags.len(),
            9,
            "todos os 9 tipos aparecem nos presets com tags distintas"
        );
    }

    #[test]
    fn eq_params_roundtrip_through_flattening() {
        let f = DspProcessorFactory;
        let eq = EqParams {
            bands: vec![EqBand {
                kind: EqBandKind::Peak,
                enabled: true,
                freq_hz: 1000.0,
                gain_db: 12.0,
                q: 1.0,
            }],
        };
        let e = Effect::Eq(eq);
        let mut p = f.create(&e, 48_000);
        let mut l = sine(1000.0, 0.1, 48_000);
        let mut r = l.clone();
        let before = rms(&l[24_000..]);
        p.process(&mut l, &mut r);
        let gain_db = 20.0 * (rms(&l[24_000..]) / before).log10();
        assert!(
            (gain_db - 12.0).abs() < 0.6,
            "pico de +12 dB a 1 kHz, obtido {gain_db:.2}"
        );
    }

    #[test]
    fn compressor_params_change_output_via_set_params() {
        let f = DspProcessorFactory;
        let e = Effect::Compressor(CompressorParams {
            threshold_db: -40.0,
            ratio: 10.0,
            ..Default::default()
        });
        let mut p = f.create(&e, 48_000);
        let mut l = sine(440.0, 0.5, 48_000);
        let mut r = l.clone();
        p.process(&mut l, &mut r);
        let squashed = rms(&l[40_000..]);
        p.set_params(&f.params(&Effect::Compressor(CompressorParams {
            threshold_db: 0.0,
            ratio: 1.0,
            ..Default::default()
        })));
        p.reset();
        let mut l2 = sine(440.0, 0.5, 48_000);
        let mut r2 = l2.clone();
        p.process(&mut l2, &mut r2);
        assert!(
            rms(&l2[40_000..]) > squashed * 4.0,
            "sem compressão o sinal é bem mais alto"
        );
    }
}
