//! # dsp — processamento de áudio em tempo real (Pure Fabrication)
//!
//! Crate técnico, **sem lógica de domínio e sem I/O**: filtros, dinâmica, efeitos, medidores e
//! metrônomo usados pela engine de áudio. Não depende de nenhum bounded context (apenas do
//! `shared-kernel` para `Db`/`Pan`); a engine traduz os parâmetros do contexto Mixing para os
//! structs de parâmetros deste crate (ver ADR-0014).
//!
//! ## Contrato de tempo real
//!
//! - Toda alocação acontece em `new`/`prepare(sample_rate, max_block)`.
//! - `process` e `set_params` **não alocam, não usam lock, não fazem I/O e não entram em panic**
//!   (verificado com `assert_no_alloc` em `tests/no_alloc.rs`).
//! - Áudio estéreo `f32` não intercalado, processado *in place*; blocos de qualquer tamanho
//!   (inclusive maiores que `max_block`). Fatias de tamanhos diferentes → usa o menor.
//! - Entradas não finitas (NaN/∞) viram silêncio antes de tocar estados internos.
//! - Parâmetros são limitados (`clamped`) e suavizados onde haveria ruído de "zíper".
//! - `reset()` zera estados e leva parâmetros suavizados imediatamente ao alvo.

pub mod biquad;
pub mod chorus;
pub mod compressor;
pub mod deesser;
pub mod delay;
pub mod gate;
pub mod limiter;
pub mod meters;
pub mod metronome;
pub mod pan;
pub mod reverb;
pub mod saturation;
pub mod signals;
pub mod smoothing;
pub mod util;

pub use biquad::{Biquad, BiquadCoeffs, BiquadState, Eq, EqBandParams, FilterKind, MAX_EQ_BANDS};
pub use chorus::{Chorus, ChorusParams};
pub use compressor::{Compressor, CompressorParams};
pub use deesser::{DeEsser, DeEsserParams};
pub use delay::{Delay, DelayParams};
pub use gate::{Gate, GateParams};
pub use limiter::{Limiter, LimiterParams};
pub use meters::{
    ClipCounter, CorrelationMeter, LoudnessMeter, PeakMeter, RmsMeter, TruePeakMeter,
};
pub use metronome::Metronome;
pub use pan::{PanLaw, Panner};
pub use reverb::{Reverb, ReverbParams};
pub use saturation::{Saturation, SaturationParams};
pub use smoothing::{GainSmoother, ParamSmoother};

/// Contrato comum de todo processador de áudio estéreo.
///
/// É *object safe*: a engine guarda cadeias como `Vec<Box<dyn Processor + Send>>` (montadas
/// fora da thread de áudio).
pub trait Processor {
    /// Prepara para a taxa de amostragem e o maior bloco esperado. **Pode alocar.**
    fn prepare(&mut self, sample_rate: f32, max_block: usize);

    /// Zera estados internos (caudas, envelopes, linhas de atraso). Sem alocação.
    fn reset(&mut self);

    /// Processa um bloco estéreo *in place*. Sem alocação, lock, I/O ou panic.
    fn process(&mut self, left: &mut [f32], right: &mut [f32]);

    /// Latência introduzida, em amostras (para compensação de atraso — PDC).
    fn latency_samples(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processors_are_object_safe_and_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Eq>();
        assert_send::<Reverb>();
        assert_send::<LoudnessMeter>();
        let mut chain: Vec<Box<dyn Processor + Send>> = vec![
            Box::new(Eq::new()),
            Box::new(Compressor::new()),
            Box::new(Limiter::new()),
        ];
        let mut l = [0.1f32; 32];
        let mut r = [0.1f32; 32];
        for p in &mut chain {
            p.prepare(48_000.0, 32);
            p.process(&mut l, &mut r);
        }
        let latency: usize = chain.iter().map(|p| p.latency_samples()).sum();
        assert_eq!(
            latency, 72,
            "somente o limitador tem latência (1,5 ms @ 48 kHz)"
        );
    }

    #[test]
    fn mismatched_and_empty_slices_do_not_panic() {
        let mut procs: Vec<Box<dyn Processor>> = vec![
            Box::new(Eq::new()),
            Box::new(Compressor::new()),
            Box::new(Gate::new()),
            Box::new(Limiter::new()),
            Box::new(Reverb::new()),
            Box::new(Delay::new()),
            Box::new(Saturation::new()),
            Box::new(Chorus::new()),
            Box::new(DeEsser::new()),
            Box::new(PeakMeter::default()),
            Box::new(RmsMeter::default()),
            Box::new(LoudnessMeter::new()),
            Box::new(TruePeakMeter::new()),
            Box::new(CorrelationMeter::new()),
            Box::new(ClipCounter::default()),
            Box::new(Metronome::new()),
            Box::new(GainSmoother::default()),
            Box::new(Panner::new()),
            Box::new(Biquad::default()),
        ];
        for p in &mut procs {
            let mut l = [0.5f32; 10];
            let mut r = [0.5f32; 3];
            p.process(&mut l, &mut r);
            p.process(&mut [], &mut []);
            // Bloco maior que max_block.
            p.prepare(44_100.0, 4);
            let mut l = vec![0.25f32; 1_000];
            let mut r = vec![0.25f32; 1_000];
            p.process(&mut l, &mut r);
            assert!(l.iter().all(|v| v.is_finite()));
        }
    }
}
