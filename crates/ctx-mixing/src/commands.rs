//! Comandos desfazíveis sobre o [`Mixer`].

use crate::domain::*;
use shared_kernel::command::{Command, CommandError};
use shared_kernel::id::TrackId;

type R = Result<(), CommandError>;

fn missing() -> CommandError {
    CommandError::new(
        "internal.revert_without_apply",
        "operação desfeita sem ter sido aplicada",
    )
}

/// Edição de um canal (volume, pan, mute, solo, efeitos, envios, saída).
pub struct EditStrip {
    pub target: StripTarget,
    pub edit: StripEdit,
    previous: Option<ChannelStrip>,
}

impl EditStrip {
    pub fn new(target: StripTarget, edit: StripEdit) -> Self {
        Self {
            target,
            edit,
            previous: None,
        }
    }
}

impl Command<Mixer> for EditStrip {
    fn apply(&mut self, m: &mut Mixer) -> R {
        self.previous = Some(m.edit(self.target, &self.edit)?);
        Ok(())
    }
    fn revert(&mut self, m: &mut Mixer) -> R {
        m.replace_strip(self.target, self.previous.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        match self.edit {
            StripEdit::Volume { .. } => "Volume",
            StripEdit::Pan { .. } => "Pan",
            StripEdit::Mute { .. } => "Mute",
            StripEdit::Solo { .. } => "Solo",
            StripEdit::AddEffect { .. } => "Adicionar efeito",
            StripEdit::RemoveEffect { .. } => "Remover efeito",
            StripEdit::MoveEffect { .. } => "Reordenar efeito",
            StripEdit::SetEffect { .. } => "Ajustar efeito",
            StripEdit::Bypass { .. } => "Bypass de efeito",
            StripEdit::Output { .. } => "Saída do canal",
            StripEdit::AddSend { .. } => "Adicionar envio",
            StripEdit::RemoveSend { .. } => "Remover envio",
            StripEdit::SendLevel { .. } => "Nível do envio",
        }
    }
}

/// Cria o canal de uma faixa.
pub struct AddStrip {
    pub track: TrackId,
    strip: ChannelStrip,
}
impl AddStrip {
    pub fn new(track: TrackId, strip: ChannelStrip) -> Self {
        Self { track, strip }
    }
}
impl Command<Mixer> for AddStrip {
    fn apply(&mut self, m: &mut Mixer) -> R {
        m.add_strip(self.track, self.strip.clone())?;
        Ok(())
    }
    fn revert(&mut self, m: &mut Mixer) -> R {
        m.remove_strip(self.track)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Criar canal"
    }
}

pub struct RemoveStrip {
    pub track: TrackId,
    removed: Option<ChannelStrip>,
}
impl RemoveStrip {
    pub fn new(track: TrackId) -> Self {
        Self {
            track,
            removed: None,
        }
    }
}
impl Command<Mixer> for RemoveStrip {
    fn apply(&mut self, m: &mut Mixer) -> R {
        self.removed = Some(m.remove_strip(self.track)?);
        Ok(())
    }
    fn revert(&mut self, m: &mut Mixer) -> R {
        m.add_strip(self.track, self.removed.take().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover canal"
    }
}

pub struct CreateBus {
    bus: Bus,
    index: usize,
}
impl CreateBus {
    pub fn new(bus: Bus, index: usize) -> Self {
        Self { bus, index }
    }
}
impl Command<Mixer> for CreateBus {
    fn apply(&mut self, m: &mut Mixer) -> R {
        m.add_bus(self.bus.clone(), self.index)?;
        Ok(())
    }
    fn revert(&mut self, m: &mut Mixer) -> R {
        m.remove_bus(self.bus.id)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Criar bus"
    }
}

pub struct DeleteBus {
    pub id: BusId,
    removed: Option<RemovedBus>,
}
impl DeleteBus {
    pub fn new(id: BusId) -> Self {
        Self { id, removed: None }
    }
}
impl Command<Mixer> for DeleteBus {
    fn apply(&mut self, m: &mut Mixer) -> R {
        self.removed = Some(m.remove_bus(self.id)?);
        Ok(())
    }
    fn revert(&mut self, m: &mut Mixer) -> R {
        let (bus, index, strips) = self.removed.take().ok_or_else(missing)?;
        m.add_bus(bus, index)?;
        m.restore_strips(strips);
        Ok(())
    }
    fn label(&self) -> &str {
        "Remover bus"
    }
}

pub struct RenameBus {
    pub id: BusId,
    pub name: String,
    previous: Option<String>,
}
impl RenameBus {
    pub fn new(id: BusId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            previous: None,
        }
    }
}
impl Command<Mixer> for RenameBus {
    fn apply(&mut self, m: &mut Mixer) -> R {
        self.previous = Some(m.rename_bus(self.id, &self.name)?);
        Ok(())
    }
    fn revert(&mut self, m: &mut Mixer) -> R {
        m.rename_bus(self.id, self.previous.as_deref().ok_or_else(missing)?)?;
        Ok(())
    }
    fn label(&self) -> &str {
        "Renomear bus"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::*;
    use shared_kernel::command::History;
    use shared_kernel::Db;

    fn roundtrip(m: &mut Mixer, c: Box<dyn Command<Mixer>>) {
        let before = m.clone();
        let mut h = History::new();
        h.execute(m, c, None).unwrap();
        let after = m.clone();
        assert_ne!(before, after);
        h.undo(m).unwrap();
        assert_eq!(*m, before);
        h.redo(m).unwrap();
        assert_eq!(*m, after);
    }

    #[test]
    fn all_mixer_commands_roundtrip() {
        let mut m = Mixer::new();
        let t = TrackId::new();
        roundtrip(&mut m, Box::new(AddStrip::new(t, ChannelStrip::default())));
        let b = Bus {
            id: BusId::new(),
            name: "Vozes".into(),
            strip: ChannelStrip::default(),
        };
        let bid = b.id;
        roundtrip(&mut m, Box::new(CreateBus::new(b.clone(), 0)));
        let tgt = StripTarget::Track(t);
        roundtrip(
            &mut m,
            Box::new(EditStrip::new(tgt, StripEdit::Volume { value: Db(-6.0) })),
        );
        roundtrip(
            &mut m,
            Box::new(EditStrip::new(
                tgt,
                StripEdit::Output {
                    value: Output::Bus(bid),
                },
            )),
        );
        m.edit(
            tgt,
            &StripEdit::AddEffect {
                slot: EffectSlot::new(Effect::Limiter(LimiterParams::default())),
                index: None,
            },
        )
        .unwrap();
        roundtrip(&mut m, Box::new(RenameBus::new(bid, "Coro")));
        roundtrip(&mut m, Box::new(DeleteBus::new(bid)));
        roundtrip(
            &mut m,
            Box::new(EditStrip::new(
                StripTarget::Master,
                StripEdit::Volume { value: Db(-1.0) },
            )),
        );
        roundtrip(&mut m, Box::new(RemoveStrip::new(t)));
    }
}
