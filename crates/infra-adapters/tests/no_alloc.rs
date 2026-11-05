//! O insert de correção de afinação não aloca nem libera memória em `process`/`set_params`/`reset`.
use app_core::ports::ProcessorFactory;
use assert_no_alloc::{assert_no_alloc, reset_violation_count, violation_count, AllocDisabler};
use ctx_mixing::{Effect, PitchCorrectorParams};
use infra_adapters::dsp_factory::DspProcessorFactory;

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

#[test]
fn pitch_corrector_insert_does_not_allocate() {
    let f = DspProcessorFactory;
    let e = Effect::PitchCorrector(PitchCorrectorParams::major(9));
    let mut p = f.create(&e, 48_000);
    let params = f.params(&Effect::PitchCorrector(PitchCorrectorParams {
        humanize: 0.3,
        ..PitchCorrectorParams::major(2)
    }));
    let mut l: Vec<f32> = (0..2048)
        .map(|i| 0.4 * (std::f32::consts::TAU * 452.0 * i as f32 / 48_000.0).sin())
        .collect();
    let mut r = l.clone();
    reset_violation_count();
    assert_no_alloc(|| {
        for i in 0..60 {
            if i == 30 {
                p.set_params(&params);
            }
            // bloco maior que MAX_BLOCK é dividido internamente
            p.process(&mut l, &mut r);
        }
        p.reset();
    });
    assert_eq!(violation_count(), 0, "o insert alocou memória");
}
