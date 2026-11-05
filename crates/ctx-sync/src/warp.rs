//! Mapa de warp: liga quadros do áudio de origem a posições musicais (ticks) para
//! time-stretch e sincronização de instrumentos com a grade de BPM do projeto.
//!
//! Entre marcadores o mapeamento é linear; antes do primeiro e depois do último usa o
//! andamento nativo da fonte (`source_bpm`). Invariante: marcadores **estritamente crescentes**
//! em quadro e em tick — logo o mapeamento é estritamente monotônico e inversível.
//! `target` é relativo ao início do clipe.

use crate::error::SyncError;
use serde::{Deserialize, Serialize};
use shared_kernel::define_id;
use shared_kernel::tempo::{TempoMap, MAX_BPM, MIN_BPM};
use shared_kernel::time::{Tick, PPQ};

define_id!(
    /// Identificador de um mapa de warp.
    WarpMapId
);

/// Marcador de warp: quadro da fonte ↔ tick (relativo ao início do clipe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpMarker {
    pub source_frame: u64,
    pub target: Tick,
}

/// Mapa de warp de um clipe (agregado).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarpMap {
    pub id: WarpMapId,
    sample_rate: u32,
    source_bpm: f64,
    markers: Vec<WarpMarker>,
}

fn check_bpm(bpm: f64) -> Result<(), SyncError> {
    if bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm) {
        Ok(())
    } else {
        Err(SyncError::InvalidBpm(bpm))
    }
}

/// Razão de duração (saída/entrada) para tocar uma fonte de `source_bpm` em `target_bpm`.
/// Ex.: 100 → 120 BPM = 0,8333 (fica mais curta). Use com `dsp_pitch::time_stretch`.
pub fn stretch_ratio_for_tempo(source_bpm: f64, target_bpm: f64) -> Result<f64, SyncError> {
    check_bpm(source_bpm)?;
    check_bpm(target_bpm)?;
    Ok(source_bpm / target_bpm)
}

/// Segundos de uma posição fracionária em ticks.
fn seconds_at(tempo: &TempoMap, ticks: f64) -> f64 {
    let t = ticks.max(0.0);
    let base = Tick(t.floor() as i64);
    tempo.tick_to_seconds(base) + (t - base.0 as f64) * 60.0 / (tempo.bpm_at(base) * PPQ as f64)
}

impl WarpMap {
    pub fn new(sample_rate: u32, source_bpm: f64) -> Result<Self, SyncError> {
        if sample_rate == 0 {
            return Err(SyncError::InvalidSampleRate);
        }
        check_bpm(source_bpm)?;
        Ok(Self {
            id: WarpMapId::new(),
            sample_rate,
            source_bpm,
            markers: Vec::new(),
        })
    }

    /// Mapa a partir de batidas detectadas na fonte: a batida `i` vai para `i · beat_ticks`.
    pub fn from_beats(
        sample_rate: u32,
        source_bpm: f64,
        beat_frames: &[u64],
        beat_ticks: i64,
    ) -> Result<Self, SyncError> {
        let mut m = Self::new(sample_rate, source_bpm)?;
        if beat_ticks <= 0 {
            return Err(SyncError::NonMonotonicMarkers);
        }
        m.markers = beat_frames
            .iter()
            .enumerate()
            .map(|(i, f)| WarpMarker {
                source_frame: *f,
                target: Tick(i as i64 * beat_ticks),
            })
            .collect();
        m.validate()?;
        Ok(m)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn source_bpm(&self) -> f64 {
        self.source_bpm
    }

    pub fn markers(&self) -> &[WarpMarker] {
        &self.markers
    }

    /// Ticks por quadro no andamento nativo da fonte.
    pub fn native_ticks_per_frame(&self) -> f64 {
        self.source_bpm * PPQ as f64 / (60.0 * self.sample_rate as f64)
    }

    pub fn validate(&self) -> Result<(), SyncError> {
        if self.sample_rate == 0 {
            return Err(SyncError::InvalidSampleRate);
        }
        check_bpm(self.source_bpm)?;
        if self.markers.iter().any(|m| m.target.0 < 0)
            || self
                .markers
                .windows(2)
                .any(|w| w[0].source_frame >= w[1].source_frame || w[0].target >= w[1].target)
        {
            return Err(SyncError::NonMonotonicMarkers);
        }
        Ok(())
    }

    fn with_markers(&self, markers: Vec<WarpMarker>) -> Result<Vec<WarpMarker>, SyncError> {
        let trial = Self {
            id: self.id,
            sample_rate: self.sample_rate,
            source_bpm: self.source_bpm,
            markers,
        };
        trial.validate()?;
        Ok(trial.markers)
    }

    /// Insere um marcador (mantendo a ordem).
    pub fn add_marker(&mut self, marker: WarpMarker) -> Result<(), SyncError> {
        let mut markers = self.markers.clone();
        let i = markers.partition_point(|m| m.source_frame < marker.source_frame);
        markers.insert(i, marker);
        self.markers = self.with_markers(markers)?;
        Ok(())
    }

    /// Remove o marcador do quadro dado.
    pub fn remove_marker(&mut self, source_frame: u64) -> Result<WarpMarker, SyncError> {
        let i = self
            .markers
            .iter()
            .position(|m| m.source_frame == source_frame)
            .ok_or(SyncError::MarkerNotFound)?;
        Ok(self.markers.remove(i))
    }

    /// Move o destino de um marcador. Retorna o destino anterior.
    pub fn move_marker(&mut self, source_frame: u64, target: Tick) -> Result<Tick, SyncError> {
        let i = self
            .markers
            .iter()
            .position(|m| m.source_frame == source_frame)
            .ok_or(SyncError::MarkerNotFound)?;
        let mut markers = self.markers.clone();
        let previous = std::mem::replace(&mut markers[i].target, target);
        self.markers = self.with_markers(markers)?;
        Ok(previous)
    }

    /// Altera o andamento nativo da fonte. Retorna o anterior.
    pub fn set_source_bpm(&mut self, bpm: f64) -> Result<f64, SyncError> {
        check_bpm(bpm)?;
        Ok(std::mem::replace(&mut self.source_bpm, bpm))
    }

    /// Quadro da fonte → tick (fracionário) relativo ao clipe.
    pub fn frame_to_tick(&self, frame: f64) -> f64 {
        let tpf = self.native_ticks_per_frame();
        let (Some(first), Some(last)) = (self.markers.first(), self.markers.last()) else {
            return frame * tpf;
        };
        if frame <= first.source_frame as f64 {
            return first.target.0 as f64 + (frame - first.source_frame as f64) * tpf;
        }
        if frame >= last.source_frame as f64 {
            return last.target.0 as f64 + (frame - last.source_frame as f64) * tpf;
        }
        let i = self
            .markers
            .partition_point(|m| (m.source_frame as f64) <= frame)
            - 1;
        let (a, b) = (self.markers[i], self.markers[i + 1]);
        let u = (frame - a.source_frame as f64) / (b.source_frame - a.source_frame) as f64;
        a.target.0 as f64 + u * (b.target.0 - a.target.0) as f64
    }

    /// Tick relativo ao clipe → quadro da fonte (inversa de [`Self::frame_to_tick`]).
    pub fn tick_to_frame(&self, tick: f64) -> f64 {
        let tpf = self.native_ticks_per_frame();
        let (Some(first), Some(last)) = (self.markers.first(), self.markers.last()) else {
            return tick / tpf;
        };
        if tick <= first.target.0 as f64 {
            return first.source_frame as f64 + (tick - first.target.0 as f64) / tpf;
        }
        if tick >= last.target.0 as f64 {
            return last.source_frame as f64 + (tick - last.target.0 as f64) / tpf;
        }
        let i = self
            .markers
            .partition_point(|m| (m.target.0 as f64) <= tick)
            - 1;
        let (a, b) = (self.markers[i], self.markers[i + 1]);
        let u = (tick - a.target.0 as f64) / (b.target.0 - a.target.0) as f64;
        a.source_frame as f64 + u * (b.source_frame - a.source_frame) as f64
    }

    /// Razão local de time-stretch (duração de saída / duração de entrada) no quadro dado,
    /// considerando o mapa de tempo do projeto e a posição do clipe (`clip_start`).
    pub fn stretch_ratio_at(&self, frame: f64, tempo: &TempoMap, clip_start: Tick) -> f64 {
        let h = (self.sample_rate as f64 * 0.001).max(1.0);
        let t0 = clip_start.0 as f64 + self.frame_to_tick(frame);
        let t1 = clip_start.0 as f64 + self.frame_to_tick(frame + h);
        let out = seconds_at(tempo, t1) - seconds_at(tempo, t0);
        out / (h / self.sample_rate as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SR: u32 = 48_000;

    #[test]
    fn ratio_for_tempo() {
        assert!((stretch_ratio_for_tempo(100.0, 120.0).unwrap() - 0.833_333).abs() < 1e-6);
        assert_eq!(stretch_ratio_for_tempo(90.0, 90.0).unwrap(), 1.0);
        assert!(stretch_ratio_for_tempo(0.0, 120.0).is_err());
        assert!(stretch_ratio_for_tempo(120.0, f64::NAN).is_err());
    }

    #[test]
    fn no_markers_uses_native_tempo() {
        let w = WarpMap::new(SR, 120.0).unwrap();
        // 120 BPM: 1 semínima = 0,5 s = 24 000 quadros.
        assert!((w.frame_to_tick(24_000.0) - PPQ as f64).abs() < 1e-9);
        assert!((w.tick_to_frame(PPQ as f64 * 2.0) - 48_000.0).abs() < 1e-9);
        // Projeto a 120 BPM: razão 1. Projeto a 60 BPM: o dobro da duração.
        let tempo = TempoMap::default();
        assert!((w.stretch_ratio_at(10_000.0, &tempo, Tick::ZERO) - 1.0).abs() < 1e-6);
        let slow = TempoMap::new(60.0, 4, 4).unwrap();
        assert!((w.stretch_ratio_at(10_000.0, &slow, Tick::ZERO) - 2.0).abs() < 1e-6);
        assert!(WarpMap::new(0, 120.0).is_err());
        assert!(WarpMap::new(SR, 5.0).is_err());
    }

    #[test]
    fn markers_warp_segments() {
        // Fonte a 100 BPM com batidas levemente irregulares, alinhadas à grade de semínimas.
        let beats = [0, 28_800, 57_000, 86_400];
        let w = WarpMap::from_beats(SR, 100.0, &beats, PPQ).unwrap();
        assert_eq!(w.markers().len(), 4);
        assert!((w.frame_to_tick(57_000.0) - 2.0 * PPQ as f64).abs() < 1e-9);
        // Metade do 2º segmento.
        assert!((w.frame_to_tick(42_900.0) - 1.5 * PPQ as f64).abs() < 1e-9);
        assert!((w.tick_to_frame(1.5 * PPQ as f64) - 42_900.0).abs() < 1e-6);
        // Depois do último: andamento nativo (100 BPM → 0,6 s por semínima).
        assert!((w.frame_to_tick(86_400.0 + 28_800.0) - 4.0 * PPQ as f64).abs() < 1e-6);
        // Num projeto a 120 BPM, o 1º segmento (0,6 s → 0,5 s) tem razão 0,8333.
        let r = w.stretch_ratio_at(1000.0, &TempoMap::default(), Tick::ZERO);
        assert!((r - 0.833_333).abs() < 1e-4, "{r}");
        assert!(WarpMap::from_beats(SR, 100.0, &[10, 5], PPQ).is_err());
        assert!(WarpMap::from_beats(SR, 100.0, &[0, 5], 0).is_err());
    }

    #[test]
    fn edits_keep_monotonicity() {
        let mut w = WarpMap::new(SR, 120.0).unwrap();
        w.add_marker(WarpMarker {
            source_frame: 48_000,
            target: Tick(1920),
        })
        .unwrap();
        w.add_marker(WarpMarker {
            source_frame: 0,
            target: Tick(0),
        })
        .unwrap();
        assert_eq!(w.markers()[0].source_frame, 0);
        // Duplicado / cruzado.
        assert_eq!(
            w.add_marker(WarpMarker {
                source_frame: 48_000,
                target: Tick(3000)
            }),
            Err(SyncError::NonMonotonicMarkers)
        );
        assert_eq!(
            w.add_marker(WarpMarker {
                source_frame: 24_000,
                target: Tick(2000)
            }),
            Err(SyncError::NonMonotonicMarkers)
        );
        assert_eq!(
            w.move_marker(48_000, Tick(0)),
            Err(SyncError::NonMonotonicMarkers)
        );
        assert_eq!(w.move_marker(48_000, Tick(2400)).unwrap(), Tick(1920));
        assert_eq!(w.move_marker(7, Tick(1)), Err(SyncError::MarkerNotFound));
        assert_eq!(w.remove_marker(0).unwrap().target, Tick(0));
        assert_eq!(w.remove_marker(0), Err(SyncError::MarkerNotFound));
        assert_eq!(w.set_source_bpm(90.0).unwrap(), 120.0);
        assert!(w.set_source_bpm(2000.0).is_err());
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains("sourceFrame"));
        assert_eq!(serde_json::from_str::<WarpMap>(&json).unwrap(), w);
    }

    proptest! {
        /// Após qualquer sequência de edições, os marcadores seguem estritamente crescentes,
        /// o mapeamento é monotônico e a inversa recupera o quadro.
        #[test]
        fn warp_map_stays_monotonic(ops in prop::collection::vec((0u8..3, 0u64..200_000, 0i64..20_000), 1..40)) {
            let mut w = WarpMap::new(SR, 110.0).unwrap();
            for (op, frame, tick) in ops {
                let _ = match op {
                    0 => w.add_marker(WarpMarker { source_frame: frame, target: Tick(tick) }),
                    1 => {
                        let f = w.markers().get(frame as usize % (w.markers().len().max(1))).map(|m| m.source_frame);
                        f.map_or(Ok(()), |f| w.move_marker(f, Tick(tick)).map(|_| ()))
                    }
                    _ => {
                        let f = w.markers().first().map(|m| m.source_frame);
                        f.map_or(Ok(()), |f| w.remove_marker(f).map(|_| ()))
                    }
                };
                prop_assert!(w.validate().is_ok());
            }
            let mut prev = f64::NEG_INFINITY;
            for k in 0..=100u64 {
                let f = k as f64 * 2_100.0;
                let t = w.frame_to_tick(f);
                prop_assert!(t > prev);
                prev = t;
                prop_assert!((w.tick_to_frame(t) - f).abs() < 1e-6 * f.max(1.0));
            }
        }
    }
}
