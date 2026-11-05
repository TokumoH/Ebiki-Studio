//! Garante que `process`, `set_params` e leituras de medidores não alocam.
//!
//! Este binário de teste instala o alocador global `AllocDisabler`; qualquer (des)alocação
//! dentro de `assert_no_alloc` incrementa o contador de violações da thread (modo "warn"),
//! permitindo uma asserção explícita em debug e em release.

use assert_no_alloc::{assert_no_alloc, reset_violation_count, violation_count, AllocDisabler};
use dsp::*;

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

const SR: f32 = 48_000.0;
const BLOCK: usize = 128;

fn signal() -> (Vec<f32>, Vec<f32>) {
    (
        signals::noise(BLOCK, 0.9, 1),
        signals::sine(440.0, 0.8, SR, BLOCK, 0.0),
    )
}

/// Executa `f` sem permitir alocação e verifica que nenhuma violação ocorreu.
fn check<F: FnOnce()>(name: &str, f: F) {
    reset_violation_count();
    assert_no_alloc(f);
    assert_eq!(
        violation_count(),
        0,
        "{name} alocou memória no caminho de tempo real"
    );
}

#[test]
fn guard_detects_allocation() {
    // Sanidade: o detector realmente pega uma alocação.
    reset_violation_count();
    assert_no_alloc(|| {
        let v = vec![1u8; 16];
        std::hint::black_box(&v);
    });
    assert!(violation_count() > 0);
    reset_violation_count();
}

#[test]
fn effects_process_and_set_params_do_not_allocate() {
    let (l0, r0) = signal();
    let (mut l, mut r) = (l0.clone(), r0.clone());

    let mut eq = Eq::new();
    eq.prepare(SR, BLOCK);
    let bands = [
        EqBandParams {
            kind: FilterKind::HighPass,
            enabled: true,
            freq_hz: 80.0,
            gain_db: 0.0,
            q: 0.707,
        },
        EqBandParams {
            kind: FilterKind::Peak,
            enabled: true,
            freq_hz: 2_500.0,
            gain_db: 4.0,
            q: 1.2,
        },
        EqBandParams {
            kind: FilterKind::HighShelf,
            enabled: true,
            freq_hz: 9_000.0,
            gain_db: -3.0,
            q: 0.707,
        },
    ];
    check("Eq", || {
        eq.set_bands(&bands);
        for _ in 0..4 {
            eq.process(&mut l, &mut r);
        }
        eq.set_band(
            1,
            EqBandParams {
                gain_db: -6.0,
                ..bands[1]
            },
        );
        eq.process(&mut l, &mut r);
        eq.reset();
    });

    let mut comp = Compressor::new();
    comp.prepare(SR, BLOCK);
    check("Compressor", || {
        comp.set_params(CompressorParams {
            threshold_db: -30.0,
            ratio: 6.0,
            ..Default::default()
        });
        comp.process(&mut l, &mut r);
        std::hint::black_box(comp.gain_reduction_db());
    });

    let mut gate = Gate::new();
    gate.prepare(SR, BLOCK);
    check("Gate", || {
        gate.set_params(GateParams::default());
        gate.process(&mut l, &mut r);
    });

    let mut lim = Limiter::new();
    lim.prepare(SR, BLOCK);
    check("Limiter", || {
        lim.set_params(LimiterParams {
            input_gain_db: 12.0,
            ..Default::default()
        });
        lim.process(&mut l, &mut r);
        lim.reset();
    });

    let mut rev = Reverb::new();
    rev.prepare(SR, BLOCK);
    check("Reverb", || {
        rev.set_params(ReverbParams {
            predelay_ms: 150.0,
            ..Default::default()
        });
        rev.process(&mut l, &mut r);
        rev.reset();
    });

    let mut del = Delay::new();
    del.prepare(SR, BLOCK);
    check("Delay", || {
        del.set_params(DelayParams {
            time_ms: 1_999.0,
            ping_pong: true,
            ..Default::default()
        });
        del.process(&mut l, &mut r);
        del.reset();
    });

    let mut sat = Saturation::new();
    sat.prepare(SR, BLOCK);
    check("Saturation", || {
        sat.set_params(SaturationParams::default());
        sat.process(&mut l, &mut r);
    });

    let mut ch = Chorus::new();
    ch.prepare(SR, BLOCK);
    check("Chorus", || {
        ch.set_params(ChorusParams::default());
        ch.process(&mut l, &mut r);
    });

    let mut de = DeEsser::new();
    de.prepare(SR, BLOCK);
    check("DeEsser", || {
        de.set_params(DeEsserParams {
            threshold_db: -50.0,
            ..Default::default()
        });
        de.process(&mut l, &mut r);
    });

    let mut gs = GainSmoother::new(1.0);
    gs.prepare(SR, BLOCK);
    let mut pan = Panner::new();
    pan.prepare(SR, BLOCK);
    let mut bq = Biquad::new(BiquadCoeffs::design(
        FilterKind::LowPass,
        5_000.0,
        0.7,
        0.0,
        SR,
    ));
    check("Utilitários", || {
        gs.set_target(0.5);
        gs.process(&mut l, &mut r);
        pan.set_params(0.3, 0.8);
        pan.process(&mut l, &mut r);
        bq.set_coeffs(BiquadCoeffs::design(
            FilterKind::HighPass,
            100.0,
            0.7,
            0.0,
            SR,
        ));
        bq.process(&mut l, &mut r);
        std::hint::black_box(PanLaw::gains(-0.5));
    });
}

#[test]
fn meters_and_metronome_do_not_allocate() {
    let (l0, r0) = signal();
    let (mut l, mut r) = (l0.clone(), r0.clone());

    let mut peak = PeakMeter::default();
    let mut rms = RmsMeter::default();
    let mut lufs = LoudnessMeter::new();
    let mut tp = TruePeakMeter::new();
    let mut corr = CorrelationMeter::new();
    let mut clip = ClipCounter::default();
    let mut metro = Metronome::new();
    peak.prepare(SR, BLOCK);
    rms.prepare(SR, BLOCK);
    lufs.prepare(SR, BLOCK);
    tp.prepare(SR, BLOCK);
    corr.prepare(SR, BLOCK);
    clip.prepare(SR, BLOCK);
    metro.prepare(SR, BLOCK);

    check("Medidores", || {
        // ~1 s de blocos: atravessa sub-blocos de 100 ms e o histograma do integrado.
        for _ in 0..400 {
            peak.analyze(&l, &r);
            rms.analyze(&l, &r);
            lufs.analyze(&l, &r);
            tp.analyze(&l, &r);
            corr.analyze(&l, &r);
            clip.analyze(&l, &r);
        }
        std::hint::black_box((
            peak.level_db(),
            peak.hold_db(),
            rms.rms_db(),
            lufs.momentary_lufs(),
            lufs.short_term_lufs(),
            lufs.integrated_lufs(),
            tp.true_peak_db(),
            corr.correlation(),
            clip.events(),
        ));
        lufs.reset();
    });

    check("Metronome", || {
        metro.set_volume_db(-3.0);
        metro.trigger(5, true);
        metro.trigger(300, false);
        for _ in 0..4 {
            metro.process(&mut l, &mut r);
        }
    });
}

#[test]
fn boxed_chain_processes_without_allocation() {
    let mut chain: Vec<Box<dyn Processor + Send>> = vec![
        Box::new(Eq::new()),
        Box::new(Gate::new()),
        Box::new(Compressor::new()),
        Box::new(DeEsser::new()),
        Box::new(Saturation::new()),
        Box::new(Chorus::new()),
        Box::new(Delay::new()),
        Box::new(Reverb::new()),
        Box::new(Limiter::new()),
    ];
    for p in &mut chain {
        p.prepare(SR, BLOCK);
    }
    let (mut l, mut r) = signal();
    check("Cadeia", || {
        for _ in 0..50 {
            for p in chain.iter_mut() {
                p.process(&mut l, &mut r);
            }
        }
    });
}
