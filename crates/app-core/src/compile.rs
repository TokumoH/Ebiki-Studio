//! Compilador: domínio (Session) → grafo de renderização da engine.

use crate::ports::ProcessorFactory;
use crate::session::Session;
use audio_engine::*;
use ctx_mixing::{ChannelStrip, Output, StripTarget};
use shared_kernel::id::{AudioSourceId, TrackId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Identifica um efeito já presente no último grafo enviado (para herdar estado).
pub type EffectKey = (u128, u128, u32);

/// Índices de medidor por canal (para a UI).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeterMap {
    pub tracks: Vec<(TrackId, usize)>,
    pub buses: Vec<(ctx_mixing::BusId, usize)>,
}

pub struct Compiled {
    pub graph: RenderGraph,
    pub meters: MeterMap,
    pub effects: HashSet<EffectKey>,
}

/// Contexto da compilação de canais (efeitos já enviados e coletados).
struct StripBuilder<'a> {
    factory: &'a dyn ProcessorFactory,
    sample_rate: u32,
    previous: &'a HashSet<EffectKey>,
    effects: HashSet<EffectKey>,
}

impl StripBuilder<'_> {
    fn build(&mut self, key: u128, slot: usize, s: &ChannelStrip, audible: bool) -> StripRender {
        let mut r = StripRender::new(key, slot);
        r.volume = s.volume.to_linear();
        r.pan = s.pan.clamped().0;
        r.audible = audible;
        for slot in &s.inserts {
            let tag = self.factory.type_tag(&slot.effect);
            let ek = (key, slot.id.0.as_u128(), tag);
            let processor = if self.previous.contains(&ek) {
                None
            } else {
                Some(self.factory.create(&slot.effect, self.sample_rate))
            };
            self.effects.insert(ek);
            r.inserts.push(InsertRender {
                id: ek.1,
                type_tag: tag,
                bypass: slot.bypass,
                params: self.factory.params(&slot.effect),
                processor,
            });
        }
        r
    }
}

/// Compila a sessão. `previous` = efeitos do último grafo enviado (herdam processador).
pub fn compile(
    session: &Session,
    sources: &HashMap<AudioSourceId, Arc<SourceBuffer>>,
    factory: &dyn ProcessorFactory,
    previous: &HashSet<EffectKey>,
    input_channels: usize,
) -> Compiled {
    let p = &session.project;
    let m = &session.mixer;
    let sr = p.format.sample_rate.hz();
    let tempo = p.tempo();
    let mut b = StripBuilder {
        factory,
        sample_rate: sr,
        previous,
        effects: HashSet::new(),
    };
    let mut meters = MeterMap::default();
    let mut g = RenderGraph::empty(sr);
    g.tempo = tempo.clone();

    let bus_index: HashMap<_, _> = m
        .buses()
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let n_tracks = p.tracks().len();
    for (i, bus) in m.buses().iter().enumerate() {
        let slot = 1 + n_tracks + i;
        meters.buses.push((bus.id, slot));
        g.buses.push(b.build(
            bus.id.0.as_u128(),
            slot,
            &bus.strip,
            m.is_bus_audible(bus.id),
        ));
    }
    for (i, t) in p.tracks().iter().enumerate() {
        let slot = 1 + i;
        meters.tracks.push((t.id, slot));
        let default_strip = ChannelStrip::default();
        let s = m.strip(StripTarget::Track(t.id)).unwrap_or(&default_strip);
        let mut sr_strip = b.build(t.id.0.as_u128(), slot, s, m.is_track_audible(t.id));
        sr_strip.dest = match s.output {
            Output::Bus(bid) => bus_index
                .get(&bid)
                .map_or(Destination::Master, |i| Destination::Bus(*i)),
            Output::Master => Destination::Master,
        };
        for snd in &s.sends {
            if let Some(&bi) = bus_index.get(&snd.bus) {
                sr_strip.sends.push(SendRender {
                    bus: bi,
                    gain: snd.level.to_linear(),
                    pre_fader: snd.pre_fader,
                });
            }
        }
        let mut clips: Vec<ClipRender> = t
            .clips
            .iter()
            .filter_map(|c| {
                let src = sources.get(&c.source_id)?;
                if c.source_end() > src.frames() {
                    return None; // fonte decodificada menor que o esperado: ignora em vez de falhar
                }
                Some(clip_render(
                    c,
                    src.clone(),
                    tempo.tick_to_frame(c.start, sr),
                ))
            })
            .collect();
        clips.sort_by_key(|c| c.start);
        let rec = session.recording.track(t.id);
        let monitor = rec.input.filter(|_| rec.monitoring()).and_then(|i| {
            let left = i.left as usize;
            (left < input_channels).then(|| InputMonitor {
                left,
                right: i.right.map(|r| r as usize).filter(|r| *r < input_channels),
            })
        });
        g.tracks.push(TrackRender {
            strip: sr_strip,
            clips,
            monitor,
        });
    }
    g.master = b.build(0, 0, m.master(), !m.master().mute);
    let r = &session.recording;
    g.metronome = MetronomeSettings {
        enabled: r.metronome.enabled,
        volume: r.metronome.volume,
        count_in_bars: r.metronome.count_in_bars as u32,
        subdivision: r.metronome.subdivision as u32,
        accent_first_beat: r.metronome.accent_first_beat,
    };
    g.loop_range = r
        .loop_range()
        .map(|(a, b)| (tempo.tick_to_frame(a, sr), tempo.tick_to_frame(b, sr)))
        .filter(|(a, b)| a < b);
    g.punch = r
        .punch()
        .map(|(a, b)| (tempo.tick_to_frame(a, sr), tempo.tick_to_frame(b, sr)));
    Compiled {
        graph: g,
        meters,
        effects: b.effects,
    }
}

/// Converte um clipe do domínio no clipe da engine (ganho, fades, polaridade e reverso).
pub(crate) fn clip_render(
    c: &ctx_project::Clip,
    source: Arc<SourceBuffer>,
    start: u64,
) -> ClipRender {
    let e = &c.edit;
    ClipRender {
        fade_in: e.fade_in_frames,
        fade_out: e.fade_out_frames,
        fade_in_shape: e.fade_in_shape,
        fade_out_shape: e.fade_out_shape,
        invert: e.polarity_inverted,
        reverse: e.reversed,
        ..ClipRender::new(
            source,
            start,
            c.offset_frames,
            c.length_frames,
            c.gain.to_linear(),
        )
    }
}
