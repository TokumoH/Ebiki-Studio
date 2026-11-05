//! Comandos desfazíveis de gravação.

use crate::domain::*;
use shared_kernel::command::{Command, CommandError};
use shared_kernel::id::TrackId;
use shared_kernel::Tick;

type R = Result<(), CommandError>;

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Altera o estado de gravação de uma faixa (armar, entrada, monitoração, takes).
pub struct SetTrackRecording {
    pub track: TrackId,
    pub value: TrackRecording,
    previous: Option<TrackRecording>,
    label: &'static str,
}

impl SetTrackRecording {
    pub fn new(track: TrackId, value: TrackRecording, label: &'static str) -> Self {
        Self {
            track,
            value,
            previous: None,
            label,
        }
    }
}

impl Command<RecordingSetup> for SetTrackRecording {
    fn apply(&mut self, r: &mut RecordingSetup) -> R {
        self.previous = Some(r.set_track(self.track, self.value.clone())?);
        Ok(())
    }
    fn revert(&mut self, r: &mut RecordingSetup) -> R {
        r.set_track(self.track, self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        self.label
    }
}

pub struct SetPunch {
    pub value: Option<(Tick, Tick)>,
    previous: Option<Option<(Tick, Tick)>>,
}
impl SetPunch {
    pub fn new(value: Option<(Tick, Tick)>) -> Self {
        Self {
            value,
            previous: None,
        }
    }
}
impl Command<RecordingSetup> for SetPunch {
    fn apply(&mut self, r: &mut RecordingSetup) -> R {
        self.previous = Some(r.set_punch(self.value)?);
        Ok(())
    }
    fn revert(&mut self, r: &mut RecordingSetup) -> R {
        r.set_punch(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Punch in/out"
    }
}

pub struct SetLoop {
    pub value: Option<(Tick, Tick)>,
    previous: Option<Option<(Tick, Tick)>>,
}
impl SetLoop {
    pub fn new(value: Option<(Tick, Tick)>) -> Self {
        Self {
            value,
            previous: None,
        }
    }
}
impl Command<RecordingSetup> for SetLoop {
    fn apply(&mut self, r: &mut RecordingSetup) -> R {
        self.previous = Some(r.set_loop(self.value)?);
        Ok(())
    }
    fn revert(&mut self, r: &mut RecordingSetup) -> R {
        r.set_loop(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Região de loop"
    }
}

pub struct SetMetronome {
    pub value: MetronomeConfig,
    previous: Option<MetronomeConfig>,
}
impl SetMetronome {
    pub fn new(value: MetronomeConfig) -> Self {
        Self {
            value,
            previous: None,
        }
    }
}
impl Command<RecordingSetup> for SetMetronome {
    fn apply(&mut self, r: &mut RecordingSetup) -> R {
        self.previous = Some(r.set_metronome(self.value)?);
        Ok(())
    }
    fn revert(&mut self, r: &mut RecordingSetup) -> R {
        r.set_metronome(self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Metrônomo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_kernel::command::History;

    #[test]
    fn commands_roundtrip() {
        let mut r = RecordingSetup::default();
        let mut h = History::new();
        let t = TrackId::new();
        let cmds: Vec<Box<dyn Command<RecordingSetup>>> = vec![
            Box::new(SetTrackRecording::new(
                t,
                TrackRecording {
                    armed: true,
                    input: Some(InputAssignment {
                        left: 0,
                        right: None,
                    }),
                    ..Default::default()
                },
                "Armar",
            )),
            Box::new(SetPunch::new(Some((Tick(960), Tick(1920))))),
            Box::new(SetLoop::new(Some((Tick(0), Tick(3840))))),
            Box::new(SetMetronome::new(MetronomeConfig {
                enabled: true,
                ..Default::default()
            })),
        ];
        for c in cmds {
            let before = r.clone();
            h.execute(&mut r, c, None).unwrap();
            let after = r.clone();
            assert_ne!(before, after);
            h.undo(&mut r).unwrap();
            assert_eq!(r, before);
            h.redo(&mut r).unwrap();
            assert_eq!(r, after);
        }
    }
}
