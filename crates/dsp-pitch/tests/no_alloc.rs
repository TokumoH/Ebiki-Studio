//! Garante que os processadores de tempo real não alocam após a preparação.
//! (Binário de teste próprio: o alocador global é substituído pelo `AllocDisabler`.)

use assert_no_alloc::{assert_no_alloc, AllocDisabler};
use dsp_pitch::corrector::{CorrectorSettings, PitchCorrector, ScaleMask};
use dsp_pitch::detector::{PitchDetector, YinConfig, YinDetector};
use dsp_pitch::shifter::{PitchShifter, PsolaConfig, PsolaShifter};
use dsp_pitch::signal;
use dsp_pitch::tuner::TunerProcessor;
use shared_kernel::music::PitchClass;

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

const SR: f32 = 48_000.0;
const BLOCK: usize = 256;

#[test]
fn yin_detect_does_not_allocate() {
    let mut yin = YinDetector::new(YinConfig::default(), SR);
    let sig = signal::sine(220.0, SR, yin.frame_len(), 0.5);
    let est = assert_no_alloc(|| yin.detect(&sig));
    assert!(est.voiced);
}

#[test]
fn tuner_processor_does_not_allocate() {
    let mut tuner = TunerProcessor::new(SR, 440.0, YinConfig::default(), 0);
    let sig = signal::sine(440.0, SR, 9600, 0.5);
    assert_no_alloc(|| {
        for block in sig.chunks(BLOCK) {
            tuner.process(block);
        }
        tuner.set_a4(442.0);
    });
    assert!(tuner.reading().is_some());
}

#[test]
fn psola_shifter_does_not_allocate() {
    let mut shifter = PsolaShifter::new(PsolaConfig::default(), SR);
    shifter.prepare(SR, BLOCK);
    let sig = signal::sine(220.0, SR, 9600, 0.5);
    let ratios = vec![1.5f32; BLOCK];
    let mut out = vec![0.0f32; BLOCK];
    assert_no_alloc(|| {
        for block in sig.chunks(BLOCK) {
            shifter.process(block, &mut out[..block.len()], 2.0);
            shifter.process_varying(block, &mut out[..block.len()], &ratios);
        }
        shifter.reset();
    });
    assert!(out.iter().all(|v| v.is_finite()));
}

#[test]
fn pitch_corrector_does_not_allocate() {
    let c = PitchClass::parse("C").unwrap();
    let mut corrector = PitchCorrector::new(
        CorrectorSettings {
            scale: ScaleMask::major(c),
            ..Default::default()
        },
        SR,
    );
    corrector.prepare(SR, BLOCK);
    let sig = signal::sine(452.0, SR, 9600, 0.5);
    let mut out = vec![0.0f32; BLOCK];
    assert_no_alloc(|| {
        for block in sig.chunks(BLOCK) {
            corrector.process(block, &mut out[..block.len()]);
        }
        corrector.set_settings(CorrectorSettings {
            retune_ms: 50.0,
            humanize: 0.3,
            ..corrector.settings()
        });
        corrector.process(&sig[..BLOCK], &mut out);
    });
    assert!(out.iter().all(|v| v.is_finite()));
}
