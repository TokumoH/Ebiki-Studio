//! Propriedades de robustez: para qualquer parâmetro (inclusive NaN/∞/valores absurdos) e
//! qualquer entrada finita em ±10⁴ (≈ +80 dBFS), nenhum processador produz NaN/∞.
//! Também: teto do limitador e limite da saturação valem para qualquer entrada.

use dsp::saturation::SATURATION_WET_BOUND;
use dsp::*;
use proptest::prelude::*;

const SR: f32 = 48_000.0;

fn any_param() -> impl Strategy<Value = f32> {
    prop_oneof![
        4 => -1.0e5f32..1.0e5,
        1 => Just(f32::NAN),
        1 => Just(f32::INFINITY),
        1 => Just(f32::NEG_INFINITY),
        1 => any::<f32>(),
    ]
}

fn audio() -> impl Strategy<Value = (Vec<f32>, Vec<f32>)> {
    (1usize..600).prop_flat_map(|n| {
        (
            prop::collection::vec(-1.0e4f32..1.0e4, n),
            prop::collection::vec(-1.0e4f32..1.0e4, n),
        )
    })
}

fn kind() -> impl Strategy<Value = FilterKind> {
    prop_oneof![
        Just(FilterKind::HighPass),
        Just(FilterKind::LowPass),
        Just(FilterKind::Peak),
        Just(FilterKind::LowShelf),
        Just(FilterKind::HighShelf),
        Just(FilterKind::BandPass),
    ]
}

/// Processa o áudio em blocos de 64 algumas vezes (estados acumulam) e confere finitude.
fn run_finite<P: Processor>(p: &mut P, l: &[f32], r: &[f32]) -> Result<(), TestCaseError> {
    for _ in 0..3 {
        let (mut l, mut r) = (l.to_vec(), r.to_vec());
        signals::process_in_blocks(p, &mut l, &mut r, 64);
        prop_assert!(l.iter().chain(r.iter()).all(|v| v.is_finite()));
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn eq_is_finite(
        (l, r) in audio(),
        k in kind(), f in any_param(), g in any_param(), q in any_param(), sr in any_param(),
    ) {
        let mut eq = Eq::new();
        eq.prepare(sr, 64);
        for i in 0..MAX_EQ_BANDS {
            eq.set_band(i, EqBandParams { kind: k, enabled: true, freq_hz: f, gain_db: g, q });
        }
        run_finite(&mut eq, &l, &r)?;
        let mut bq = Biquad::new(BiquadCoeffs::design(k, f, q, g, sr));
        run_finite(&mut bq, &l, &r)?;
    }

    #[test]
    fn dynamics_are_finite(
        (l, r) in audio(),
        a in any_param(), b in any_param(), c in any_param(),
        d in any_param(), e in any_param(), f in any_param(),
    ) {
        let mut comp = Compressor::new();
        comp.prepare(SR, 64);
        comp.set_params(CompressorParams {
            threshold_db: a, ratio: b, attack_ms: c, release_ms: d, knee_db: e, makeup_db: f,
        });
        run_finite(&mut comp, &l, &r)?;
        prop_assert!(comp.gain_reduction_db().is_finite());

        let mut gate = Gate::new();
        gate.prepare(SR, 64);
        gate.set_params(GateParams {
            threshold_db: a, attack_ms: b, hold_ms: c, release_ms: d, range_db: e,
        });
        run_finite(&mut gate, &l, &r)?;

        let mut de = DeEsser::new();
        de.prepare(SR, 64);
        de.set_params(DeEsserParams { freq_hz: a, threshold_db: b, reduction_db: c });
        run_finite(&mut de, &l, &r)?;
    }

    #[test]
    fn limiter_never_exceeds_ceiling(
        (l, r) in audio(),
        ceiling in any_param(), gain in any_param(), rel in any_param(),
    ) {
        let mut lim = Limiter::new();
        lim.prepare(SR, 64);
        lim.set_params(LimiterParams { ceiling_db: ceiling, input_gain_db: gain, release_ms: rel });
        let ceil = util::db_to_lin(lim.params().ceiling_db);
        let (mut l, mut r) = (l, r);
        signals::process_in_blocks(&mut lim, &mut l, &mut r, 64);
        prop_assert!(l.iter().chain(r.iter()).all(|v| v.is_finite() && v.abs() <= ceil));
    }

    #[test]
    fn time_based_effects_are_finite(
        (l, r) in audio(),
        a in any_param(), b in any_param(), c in any_param(),
        d in any_param(), e in any_param(), pp in any::<bool>(),
    ) {
        let mut rev = Reverb::new();
        rev.prepare(SR, 64);
        rev.set_params(ReverbParams { room_size: a, damping: b, width: c, predelay_ms: d, mix: e });
        run_finite(&mut rev, &l, &r)?;

        let mut del = Delay::new();
        del.prepare(SR, 64);
        del.set_params(DelayParams { time_ms: a, feedback: b, high_cut_hz: c, mix: d, ping_pong: pp });
        prop_assert!(del.params().feedback < 1.0);
        run_finite(&mut del, &l, &r)?;

        let mut ch = Chorus::new();
        ch.prepare(SR, 64);
        ch.set_params(ChorusParams { rate_hz: a, depth_ms: b, mix: c });
        run_finite(&mut ch, &l, &r)?;
    }

    #[test]
    fn saturation_wet_is_bounded(
        (l, r) in audio(), drive in any_param(), tone in any_param(),
    ) {
        let mut s = Saturation::new();
        s.prepare(SR, 64);
        s.set_params(SaturationParams { drive_db: drive, tone, mix: 1.0 });
        s.reset();
        let (mut l, mut r) = (l, r);
        signals::process_in_blocks(&mut s, &mut l, &mut r, 64);
        prop_assert!(l.iter().chain(r.iter()).all(|v| v.is_finite() && v.abs() <= SATURATION_WET_BOUND));
    }

    #[test]
    fn meters_report_finite_values((l, r) in audio()) {
        let mut lufs = LoudnessMeter::new();
        let mut tp = TruePeakMeter::new();
        let mut corr = CorrelationMeter::new();
        let mut peak = PeakMeter::default();
        let mut rms = RmsMeter::default();
        for _ in 0..20 {
            lufs.analyze(&l, &r);
            tp.analyze(&l, &r);
            corr.analyze(&l, &r);
            peak.analyze(&l, &r);
            rms.analyze(&l, &r);
        }
        prop_assert!(lufs.momentary_lufs().is_finite());
        prop_assert!(lufs.integrated_lufs().unwrap_or(0.0).is_finite());
        prop_assert!(tp.max_db().is_finite());
        // True peak ≥ pico amostral.
        let sp = signals::peak(&l).max(signals::peak(&r));
        prop_assert!(tp.max_db() >= util::lin_to_db(sp) - 1e-3);
        let c = corr.correlation();
        prop_assert!((-1.0..=1.0).contains(&c));
        prop_assert!(peak.level_db().iter().all(|v| v.is_finite()));
        prop_assert!(rms.rms_db().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn gain_smoother_is_monotonic_toward_target(start in -10.0f32..10.0, target in -10.0f32..10.0) {
        let mut g = GainSmoother::new(start);
        g.prepare(SR, 64);
        g.set_target(target);
        let mut prev = start;
        for _ in 0..2_000 {
            let v = g.next_value();
            prop_assert!((v - prev) * (target - start) >= -1e-6);
            prev = v;
        }
        prop_assert_eq!(g.current(), target);
    }
}

/// Regressão (achada pelo proptest na CI): o erro acumulado da rampa ultrapassava o alvo e o
/// último passo recuava. Valores exatos (bits) de um caso que falhava no código antigo.
#[test]
fn gain_smoother_regression_float_overshoot() {
    let (start, target) = (f32::from_bits(0x4092_eb58), f32::from_bits(0x4094_fa46));
    let mut g = GainSmoother::new(start);
    g.prepare(SR, 64);
    g.set_target(target);
    let mut prev = start;
    for _ in 0..2_000 {
        let v = g.next_value();
        assert!(
            (v - prev) * (target - start) >= -1e-6,
            "recuou: {prev} → {v}"
        );
        prev = v;
    }
    assert_eq!(g.current(), target);
}
