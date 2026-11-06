//! Dispositivo "nulo": thread que chama o núcleo em tempo real simulado, sem placa de som.
//! Usado quando não há hardware (CI, máquinas sem áudio) e nos testes de integração.

use crate::core::EngineCore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Gera a entrada de cada bloco (intercalada). Recebe o número de quadros.
pub type InputGenerator = Box<dyn FnMut(&mut [f32]) + Send>;

pub struct NullDevice {
    running: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl NullDevice {
    /// Inicia com blocos de `block` quadros, `in_channels` canais de entrada simulados.
    pub fn start(
        mut core: EngineCore,
        block: usize,
        in_channels: usize,
        mut input: Option<InputGenerator>,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let sr = core.sample_rate() as f64;
        let thread = std::thread::Builder::new()
            .name("ebiki-null-audio".into())
            .spawn(move || {
                let mut out = vec![0.0f32; block * 2];
                let mut inp = vec![0.0f32; block * in_channels];
                let period = Duration::from_secs_f64(block as f64 / sr);
                let mut next = Instant::now();
                while r.load(Ordering::Acquire) {
                    if let Some(g) = input.as_mut() {
                        g(&mut inp);
                    }
                    core.process(&mut out, 2, &inp, in_channels);
                    next += period;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                }
            })
            .expect("criar thread de áudio nula");
        Self {
            running,
            thread: Some(thread),
        }
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for NullDevice {
    fn drop(&mut self) {
        self.shutdown();
    }
}
