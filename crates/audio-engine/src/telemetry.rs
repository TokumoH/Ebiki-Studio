//! Telemetria lock-free publicada pela thread de áudio.

use crate::transport::TransportState;
use crate::MAX_STRIPS;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

/// Valores compartilhados entre a thread de áudio (escrita) e a de controle (leitura).
pub struct Telemetry {
    position: AtomicU64,
    state: AtomicU8,
    record_start: AtomicU64,
    xruns: AtomicU64,
    record_overflow: AtomicU64,
    /// Picos por canal (L, R) como bits de f32 positivos (ordem de u32 = ordem de f32 ≥ 0).
    peaks: Vec<[AtomicU32; 2]>,
}

/// Cópia dos valores para a UI.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySnapshot {
    pub position: u64,
    pub state: TransportState,
    pub record_start: Option<u64>,
    pub xruns: u64,
    pub record_overflow: u64,
    /// Picos desde a última leitura, indexados pelo `meter_slot` (0 = master).
    pub peaks: Vec<[f32; 2]>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            position: AtomicU64::new(0),
            state: AtomicU8::new(TransportState::Stopped as u8),
            record_start: AtomicU64::new(u64::MAX),
            xruns: AtomicU64::new(0),
            record_overflow: AtomicU64::new(0),
            peaks: (0..MAX_STRIPS)
                .map(|_| [AtomicU32::new(0), AtomicU32::new(0)])
                .collect(),
        }
    }
}

impl Telemetry {
    pub(crate) fn set_position(&self, p: u64) {
        self.position.store(p, Ordering::Relaxed);
    }

    pub(crate) fn set_state(&self, s: TransportState) {
        self.state.store(s as u8, Ordering::Release);
    }

    pub(crate) fn set_record_start(&self, frame: Option<u64>) {
        self.record_start
            .store(frame.unwrap_or(u64::MAX), Ordering::Release);
    }

    pub(crate) fn add_record_overflow(&self, n: u64) {
        self.record_overflow.fetch_add(n, Ordering::Relaxed);
    }

    /// Registra um xrun/sobrecarga (chamado pelo backend).
    pub fn add_xrun(&self) {
        self.xruns.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn push_peak(&self, slot: usize, l: f32, r: f32) {
        if let Some(p) = self.peaks.get(slot) {
            p[0].fetch_max(l.abs().min(f32::MAX).to_bits(), Ordering::Relaxed);
            p[1].fetch_max(r.abs().min(f32::MAX).to_bits(), Ordering::Relaxed);
        }
    }

    pub fn position(&self) -> u64 {
        self.position.load(Ordering::Relaxed)
    }

    pub fn record_start(&self) -> Option<u64> {
        let rs = self.record_start.load(Ordering::Acquire);
        (rs != u64::MAX).then_some(rs)
    }

    pub fn state(&self) -> TransportState {
        TransportState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Lê e zera os picos dos `slots` primeiros canais.
    pub fn snapshot(&self, slots: usize) -> TelemetrySnapshot {
        let n = slots.min(self.peaks.len());
        let peaks = self.peaks[..n]
            .iter()
            .map(|p| {
                [
                    f32::from_bits(p[0].swap(0, Ordering::Relaxed)),
                    f32::from_bits(p[1].swap(0, Ordering::Relaxed)),
                ]
            })
            .collect();
        let rs = self.record_start.load(Ordering::Acquire);
        TelemetrySnapshot {
            position: self.position(),
            state: self.state(),
            record_start: (rs != u64::MAX).then_some(rs),
            xruns: self.xruns.load(Ordering::Relaxed),
            record_overflow: self.record_overflow.load(Ordering::Relaxed),
            peaks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_keep_max_and_reset_on_read() {
        let t = Telemetry::default();
        t.push_peak(1, 0.5, -0.9);
        t.push_peak(1, 0.2, 0.1);
        let s = t.snapshot(2);
        assert_eq!(s.peaks[1], [0.5, 0.9]);
        assert_eq!(t.snapshot(2).peaks[1], [0.0, 0.0]);
        t.push_peak(MAX_STRIPS + 5, 1.0, 1.0); // fora do limite: ignorado sem pânico
    }
}
