//! Grafo de renderização compilado (imutável em estrutura, montado fora da thread de áudio).

use crate::MAX_BLOCK;
use shared_kernel::TempoMap;
use std::sync::Arc;

/// Áudio decodificado na taxa do projeto, canais não intercalados. Compartilhado (Arc) entre clipes.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceBuffer {
    pub channels: Vec<Vec<f32>>,
}

impl SourceBuffer {
    pub fn frames(&self) -> u64 {
        self.channels.first().map_or(0, |c| c.len() as u64)
    }
}

/// Clipe pronto para tocar: tudo em amostras.
#[derive(Debug, Clone)]
pub struct ClipRender {
    pub source: Arc<SourceBuffer>,
    /// Início na timeline (amostras).
    pub start: u64,
    /// Deslocamento dentro da fonte.
    pub offset: u64,
    pub length: u64,
    /// Ganho linear do clipe.
    pub gain: f32,
}

impl ClipRender {
    pub fn end(&self) -> u64 {
        self.start + self.length
    }
}

/// Porta para processadores de efeito (ADR-0011). Implementações não podem alocar em
/// `set_params`/`process`/`reset`.
pub trait AudioProcessor: Send {
    fn set_params(&mut self, params: &[f32]);
    fn process(&mut self, left: &mut [f32], right: &mut [f32]);
    fn reset(&mut self);
    fn latency_samples(&self) -> usize {
        0
    }
}

/// Efeito inserido. `processor` pode vir vazio quando deve ser herdado do grafo anterior
/// (mesmo `id` e `type_tag`), preservando o estado interno.
pub struct InsertRender {
    pub id: u128,
    pub type_tag: u32,
    pub bypass: bool,
    pub params: Vec<f32>,
    pub processor: Option<Box<dyn AudioProcessor>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Master,
    Bus(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SendRender {
    pub bus: usize,
    pub gain: f32,
    pub pre_fader: bool,
}

/// Estado que sobrevive à troca de grafo (por chave do canal).
#[derive(Debug, Clone, Copy, Default)]
pub struct StripState {
    pub current_gain: [f32; 2],
    pub initialized: bool,
}

/// Canal compilado.
pub struct StripRender {
    /// Chave estável (id da faixa/bus; 0 = master).
    pub key: u128,
    /// Índice do medidor na telemetria.
    pub meter_slot: usize,
    pub volume: f32,
    pub pan: f32,
    pub audible: bool,
    pub inserts: Vec<InsertRender>,
    pub sends: Vec<SendRender>,
    pub dest: Destination,
    pub state: StripState,
    pub buf_l: Vec<f32>,
    pub buf_r: Vec<f32>,
}

impl StripRender {
    pub fn new(key: u128, meter_slot: usize) -> Self {
        Self {
            key,
            meter_slot,
            volume: 1.0,
            pan: 0.0,
            audible: true,
            inserts: Vec::new(),
            sends: Vec::new(),
            dest: Destination::Master,
            state: StripState::default(),
            buf_l: vec![0.0; MAX_BLOCK],
            buf_r: vec![0.0; MAX_BLOCK],
        }
    }

    /// Ganhos alvo (L, R) pela lei de **balanço** (canais estéreo): centro = ganho unitário em ambos
    /// os lados; pan para um lado atenua só o lado oposto. 0 se inaudível.
    pub fn target_gains(&self) -> [f32; 2] {
        if !self.audible {
            return [0.0, 0.0];
        }
        [
            self.volume * balance_left(self.pan),
            self.volume * balance_right(self.pan),
        ]
    }
}

/// Ganho do lado esquerdo na lei de balanço.
pub fn balance_left(pan: f32) -> f32 {
    let p = if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    if p <= 0.0 {
        1.0
    } else {
        1.0 - p
    }
}

/// Ganho do lado direito na lei de balanço.
pub fn balance_right(pan: f32) -> f32 {
    let p = if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    if p >= 0.0 {
        1.0
    } else {
        1.0 + p
    }
}

/// Monitoração de entrada numa faixa (canais da entrada; `right` None = mono duplicado).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputMonitor {
    pub left: usize,
    pub right: Option<usize>,
}

pub struct TrackRender {
    pub strip: StripRender,
    /// Ordenados por `start`.
    pub clips: Vec<ClipRender>,
    pub monitor: Option<InputMonitor>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetronomeSettings {
    pub enabled: bool,
    pub volume: f32,
    pub count_in_bars: u32,
    /// Subdivisões por tempo (1 = só tempos).
    pub subdivision: u32,
    pub accent_first_beat: bool,
}

impl Default for MetronomeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0.5,
            count_in_bars: 1,
            subdivision: 1,
            accent_first_beat: true,
        }
    }
}

/// Grafo completo.
pub struct RenderGraph {
    pub sample_rate: u32,
    pub tracks: Vec<TrackRender>,
    pub buses: Vec<StripRender>,
    pub master: StripRender,
    pub tempo: TempoMap,
    pub metronome: MetronomeSettings,
    pub loop_range: Option<(u64, u64)>,
    pub punch: Option<(u64, u64)>,
}

impl RenderGraph {
    pub fn empty(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            tracks: Vec::new(),
            buses: Vec::new(),
            master: StripRender::new(0, 0),
            tempo: TempoMap::default(),
            metronome: MetronomeSettings::default(),
            loop_range: None,
            punch: None,
        }
    }

    /// Fim do conteúdo em amostras.
    pub fn content_end(&self) -> u64 {
        self.tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(ClipRender::end)
            .max()
            .unwrap_or(0)
    }

    /// Valida índices de roteamento (chamado antes de enviar ao núcleo).
    pub fn validate(&self) -> Result<(), String> {
        let nb = self.buses.len();
        for t in &self.tracks {
            if let Destination::Bus(b) = t.strip.dest {
                if b >= nb {
                    return Err(format!("faixa roteada para bus inexistente {b}"));
                }
            }
            if t.strip.sends.iter().any(|s| s.bus >= nb) {
                return Err("envio para bus inexistente".into());
            }
            for w in t.clips.windows(2) {
                if w[0].start > w[1].start {
                    return Err("clipes fora de ordem".into());
                }
            }
            for c in &t.clips {
                if c.offset + c.length > c.source.frames() {
                    return Err("clipe além do fim da fonte".into());
                }
            }
        }
        for b in &self.buses {
            if b.dest != Destination::Master || !b.sends.is_empty() {
                return Err("bus deve sair no master sem envios".into());
            }
        }
        if let Some((a, b)) = self.loop_range {
            if a >= b {
                return Err("loop inválido".into());
            }
        }
        let slots = 1 + self.tracks.len() + self.buses.len();
        if slots > crate::MAX_STRIPS {
            return Err("canais demais".into());
        }
        Ok(())
    }
}
