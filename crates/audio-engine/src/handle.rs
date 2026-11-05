//! Lado de controle da engine.

use crate::core::{EngineCore, Garbage, Message};
use crate::graph::RenderGraph;
use crate::recorder::RecorderRt;
use crate::telemetry::{Telemetry, TelemetrySnapshot};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub message_capacity: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            message_capacity: 1024,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EngineError {
    #[error("fila de mensagens da engine cheia")]
    QueueFull,
    #[error("grafo inválido: {0}")]
    InvalidGraph(String),
}

/// Cria o par (handle de controle, núcleo de tempo real).
pub fn create_engine(config: EngineConfig) -> (EngineHandle, EngineCore) {
    let (mp, mc) = rtrb::RingBuffer::new(config.message_capacity);
    let (gp, gc) = rtrb::RingBuffer::new(config.message_capacity);
    let telemetry = Arc::new(Telemetry::default());
    let core = EngineCore::new(
        Box::new(RenderGraph::empty(config.sample_rate)),
        mc,
        gp,
        telemetry.clone(),
    );
    (
        EngineHandle {
            messages: mp,
            garbage: gc,
            telemetry,
            slots: 1,
            sample_rate: config.sample_rate,
        },
        core,
    )
}

pub struct EngineHandle {
    messages: rtrb::Producer<Message>,
    garbage: rtrb::Consumer<Garbage>,
    telemetry: Arc<Telemetry>,
    slots: usize,
    sample_rate: u32,
}

impl EngineHandle {
    fn send(&mut self, m: Message) -> Result<(), EngineError> {
        self.collect_garbage();
        self.messages.push(m).map_err(|_| EngineError::QueueFull)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Envia um novo grafo (validado). Processadores com `None` herdam do grafo atual.
    pub fn swap_graph(&mut self, graph: RenderGraph) -> Result<(), EngineError> {
        graph.validate().map_err(EngineError::InvalidGraph)?;
        self.slots = 1 + graph.tracks.len() + graph.buses.len();
        self.send(Message::SwapGraph(Box::new(graph)))
    }

    pub fn play(&mut self) -> Result<(), EngineError> {
        self.send(Message::Play)
    }

    pub fn pause(&mut self) -> Result<(), EngineError> {
        self.send(Message::Pause)
    }

    pub fn stop(&mut self, return_to: Option<u64>) -> Result<(), EngineError> {
        self.send(Message::Stop { return_to })
    }

    pub fn seek(&mut self, frame: u64) -> Result<(), EngineError> {
        self.send(Message::Seek(frame))
    }

    pub fn record(&mut self, start: u64, count_in: bool) -> Result<(), EngineError> {
        self.telemetry.set_record_start(None);
        self.send(Message::Record { start, count_in })
    }

    pub fn attach_recorder(&mut self, r: RecorderRt) -> Result<(), EngineError> {
        self.send(Message::AttachRecorder(Box::new(r)))
    }

    pub fn detach_recorder(&mut self) -> Result<(), EngineError> {
        self.send(Message::DetachRecorder)
    }

    /// Libera objetos devolvidos pelo núcleo. Retorna quantos foram liberados.
    pub fn collect_garbage(&mut self) -> usize {
        let mut n = 0;
        while let Ok(g) = self.garbage.pop() {
            drop(g);
            n += 1;
        }
        n
    }

    pub fn telemetry(&self) -> &Arc<Telemetry> {
        &self.telemetry
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        self.telemetry.snapshot(self.slots)
    }
}
