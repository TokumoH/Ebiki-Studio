#![allow(dead_code)]
use audio_engine::*;
use std::sync::Arc;

pub const SR: u32 = 48_000;

pub fn sine(freq: f32, amp: f32, frames: usize) -> Arc<SourceBuffer> {
    let ch: Vec<f32> = (0..frames)
        .map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / SR as f32).sin())
        .collect();
    Arc::new(SourceBuffer {
        channels: vec![ch.clone(), ch],
    })
}

pub fn dc(value: f32, frames: usize) -> Arc<SourceBuffer> {
    Arc::new(SourceBuffer {
        channels: vec![vec![value; frames]],
    })
}

/// Rampa 0, 1, 2, … (identifica a amostra de origem pelo valor).
pub fn ramp(frames: usize) -> Arc<SourceBuffer> {
    Arc::new(SourceBuffer {
        channels: vec![(0..frames).map(|i| i as f32).collect()],
    })
}

/// Impulso unitário na amostra `at`.
pub fn impulse(at: usize, frames: usize) -> Arc<SourceBuffer> {
    let mut v = vec![0.0; frames];
    v[at] = 1.0;
    Arc::new(SourceBuffer { channels: vec![v] })
}

pub fn track(key: u128, slot: usize, clips: Vec<ClipRender>) -> TrackRender {
    TrackRender {
        strip: StripRender::new(key, slot),
        clips,
        monitor: None,
    }
}

pub fn clip(source: Arc<SourceBuffer>, start: u64) -> ClipRender {
    let length = source.frames();
    ClipRender::new(source, start, 0, length, 1.0)
}

/// Engine com o grafo aplicado e tocando.
pub fn playing(graph: RenderGraph) -> (EngineHandle, EngineCore) {
    let (mut h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    h.swap_graph(graph).unwrap();
    h.play().unwrap();
    core.process(&mut [], 2, &[], 0); // aplica mensagens
    (h, core)
}

/// Processador de teste: multiplica por `params[0]` e conta amostras processadas (estado).
pub struct Gain {
    pub g: f32,
    pub processed: Arc<std::sync::atomic::AtomicU64>,
}

impl AudioProcessor for Gain {
    fn set_params(&mut self, p: &[f32]) {
        self.g = p.first().copied().unwrap_or(1.0);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for x in l.iter_mut().chain(r.iter_mut()) {
            *x *= self.g;
        }
        self.processed
            .fetch_add(l.len() as u64, std::sync::atomic::Ordering::Relaxed);
    }
    fn reset(&mut self) {}
}
