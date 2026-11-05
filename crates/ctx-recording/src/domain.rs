//! Configuração de gravação e takes.

use serde::{Deserialize, Serialize};
use shared_kernel::command::CommandError;
use shared_kernel::define_id;
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::Tick;
use std::collections::BTreeMap;

define_id!(
    /// Identificador de take (uma passada de gravação).
    TakeId
);

pub const MAX_COUNT_IN_BARS: u8 = 4;
pub const MAX_INPUT_CHANNEL: u16 = 63;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RecordingError {
    #[error("intervalo de punch inválido (fim deve ser depois do início)")]
    InvalidPunch,
    #[error("canal de entrada inválido")]
    InvalidInput,
    #[error("take não encontrado")]
    TakeNotFound,
    #[error("valor fora da faixa permitida")]
    OutOfRange,
    #[error("já existe um take com este identificador")]
    DuplicateTake,
}

impl RecordingError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidPunch => "recording.invalid_punch",
            Self::InvalidInput => "recording.invalid_input",
            Self::TakeNotFound => "recording.take_not_found",
            Self::OutOfRange => "recording.out_of_range",
            Self::DuplicateTake => "recording.duplicate_take",
        }
    }
}

impl From<RecordingError> for CommandError {
    fn from(e: RecordingError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

/// Canais da interface de áudio usados por uma faixa (0-based). `right` None = mono.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputAssignment {
    pub left: u16,
    pub right: Option<u16>,
}

impl InputAssignment {
    pub fn validate(&self) -> Result<(), RecordingError> {
        let ok = self.left <= MAX_INPUT_CHANNEL
            && self
                .right
                .is_none_or(|r| r <= MAX_INPUT_CHANNEL && r != self.left);
        if ok {
            Ok(())
        } else {
            Err(RecordingError::InvalidInput)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MonitorMode {
    /// Sem monitoração por software.
    #[default]
    Off,
    /// Monitora sempre que a faixa estiver armada.
    WhenArmed,
    /// Monitora sempre.
    Always,
}

/// Estado de gravação de uma faixa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackRecording {
    pub armed: bool,
    pub input: Option<InputAssignment>,
    pub monitor: MonitorMode,
    pub takes: Vec<Take>,
    pub active_take: Option<TakeId>,
}

impl TrackRecording {
    pub fn monitoring(&self) -> bool {
        self.input.is_some()
            && matches!(
                (self.monitor, self.armed),
                (MonitorMode::Always, _) | (MonitorMode::WhenArmed, true)
            )
    }
}

/// Uma passada gravada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Take {
    pub id: TakeId,
    pub source_id: AudioSourceId,
    pub start: Tick,
    pub length_frames: u64,
    /// Número sequencial exibido ("Take 3").
    pub number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetronomeConfig {
    pub enabled: bool,
    pub volume: f32,
    pub subdivision: u8,
    pub accent_first_beat: bool,
    pub count_in_bars: u8,
}

impl Default for MetronomeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0.6,
            subdivision: 1,
            accent_first_beat: true,
            count_in_bars: 1,
        }
    }
}

impl MetronomeConfig {
    pub fn validate(&self) -> Result<(), RecordingError> {
        let ok = self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
            && [1, 2, 3, 4].contains(&self.subdivision)
            && self.count_in_bars <= MAX_COUNT_IN_BARS;
        if ok {
            Ok(())
        } else {
            Err(RecordingError::OutOfRange)
        }
    }
}

/// Agregado: configuração de gravação do projeto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSetup {
    pub metronome: MetronomeConfig,
    punch: Option<(Tick, Tick)>,
    loop_range: Option<(Tick, Tick)>,
    /// Latência de ida e volta compensada nas gravações (amostras).
    pub latency_compensation_frames: u32,
    tracks: BTreeMap<TrackId, TrackRecording>,
}

fn check_range(r: Option<(Tick, Tick)>) -> Result<(), RecordingError> {
    match r {
        Some((a, b)) if a.0 < 0 || b <= a => Err(RecordingError::InvalidPunch),
        _ => Ok(()),
    }
}

impl RecordingSetup {
    pub fn restore(
        metronome: MetronomeConfig,
        punch: Option<(Tick, Tick)>,
        loop_range: Option<(Tick, Tick)>,
        latency_compensation_frames: u32,
        tracks: BTreeMap<TrackId, TrackRecording>,
    ) -> Result<Self, RecordingError> {
        metronome.validate()?;
        check_range(punch)?;
        check_range(loop_range)?;
        for t in tracks.values() {
            if let Some(i) = t.input {
                i.validate()?;
            }
            if let Some(a) = t.active_take {
                if !t.takes.iter().any(|x| x.id == a) {
                    return Err(RecordingError::TakeNotFound);
                }
            }
        }
        Ok(Self {
            metronome,
            punch,
            loop_range,
            latency_compensation_frames,
            tracks,
        })
    }

    pub fn punch(&self) -> Option<(Tick, Tick)> {
        self.punch
    }

    pub fn loop_range(&self) -> Option<(Tick, Tick)> {
        self.loop_range
    }

    pub fn tracks(&self) -> &BTreeMap<TrackId, TrackRecording> {
        &self.tracks
    }

    pub fn track(&self, id: TrackId) -> TrackRecording {
        self.tracks.get(&id).cloned().unwrap_or_default()
    }

    pub fn armed_tracks(&self) -> impl Iterator<Item = (TrackId, &TrackRecording)> {
        self.tracks
            .iter()
            .filter(|(_, t)| t.armed && t.input.is_some())
            .map(|(id, t)| (*id, t))
    }

    /// Substitui o estado de uma faixa (retorna o anterior).
    pub fn set_track(
        &mut self,
        id: TrackId,
        t: TrackRecording,
    ) -> Result<TrackRecording, RecordingError> {
        if let Some(i) = t.input {
            i.validate()?;
        }
        let mut seen = std::collections::HashSet::new();
        if !t.takes.iter().all(|x| seen.insert(x.id)) {
            return Err(RecordingError::DuplicateTake);
        }
        if let Some(a) = t.active_take {
            if !t.takes.iter().any(|x| x.id == a) {
                return Err(RecordingError::TakeNotFound);
            }
        }
        let prev = self.track(id);
        if t == TrackRecording::default() {
            self.tracks.remove(&id);
        } else {
            self.tracks.insert(id, t);
        }
        Ok(prev)
    }

    pub fn set_punch(
        &mut self,
        r: Option<(Tick, Tick)>,
    ) -> Result<Option<(Tick, Tick)>, RecordingError> {
        check_range(r)?;
        Ok(std::mem::replace(&mut self.punch, r))
    }

    pub fn set_loop(
        &mut self,
        r: Option<(Tick, Tick)>,
    ) -> Result<Option<(Tick, Tick)>, RecordingError> {
        check_range(r)?;
        Ok(std::mem::replace(&mut self.loop_range, r))
    }

    pub fn set_metronome(&mut self, m: MetronomeConfig) -> Result<MetronomeConfig, RecordingError> {
        m.validate()?;
        Ok(std::mem::replace(&mut self.metronome, m))
    }

    /// Próximo número de take para a faixa.
    pub fn next_take_number(&self, id: TrackId) -> u32 {
        self.tracks
            .get(&id)
            .and_then(|t| t.takes.iter().map(|x| x.number).max())
            .unwrap_or(0)
            + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_and_ranges_are_validated() {
        assert!(InputAssignment {
            left: 0,
            right: Some(0)
        }
        .validate()
        .is_err());
        assert!(InputAssignment {
            left: 0,
            right: Some(1)
        }
        .validate()
        .is_ok());
        let mut r = RecordingSetup::default();
        assert_eq!(
            r.set_punch(Some((Tick(10), Tick(10)))),
            Err(RecordingError::InvalidPunch)
        );
        assert!(r.set_loop(Some((Tick(0), Tick(960)))).is_ok());
        assert!(r
            .set_metronome(MetronomeConfig {
                volume: 2.0,
                ..Default::default()
            })
            .is_err());
        assert!(r
            .set_metronome(MetronomeConfig {
                count_in_bars: 9,
                ..Default::default()
            })
            .is_err());
    }

    #[test]
    fn monitoring_rules() {
        let t = TrackRecording {
            armed: false,
            input: Some(InputAssignment {
                left: 0,
                right: None,
            }),
            monitor: MonitorMode::WhenArmed,
            ..Default::default()
        };
        assert!(!t.monitoring());
        assert!(TrackRecording {
            armed: true,
            ..t.clone()
        }
        .monitoring());
        assert!(TrackRecording {
            monitor: MonitorMode::Always,
            ..t.clone()
        }
        .monitoring());
        assert!(!TrackRecording {
            input: None,
            monitor: MonitorMode::Always,
            ..t
        }
        .monitoring());
    }

    #[test]
    fn takes_and_active_take_consistency() {
        let mut r = RecordingSetup::default();
        let tr = TrackId::new();
        let take = Take {
            id: TakeId::new(),
            source_id: AudioSourceId::new(),
            start: Tick(0),
            length_frames: 10,
            number: 1,
        };
        let bad = TrackRecording {
            active_take: Some(TakeId::new()),
            takes: vec![take.clone()],
            ..Default::default()
        };
        assert_eq!(r.set_track(tr, bad), Err(RecordingError::TakeNotFound));
        let dup = TrackRecording {
            takes: vec![take.clone(), take.clone()],
            ..Default::default()
        };
        assert_eq!(r.set_track(tr, dup), Err(RecordingError::DuplicateTake));
        r.set_track(
            tr,
            TrackRecording {
                active_take: Some(take.id),
                takes: vec![take],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.next_take_number(tr), 2);
        r.set_track(tr, TrackRecording::default()).unwrap();
        assert!(r.tracks().is_empty(), "estado padrão não é armazenado");
    }
}
