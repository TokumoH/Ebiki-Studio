//! Mapa de tempo com BPM e fórmula de compasso variáveis.
//!
//! O tempo é constante entre eventos (sem rampas nesta versão). Toda conversão entre tempo musical
//! ([`Tick`]) e tempo real (segundos/amostras) passa por aqui.

use crate::time::{Tick, PPQ};
use serde::{Deserialize, Serialize};

pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 999.0;
const VALID_DENOMINATORS: [u8; 6] = [1, 2, 4, 8, 16, 32];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoEvent {
    pub tick: Tick,
    pub bpm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterEvent {
    pub tick: Tick,
    pub numerator: u8,
    pub denominator: u8,
}

impl MeterEvent {
    /// Duração de uma unidade de tempo (beat) em ticks.
    pub fn beat_ticks(&self) -> i64 {
        PPQ * 4 / self.denominator as i64
    }

    /// Duração de um compasso em ticks.
    pub fn bar_ticks(&self) -> i64 {
        self.beat_ticks() * self.numerator as i64
    }
}

/// Posição em compasso/tempo (1-based) com resto em ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarBeat {
    pub bar: i64,
    pub beat: i64,
    pub tick_in_beat: i64,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TempoError {
    #[error("BPM fora do intervalo {MIN_BPM}–{MAX_BPM}: {0}")]
    BpmOutOfRange(f64),
    #[error("posição negativa: {0}")]
    NegativeTick(i64),
    #[error("o evento no início (tick 0) não pode ser removido")]
    CannotRemoveInitial,
    #[error("não existe evento no tick {0}")]
    NotFound(i64),
    #[error("fórmula de compasso inválida: {0}/{1}")]
    InvalidMeter(u8, u8),
    #[error("mudança de compasso fora do início de um compasso (tick {0})")]
    MeterNotOnBarLine(i64),
    #[error("mapa de tempo sem evento inicial no tick 0")]
    MissingInitialEvent,
    #[error("eventos fora de ordem ou duplicados")]
    Unordered,
}

/// Mapa de tempo: invariantes garantidas por construção (ver [`TempoMap::validate`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoMap {
    tempos: Vec<TempoEvent>,
    meters: Vec<MeterEvent>,
}

impl Default for TempoMap {
    fn default() -> Self {
        Self::new(120.0, 4, 4).expect("valores padrão são válidos")
    }
}

impl TempoMap {
    pub fn new(bpm: f64, numerator: u8, denominator: u8) -> Result<Self, TempoError> {
        check_bpm(bpm)?;
        check_meter(numerator, denominator)?;
        Ok(Self {
            tempos: vec![TempoEvent {
                tick: Tick::ZERO,
                bpm,
            }],
            meters: vec![MeterEvent {
                tick: Tick::ZERO,
                numerator,
                denominator,
            }],
        })
    }

    /// Reconstrói a partir de eventos persistidos, validando todas as invariantes.
    pub fn from_events(
        mut tempos: Vec<TempoEvent>,
        mut meters: Vec<MeterEvent>,
    ) -> Result<Self, TempoError> {
        tempos.sort_by_key(|e| e.tick);
        meters.sort_by_key(|e| e.tick);
        let map = Self { tempos, meters };
        map.validate()?;
        Ok(map)
    }

    pub fn validate(&self) -> Result<(), TempoError> {
        if self.tempos.first().map(|e| e.tick) != Some(Tick::ZERO)
            || self.meters.first().map(|e| e.tick) != Some(Tick::ZERO)
        {
            return Err(TempoError::MissingInitialEvent);
        }
        for w in self.tempos.windows(2) {
            if w[0].tick >= w[1].tick {
                return Err(TempoError::Unordered);
            }
        }
        for e in &self.tempos {
            check_bpm(e.bpm)?;
        }
        for w in self.meters.windows(2) {
            if w[0].tick >= w[1].tick {
                return Err(TempoError::Unordered);
            }
        }
        for (i, m) in self.meters.iter().enumerate() {
            check_meter(m.numerator, m.denominator)?;
            if i > 0 {
                let prev = &self.meters[i - 1];
                if (m.tick.0 - prev.tick.0) % prev.bar_ticks() != 0 {
                    return Err(TempoError::MeterNotOnBarLine(m.tick.0));
                }
            }
        }
        Ok(())
    }

    pub fn tempos(&self) -> &[TempoEvent] {
        &self.tempos
    }

    pub fn meters(&self) -> &[MeterEvent] {
        &self.meters
    }

    /// BPM vigente em `tick`.
    pub fn bpm_at(&self, tick: Tick) -> f64 {
        self.tempos
            .iter()
            .rev()
            .find(|e| e.tick <= tick)
            .map_or(self.tempos[0].bpm, |e| e.bpm)
    }

    /// Fórmula de compasso vigente em `tick`.
    pub fn meter_at(&self, tick: Tick) -> MeterEvent {
        *self
            .meters
            .iter()
            .rev()
            .find(|e| e.tick <= tick)
            .unwrap_or(&self.meters[0])
    }

    /// Insere ou substitui um evento de tempo. Retorna o evento anterior no mesmo tick, se houver.
    pub fn set_tempo(&mut self, tick: Tick, bpm: f64) -> Result<Option<TempoEvent>, TempoError> {
        check_tick(tick)?;
        check_bpm(bpm)?;
        let ev = TempoEvent { tick, bpm };
        match self.tempos.binary_search_by_key(&tick, |e| e.tick) {
            Ok(i) => Ok(Some(std::mem::replace(&mut self.tempos[i], ev))),
            Err(i) => {
                self.tempos.insert(i, ev);
                Ok(None)
            }
        }
    }

    pub fn remove_tempo(&mut self, tick: Tick) -> Result<TempoEvent, TempoError> {
        if tick == Tick::ZERO {
            return Err(TempoError::CannotRemoveInitial);
        }
        let i = self
            .tempos
            .binary_search_by_key(&tick, |e| e.tick)
            .map_err(|_| TempoError::NotFound(tick.0))?;
        Ok(self.tempos.remove(i))
    }

    /// Insere ou substitui uma mudança de compasso (precisa cair numa barra de compasso
    /// e não pode desalinhar mudanças posteriores).
    pub fn set_meter(
        &mut self,
        tick: Tick,
        numerator: u8,
        denominator: u8,
    ) -> Result<Option<MeterEvent>, TempoError> {
        check_tick(tick)?;
        check_meter(numerator, denominator)?;
        let mut candidate = self.meters.clone();
        let ev = MeterEvent {
            tick,
            numerator,
            denominator,
        };
        let previous = match candidate.binary_search_by_key(&tick, |e| e.tick) {
            Ok(i) => Some(std::mem::replace(&mut candidate[i], ev)),
            Err(i) => {
                candidate.insert(i, ev);
                None
            }
        };
        let trial = Self {
            tempos: self.tempos.clone(),
            meters: candidate,
        };
        trial.validate()?;
        self.meters = trial.meters;
        Ok(previous)
    }

    pub fn remove_meter(&mut self, tick: Tick) -> Result<MeterEvent, TempoError> {
        if tick == Tick::ZERO {
            return Err(TempoError::CannotRemoveInitial);
        }
        let i = self
            .meters
            .binary_search_by_key(&tick, |e| e.tick)
            .map_err(|_| TempoError::NotFound(tick.0))?;
        let mut candidate = self.meters.clone();
        let removed = candidate.remove(i);
        let trial = Self {
            tempos: self.tempos.clone(),
            meters: candidate,
        };
        trial.validate()?;
        self.meters = trial.meters;
        Ok(removed)
    }

    /// Converte posição musical em segundos.
    pub fn tick_to_seconds(&self, tick: Tick) -> f64 {
        let t = tick.0.max(0);
        let mut seconds = 0.0;
        for (i, ev) in self.tempos.iter().enumerate() {
            if ev.tick.0 >= t {
                break;
            }
            let end = self.tempos.get(i + 1).map_or(t, |n| n.tick.0.min(t));
            seconds += (end - ev.tick.0) as f64 * seconds_per_tick(ev.bpm);
        }
        seconds
    }

    /// Converte segundos em posição musical (fracionária).
    pub fn seconds_to_ticks_f64(&self, seconds: f64) -> f64 {
        let s = seconds.max(0.0);
        let mut acc = 0.0;
        for (i, ev) in self.tempos.iter().enumerate() {
            let spt = seconds_per_tick(ev.bpm);
            match self.tempos.get(i + 1) {
                Some(next) => {
                    let seg = (next.tick.0 - ev.tick.0) as f64 * spt;
                    if acc + seg > s {
                        return ev.tick.0 as f64 + (s - acc) / spt;
                    }
                    acc += seg;
                }
                None => return ev.tick.0 as f64 + (s - acc) / spt,
            }
        }
        unreachable!("tempos nunca é vazio")
    }

    /// Converte segundos em tick (arredondado para o mais próximo).
    pub fn seconds_to_tick(&self, seconds: f64) -> Tick {
        Tick(self.seconds_to_ticks_f64(seconds).round() as i64)
    }

    /// Converte tick em índice de amostra (arredondado).
    pub fn tick_to_frame(&self, tick: Tick, sample_rate: u32) -> u64 {
        (self.tick_to_seconds(tick) * sample_rate as f64).round() as u64
    }

    /// Converte índice de amostra em tick (arredondado).
    pub fn frame_to_tick(&self, frame: u64, sample_rate: u32) -> Tick {
        self.seconds_to_tick(frame as f64 / sample_rate as f64)
    }

    /// Posição em compasso:tempo.
    pub fn bar_beat(&self, tick: Tick) -> BarBeat {
        let t = tick.0.max(0);
        let mut bar_base = 0i64;
        for (i, m) in self.meters.iter().enumerate() {
            let seg_end = self.meters.get(i + 1).map(|n| n.tick.0);
            if seg_end.is_some_and(|e| t >= e) {
                bar_base += (seg_end.unwrap() - m.tick.0) / m.bar_ticks();
                continue;
            }
            let rel = t - m.tick.0;
            let bars = rel / m.bar_ticks();
            let in_bar = rel % m.bar_ticks();
            return BarBeat {
                bar: bar_base + bars + 1,
                beat: in_bar / m.beat_ticks() + 1,
                tick_in_beat: in_bar % m.beat_ticks(),
            };
        }
        unreachable!("meters nunca é vazio")
    }

    /// Tick do início de um compasso (1-based).
    pub fn bar_start(&self, bar: i64) -> Tick {
        let target = bar.max(1) - 1;
        let mut bar_base = 0i64;
        for (i, m) in self.meters.iter().enumerate() {
            if let Some(next) = self.meters.get(i + 1) {
                let bars_in_seg = (next.tick.0 - m.tick.0) / m.bar_ticks();
                if target < bar_base + bars_in_seg {
                    return Tick(m.tick.0 + (target - bar_base) * m.bar_ticks());
                }
                bar_base += bars_in_seg;
            } else {
                return Tick(m.tick.0 + (target - bar_base) * m.bar_ticks());
            }
        }
        unreachable!("meters nunca é vazio")
    }
}

fn seconds_per_tick(bpm: f64) -> f64 {
    60.0 / (bpm * PPQ as f64)
}

fn check_bpm(bpm: f64) -> Result<(), TempoError> {
    if bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm) {
        Ok(())
    } else {
        Err(TempoError::BpmOutOfRange(bpm))
    }
}

fn check_meter(n: u8, d: u8) -> Result<(), TempoError> {
    if (1..=32).contains(&n) && VALID_DENOMINATORS.contains(&d) {
        Ok(())
    } else {
        Err(TempoError::InvalidMeter(n, d))
    }
}

fn check_tick(t: Tick) -> Result<(), TempoError> {
    if t.0 < 0 {
        Err(TempoError::NegativeTick(t.0))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn constant_tempo_conversion() {
        let m = TempoMap::default(); // 120 BPM
        assert!((m.tick_to_seconds(Tick::quarters(1)) - 0.5).abs() < 1e-12);
        assert_eq!(m.tick_to_frame(Tick::quarters(2), 48_000), 48_000);
        assert_eq!(m.frame_to_tick(24_000, 48_000), Tick::quarters(1));
    }

    #[test]
    fn variable_tempo_accumulates_segments() {
        let mut m = TempoMap::default();
        m.set_tempo(Tick::quarters(4), 60.0).unwrap(); // 4 semínimas a 120 = 2 s; depois 1 s por semínima
        assert!((m.tick_to_seconds(Tick::quarters(4)) - 2.0).abs() < 1e-12);
        assert!((m.tick_to_seconds(Tick::quarters(6)) - 4.0).abs() < 1e-12);
        assert_eq!(m.seconds_to_tick(3.0), Tick::quarters(5));
        assert_eq!(m.bpm_at(Tick::quarters(3)), 120.0);
        assert_eq!(m.bpm_at(Tick::quarters(4)), 60.0);
    }

    #[test]
    fn set_tempo_replaces_and_remove_protects_initial() {
        let mut m = TempoMap::default();
        assert_eq!(m.set_tempo(Tick::ZERO, 90.0).unwrap().unwrap().bpm, 120.0);
        assert_eq!(
            m.remove_tempo(Tick::ZERO),
            Err(TempoError::CannotRemoveInitial)
        );
        assert_eq!(m.remove_tempo(Tick(5)), Err(TempoError::NotFound(5)));
        assert!(m.set_tempo(Tick(10), 5.0).is_err());
        assert!(m.set_tempo(Tick(-1), 100.0).is_err());
        assert!(m.set_tempo(Tick(10), f64::NAN).is_err());
    }

    #[test]
    fn bar_beat_with_meter_change() {
        let mut m = TempoMap::default(); // 4/4: compasso = 3840 ticks
        m.set_meter(Tick(3840 * 2), 3, 4).unwrap(); // a partir do compasso 3, 3/4 (2880 ticks)
        assert_eq!(
            m.bar_beat(Tick::ZERO),
            BarBeat {
                bar: 1,
                beat: 1,
                tick_in_beat: 0
            }
        );
        assert_eq!(
            m.bar_beat(Tick(3840 + 960 + 10)),
            BarBeat {
                bar: 2,
                beat: 2,
                tick_in_beat: 10
            }
        );
        assert_eq!(
            m.bar_beat(Tick(7680 + 2880)),
            BarBeat {
                bar: 4,
                beat: 1,
                tick_in_beat: 0
            }
        );
        assert_eq!(m.bar_start(4), Tick(7680 + 2880));
        assert_eq!(m.bar_start(2), Tick(3840));
        assert_eq!(m.bar_start(1), Tick::ZERO);
    }

    #[test]
    fn eighth_note_denominator() {
        let m = TempoMap::new(120.0, 6, 8).unwrap();
        assert_eq!(m.meter_at(Tick::ZERO).bar_ticks(), 2880);
        assert_eq!(
            m.bar_beat(Tick(480 * 7)),
            BarBeat {
                bar: 2,
                beat: 2,
                tick_in_beat: 0
            }
        );
    }

    #[test]
    fn meter_must_land_on_bar_line_and_keep_later_changes_aligned() {
        let mut m = TempoMap::default();
        assert_eq!(
            m.set_meter(Tick(100), 3, 4),
            Err(TempoError::MeterNotOnBarLine(100))
        );
        m.set_meter(Tick(3840 * 2), 3, 4).unwrap();
        // mudar o compasso inicial para 3/4 desalinha a mudança no tick 7680 (7680 % 2880 != 0)
        assert_eq!(
            m.set_meter(Tick::ZERO, 3, 4),
            Err(TempoError::MeterNotOnBarLine(7680))
        );
        assert_eq!(m.meters()[0].numerator, 4, "estado não muda em erro");
        assert!(m.set_meter(Tick(0), 5, 3).is_err());
        assert_eq!(m.remove_meter(Tick(3840 * 2)).unwrap().numerator, 3);
    }

    #[test]
    fn from_events_validates() {
        assert_eq!(
            TempoMap::from_events(
                vec![TempoEvent {
                    tick: Tick(10),
                    bpm: 120.0
                }],
                vec![]
            ),
            Err(TempoError::MissingInitialEvent)
        );
        let ok = TempoMap::from_events(
            vec![
                TempoEvent {
                    tick: Tick(960),
                    bpm: 90.0,
                },
                TempoEvent {
                    tick: Tick::ZERO,
                    bpm: 120.0,
                },
            ],
            vec![MeterEvent {
                tick: Tick::ZERO,
                numerator: 4,
                denominator: 4,
            }],
        )
        .unwrap();
        assert_eq!(ok.tempos()[0].tick, Tick::ZERO);
    }

    proptest! {
        #[test]
        fn prop_roundtrip_tick_seconds(
            changes in proptest::collection::vec((1i64..200_000, 20.0f64..999.0), 0..8),
            t in 0i64..2_000_000,
        ) {
            let mut m = TempoMap::default();
            for (tick, bpm) in changes { m.set_tempo(Tick(tick), bpm).unwrap(); }
            let back = m.seconds_to_ticks_f64(m.tick_to_seconds(Tick(t)));
            prop_assert!((back - t as f64).abs() < 1e-6);
        }

        #[test]
        fn prop_monotonic(t1 in 0i64..1_000_000, d in 1i64..10_000, bpm in 20.0f64..999.0) {
            let mut m = TempoMap::default();
            m.set_tempo(Tick(5000), bpm).unwrap();
            prop_assert!(m.tick_to_seconds(Tick(t1 + d)) > m.tick_to_seconds(Tick(t1)));
        }

        #[test]
        fn prop_bar_start_inverts_bar_beat(bar in 1i64..500) {
            let mut m = TempoMap::default();
            m.set_meter(Tick(3840 * 4), 7, 8).unwrap();
            m.set_meter(Tick(3840 * 4 + 3360 * 3), 3, 4).unwrap();
            let bb = m.bar_beat(m.bar_start(bar));
            prop_assert_eq!(bb, BarBeat { bar, beat: 1, tick_in_beat: 0 });
        }
    }
}
