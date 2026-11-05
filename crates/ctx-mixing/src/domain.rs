//! Mixer: canais por faixa, buses, master, sends e roteamento.
//!
//! Roteamento permitido (sem ciclos por construção): faixa → master | bus; bus → master;
//! sends: faixa → bus.

use crate::effects::{Effect, EffectId, EffectSlot};
use serde::{Deserialize, Serialize};
use shared_kernel::command::CommandError;
use shared_kernel::define_id;
use shared_kernel::id::TrackId;
use shared_kernel::{Db, Pan};
use std::collections::BTreeMap;

define_id!(
    /// Identificador de bus (grupo/retorno de efeito).
    BusId
);
define_id!(
    /// Identificador de envio auxiliar.
    SendId
);

pub const MAX_INSERTS: usize = 16;
pub const MAX_SENDS: usize = 8;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MixingError {
    #[error("canal não encontrado")]
    StripNotFound,
    #[error("bus não encontrado")]
    BusNotFound,
    #[error("efeito não encontrado")]
    EffectNotFound,
    #[error("envio não encontrado")]
    SendNotFound,
    #[error("este canal já existe")]
    DuplicateStrip,
    #[error("limite de efeitos por canal atingido ({MAX_INSERTS})")]
    TooManyInserts,
    #[error("limite de envios por canal atingido ({MAX_SENDS})")]
    TooManySends,
    #[error("roteamento inválido: {0}")]
    InvalidRouting(&'static str),
    #[error("nome de bus inválido")]
    InvalidName,
    #[error("o tipo do efeito não pode ser trocado")]
    EffectTypeMismatch,
}

impl MixingError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::StripNotFound => "mixing.strip_not_found",
            Self::BusNotFound => "mixing.bus_not_found",
            Self::EffectNotFound => "mixing.effect_not_found",
            Self::SendNotFound => "mixing.send_not_found",
            Self::DuplicateStrip => "mixing.duplicate_strip",
            Self::TooManyInserts => "mixing.too_many_inserts",
            Self::TooManySends => "mixing.too_many_sends",
            Self::InvalidRouting(_) => "mixing.invalid_routing",
            Self::InvalidName => "mixing.invalid_name",
            Self::EffectTypeMismatch => "mixing.effect_type_mismatch",
        }
    }
}

impl From<MixingError> for CommandError {
    fn from(e: MixingError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

/// Destino da saída de um canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum Output {
    #[default]
    Master,
    Bus(BusId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuxSend {
    pub id: SendId,
    pub bus: BusId,
    pub level: Db,
    pub pre_fader: bool,
}

/// Canal (faixa, bus ou master).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStrip {
    pub volume: Db,
    pub pan: Pan,
    pub mute: bool,
    pub solo: bool,
    pub inserts: Vec<EffectSlot>,
    pub sends: Vec<AuxSend>,
    pub output: Output,
}

impl Default for ChannelStrip {
    fn default() -> Self {
        Self {
            volume: Db::UNITY,
            pan: Pan::CENTER,
            mute: false,
            solo: false,
            inserts: Vec::new(),
            sends: Vec::new(),
            output: Output::Master,
        }
    }
}

impl ChannelStrip {
    pub fn effect_index(&self, id: EffectId) -> Result<usize, MixingError> {
        self.inserts
            .iter()
            .position(|e| e.id == id)
            .ok_or(MixingError::EffectNotFound)
    }

    pub fn send_index(&self, id: SendId) -> Result<usize, MixingError> {
        self.sends
            .iter()
            .position(|s| s.id == id)
            .ok_or(MixingError::SendNotFound)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bus {
    pub id: BusId,
    pub name: String,
    pub strip: ChannelStrip,
}

/// Qual canal uma edição atinge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum StripTarget {
    Track(TrackId),
    Bus(BusId),
    Master,
}

/// Edição atômica de um canal (alvo dos comandos de mixagem).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum StripEdit {
    Volume {
        value: Db,
    },
    Pan {
        value: Pan,
    },
    Mute {
        value: bool,
    },
    Solo {
        value: bool,
    },
    AddEffect {
        slot: EffectSlot,
        index: Option<usize>,
    },
    RemoveEffect {
        id: EffectId,
    },
    MoveEffect {
        id: EffectId,
        to: usize,
    },
    SetEffect {
        id: EffectId,
        effect: Effect,
    },
    Bypass {
        id: EffectId,
        value: bool,
    },
    Output {
        value: Output,
    },
    AddSend {
        send: AuxSend,
    },
    RemoveSend {
        id: SendId,
    },
    SendLevel {
        id: SendId,
        value: Db,
    },
}

/// Resultado da remoção de um bus: (bus, índice, canais afetados antes da remoção).
pub type RemovedBus = (Bus, usize, Vec<(TrackId, ChannelStrip)>);

/// Agregado raiz da mixagem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Mixer {
    strips: BTreeMap<TrackId, ChannelStrip>,
    buses: Vec<Bus>,
    master: ChannelStrip,
}

impl Mixer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn restore(
        strips: BTreeMap<TrackId, ChannelStrip>,
        buses: Vec<Bus>,
        master: ChannelStrip,
    ) -> Result<Self, MixingError> {
        let mut m = Self {
            strips: BTreeMap::new(),
            buses,
            master: ChannelStrip::default(),
        };
        for b in &m.buses {
            if !matches!(b.strip.output, Output::Master) || !b.strip.sends.is_empty() {
                return Err(MixingError::InvalidRouting(
                    "bus deve sair no master e não pode ter envios",
                ));
            }
        }
        // valida canais aplicando cada edição sobre canais padrão
        for (id, s) in strips {
            m.check_strip(&s, StripTarget::Track(id))?;
            m.strips.insert(id, s);
        }
        m.check_strip(&master, StripTarget::Master)?;
        m.master = master;
        Ok(m)
    }

    pub fn strips(&self) -> &BTreeMap<TrackId, ChannelStrip> {
        &self.strips
    }

    pub fn buses(&self) -> &[Bus] {
        &self.buses
    }

    pub fn master(&self) -> &ChannelStrip {
        &self.master
    }

    pub fn bus(&self, id: BusId) -> Result<&Bus, MixingError> {
        self.buses
            .iter()
            .find(|b| b.id == id)
            .ok_or(MixingError::BusNotFound)
    }

    pub fn strip(&self, target: StripTarget) -> Result<&ChannelStrip, MixingError> {
        match target {
            StripTarget::Track(id) => self.strips.get(&id).ok_or(MixingError::StripNotFound),
            StripTarget::Bus(id) => self.bus(id).map(|b| &b.strip),
            StripTarget::Master => Ok(&self.master),
        }
    }

    fn strip_mut(&mut self, target: StripTarget) -> Result<&mut ChannelStrip, MixingError> {
        match target {
            StripTarget::Track(id) => self.strips.get_mut(&id).ok_or(MixingError::StripNotFound),
            StripTarget::Bus(id) => self
                .buses
                .iter_mut()
                .find(|b| b.id == id)
                .map(|b| &mut b.strip)
                .ok_or(MixingError::BusNotFound),
            StripTarget::Master => Ok(&mut self.master),
        }
    }

    pub fn add_strip(&mut self, track: TrackId, strip: ChannelStrip) -> Result<(), MixingError> {
        if self.strips.contains_key(&track) {
            return Err(MixingError::DuplicateStrip);
        }
        self.check_strip(&strip, StripTarget::Track(track))?;
        self.strips.insert(track, strip);
        Ok(())
    }

    pub fn remove_strip(&mut self, track: TrackId) -> Result<ChannelStrip, MixingError> {
        self.strips.remove(&track).ok_or(MixingError::StripNotFound)
    }

    pub fn add_bus(&mut self, bus: Bus, index: usize) -> Result<(), MixingError> {
        let name = bus.name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err(MixingError::InvalidName);
        }
        if self.buses.iter().any(|b| b.id == bus.id) {
            return Err(MixingError::DuplicateStrip);
        }
        if !matches!(bus.strip.output, Output::Master) || !bus.strip.sends.is_empty() {
            return Err(MixingError::InvalidRouting(
                "bus deve sair no master e não pode ter envios",
            ));
        }
        let i = index.min(self.buses.len());
        self.buses.insert(
            i,
            Bus {
                name: name.to_owned(),
                ..bus
            },
        );
        Ok(())
    }

    /// Remove um bus. Faixas roteadas para ele voltam ao master e envios para ele são removidos.
    /// Retorna o bus, o índice e o estado anterior dos canais afetados (para desfazer).
    pub fn remove_bus(&mut self, id: BusId) -> Result<RemovedBus, MixingError> {
        let i = self
            .buses
            .iter()
            .position(|b| b.id == id)
            .ok_or(MixingError::BusNotFound)?;
        let mut affected = Vec::new();
        for (tid, s) in self.strips.iter_mut() {
            let uses = s.output == Output::Bus(id) || s.sends.iter().any(|x| x.bus == id);
            if uses {
                affected.push((*tid, s.clone()));
                if s.output == Output::Bus(id) {
                    s.output = Output::Master;
                }
                s.sends.retain(|x| x.bus != id);
            }
        }
        Ok((self.buses.remove(i), i, affected))
    }

    /// Restaura canais de faixa exatamente (usado ao desfazer remoção de bus).
    pub fn restore_strips(&mut self, strips: Vec<(TrackId, ChannelStrip)>) {
        for (id, s) in strips {
            self.strips.insert(id, s);
        }
    }

    pub fn rename_bus(&mut self, id: BusId, name: &str) -> Result<String, MixingError> {
        let n = name.trim();
        if n.is_empty() || n.chars().count() > 64 || n.chars().any(char::is_control) {
            return Err(MixingError::InvalidName);
        }
        let b = self
            .buses
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or(MixingError::BusNotFound)?;
        Ok(std::mem::replace(&mut b.name, n.to_owned()))
    }

    fn check_strip(&self, s: &ChannelStrip, target: StripTarget) -> Result<(), MixingError> {
        if s.inserts.len() > MAX_INSERTS {
            return Err(MixingError::TooManyInserts);
        }
        if s.sends.len() > MAX_SENDS {
            return Err(MixingError::TooManySends);
        }
        match target {
            StripTarget::Track(_) => {
                if let Output::Bus(b) = s.output {
                    self.bus(b)?;
                }
                for snd in &s.sends {
                    self.bus(snd.bus)?;
                }
            }
            StripTarget::Bus(_) | StripTarget::Master => {
                if s.output != Output::Master || !s.sends.is_empty() {
                    return Err(MixingError::InvalidRouting(
                        "apenas faixas podem ter envios e saída para bus",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Aplica uma edição num canal. Retorna o estado anterior do canal (para desfazer).
    pub fn edit(
        &mut self,
        target: StripTarget,
        edit: &StripEdit,
    ) -> Result<ChannelStrip, MixingError> {
        let mut s = self.strip(target)?.clone();
        let previous = s.clone();
        match edit {
            StripEdit::Volume { value } => s.volume = value.clamp_fader(),
            StripEdit::Pan { value } => s.pan = value.clamped(),
            StripEdit::Mute { value } => s.mute = *value,
            StripEdit::Solo { value } => {
                if target == StripTarget::Master {
                    return Err(MixingError::InvalidRouting("o master não tem solo"));
                }
                s.solo = *value
            }
            StripEdit::AddEffect { slot, index } => {
                if s.inserts.len() >= MAX_INSERTS {
                    return Err(MixingError::TooManyInserts);
                }
                let slot = EffectSlot {
                    effect: slot.effect.clamped(),
                    ..slot.clone()
                };
                let i = index.unwrap_or(usize::MAX).min(s.inserts.len());
                s.inserts.insert(i, slot);
            }
            StripEdit::RemoveEffect { id } => {
                let i = s.effect_index(*id)?;
                s.inserts.remove(i);
            }
            StripEdit::MoveEffect { id, to } => {
                let i = s.effect_index(*id)?;
                let e = s.inserts.remove(i);
                let to = (*to).min(s.inserts.len());
                s.inserts.insert(to, e);
            }
            StripEdit::SetEffect { id, effect } => {
                let i = s.effect_index(*id)?;
                if std::mem::discriminant(&s.inserts[i].effect) != std::mem::discriminant(effect) {
                    return Err(MixingError::EffectTypeMismatch);
                }
                s.inserts[i].effect = effect.clamped();
            }
            StripEdit::Bypass { id, value } => {
                let i = s.effect_index(*id)?;
                s.inserts[i].bypass = *value;
            }
            StripEdit::Output { value } => s.output = *value,
            StripEdit::AddSend { send } => {
                if s.sends.len() >= MAX_SENDS {
                    return Err(MixingError::TooManySends);
                }
                if s.sends.iter().any(|x| x.bus == send.bus) {
                    return Err(MixingError::InvalidRouting("já existe envio para este bus"));
                }
                s.sends.push(AuxSend {
                    level: send.level.clamp_fader(),
                    ..send.clone()
                });
            }
            StripEdit::RemoveSend { id } => {
                let i = s.send_index(*id)?;
                s.sends.remove(i);
            }
            StripEdit::SendLevel { id, value } => {
                let i = s.send_index(*id)?;
                s.sends[i].level = value.clamp_fader();
            }
        }
        self.check_strip(&s, target)?;
        *self.strip_mut(target)? = s;
        Ok(previous)
    }

    /// Restaura um canal inteiro (usado ao desfazer).
    pub fn replace_strip(
        &mut self,
        target: StripTarget,
        strip: ChannelStrip,
    ) -> Result<(), MixingError> {
        *self.strip_mut(target)? = strip;
        Ok(())
    }

    /// A faixa é audível considerando mute e solo? (solo em qualquer faixa silencia as não solo)
    pub fn is_track_audible(&self, track: TrackId) -> bool {
        let Some(s) = self.strips.get(&track) else {
            return false;
        };
        if s.mute {
            return false;
        }
        let any_solo =
            self.strips.values().any(|x| x.solo) || self.buses.iter().any(|b| b.strip.solo);
        if !any_solo {
            return true;
        }
        if s.solo {
            return true;
        }
        // faixa roteada para um bus em solo também soa
        matches!(s.output, Output::Bus(b) if self.bus(b).map(|b| b.strip.solo).unwrap_or(false))
    }

    /// O bus é audível? (um bus soa se não está mudo e: não há solo, ou ele está em solo,
    /// ou alguma faixa audível alimenta o bus)
    pub fn is_bus_audible(&self, bus: BusId) -> bool {
        let Ok(b) = self.bus(bus) else { return false };
        if b.strip.mute {
            return false;
        }
        let any_solo =
            self.strips.values().any(|x| x.solo) || self.buses.iter().any(|b| b.strip.solo);
        if !any_solo || b.strip.solo {
            return true;
        }
        self.strips.iter().any(|(id, s)| {
            (s.output == Output::Bus(bus) || s.sends.iter().any(|x| x.bus == bus))
                && s.solo
                && self.is_track_audible(*id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::*;

    fn bus(name: &str) -> Bus {
        Bus {
            id: BusId::new(),
            name: name.into(),
            strip: ChannelStrip::default(),
        }
    }

    #[test]
    fn edit_clamps_and_returns_previous() {
        let mut m = Mixer::new();
        let t = TrackId::new();
        m.add_strip(t, ChannelStrip::default()).unwrap();
        let prev = m
            .edit(
                StripTarget::Track(t),
                &StripEdit::Volume { value: Db(99.0) },
            )
            .unwrap();
        assert_eq!(prev.volume, Db::UNITY);
        assert_eq!(
            m.strip(StripTarget::Track(t)).unwrap().volume,
            Db(Db::FADER_MAX)
        );
        m.edit(
            StripTarget::Track(t),
            &StripEdit::Pan {
                value: Pan(f32::NAN),
            },
        )
        .unwrap();
        assert_eq!(m.strip(StripTarget::Track(t)).unwrap().pan, Pan::CENTER);
    }

    #[test]
    fn routing_rules() {
        let mut m = Mixer::new();
        let t = TrackId::new();
        m.add_strip(t, ChannelStrip::default()).unwrap();
        let ghost = BusId::new();
        assert_eq!(
            m.edit(
                StripTarget::Track(t),
                &StripEdit::Output {
                    value: Output::Bus(ghost)
                }
            ),
            Err(MixingError::BusNotFound)
        );
        let b = bus("Vozes");
        let bid = b.id;
        m.add_bus(b, 0).unwrap();
        m.edit(
            StripTarget::Track(t),
            &StripEdit::Output {
                value: Output::Bus(bid),
            },
        )
        .unwrap();
        assert!(
            m.edit(
                StripTarget::Bus(bid),
                &StripEdit::Output {
                    value: Output::Bus(bid)
                }
            )
            .is_err(),
            "bus → bus proibido"
        );
        let send = AuxSend {
            id: SendId::new(),
            bus: bid,
            level: Db(-10.0),
            pre_fader: false,
        };
        m.edit(
            StripTarget::Track(t),
            &StripEdit::AddSend { send: send.clone() },
        )
        .unwrap();
        assert!(
            m.edit(StripTarget::Track(t), &StripEdit::AddSend { send })
                .is_err(),
            "envio duplicado"
        );
        assert!(m
            .edit(StripTarget::Master, &StripEdit::Solo { value: true })
            .is_err());
    }

    #[test]
    fn removing_bus_reroutes_to_master_and_drops_sends() {
        let mut m = Mixer::new();
        let t = TrackId::new();
        m.add_strip(t, ChannelStrip::default()).unwrap();
        let b = bus("Reverb");
        let bid = b.id;
        m.add_bus(b, 0).unwrap();
        m.edit(
            StripTarget::Track(t),
            &StripEdit::Output {
                value: Output::Bus(bid),
            },
        )
        .unwrap();
        m.edit(
            StripTarget::Track(t),
            &StripEdit::AddSend {
                send: AuxSend {
                    id: SendId::new(),
                    bus: bid,
                    level: Db(0.0),
                    pre_fader: true,
                },
            },
        )
        .unwrap();
        let (_, _, affected) = m.remove_bus(bid).unwrap();
        let s = m.strip(StripTarget::Track(t)).unwrap();
        assert_eq!(s.output, Output::Master);
        assert!(s.sends.is_empty());
        assert_eq!(affected.len(), 1);
    }

    #[test]
    fn solo_and_mute_logic() {
        let mut m = Mixer::new();
        let (a, b) = (TrackId::new(), TrackId::new());
        m.add_strip(a, ChannelStrip::default()).unwrap();
        m.add_strip(b, ChannelStrip::default()).unwrap();
        assert!(m.is_track_audible(a) && m.is_track_audible(b));
        m.edit(StripTarget::Track(a), &StripEdit::Solo { value: true })
            .unwrap();
        assert!(m.is_track_audible(a) && !m.is_track_audible(b));
        m.edit(StripTarget::Track(a), &StripEdit::Mute { value: true })
            .unwrap();
        assert!(!m.is_track_audible(a), "mute vence solo");
        assert!(!m.is_track_audible(TrackId::new()));
    }

    #[test]
    fn soloed_track_keeps_its_bus_audible() {
        let mut m = Mixer::new();
        let (a, b) = (TrackId::new(), TrackId::new());
        m.add_strip(a, ChannelStrip::default()).unwrap();
        m.add_strip(b, ChannelStrip::default()).unwrap();
        let fx = bus("FX");
        let other = bus("Outro");
        let (fid, oid) = (fx.id, other.id);
        m.add_bus(fx, 0).unwrap();
        m.add_bus(other, 1).unwrap();
        m.edit(
            StripTarget::Track(a),
            &StripEdit::AddSend {
                send: AuxSend {
                    id: SendId::new(),
                    bus: fid,
                    level: Db(0.0),
                    pre_fader: false,
                },
            },
        )
        .unwrap();
        m.edit(StripTarget::Track(a), &StripEdit::Solo { value: true })
            .unwrap();
        assert!(m.is_bus_audible(fid));
        assert!(!m.is_bus_audible(oid));
    }

    #[test]
    fn effects_insert_move_set_bypass() {
        let mut m = Mixer::new();
        let t = TrackId::new();
        m.add_strip(t, ChannelStrip::default()).unwrap();
        let target = StripTarget::Track(t);
        let eq = EffectSlot::new(Effect::Eq(EqParams::default()));
        let comp = EffectSlot::new(Effect::Compressor(CompressorParams::default()));
        let (eqid, cid) = (eq.id, comp.id);
        m.edit(
            target,
            &StripEdit::AddEffect {
                slot: eq,
                index: None,
            },
        )
        .unwrap();
        m.edit(
            target,
            &StripEdit::AddEffect {
                slot: comp,
                index: Some(0),
            },
        )
        .unwrap();
        assert_eq!(m.strip(target).unwrap().inserts[0].id, cid);
        m.edit(target, &StripEdit::MoveEffect { id: cid, to: 9 })
            .unwrap();
        assert_eq!(m.strip(target).unwrap().inserts[1].id, cid);
        assert_eq!(
            m.edit(
                target,
                &StripEdit::SetEffect {
                    id: eqid,
                    effect: Effect::Gate(GateParams::default())
                }
            ),
            Err(MixingError::EffectTypeMismatch)
        );
        m.edit(
            target,
            &StripEdit::Bypass {
                id: eqid,
                value: true,
            },
        )
        .unwrap();
        assert!(m.strip(target).unwrap().inserts[0].bypass);
        for _ in 0..(MAX_INSERTS - 2) {
            m.edit(
                target,
                &StripEdit::AddEffect {
                    slot: EffectSlot::new(Effect::Chorus(ChorusParams::default())),
                    index: None,
                },
            )
            .unwrap();
        }
        assert_eq!(
            m.edit(
                target,
                &StripEdit::AddEffect {
                    slot: EffectSlot::new(Effect::Chorus(ChorusParams::default())),
                    index: None
                }
            ),
            Err(MixingError::TooManyInserts)
        );
    }
}
