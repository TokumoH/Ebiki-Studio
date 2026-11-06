//! "Ouvir entrada" (monitor direto do dispositivo) — caso de uso da camada de aplicação.
//!
//! É configuração do **dispositivo**, não do projeto: vale para a sessão do app, começa desligado
//! (evita microfonia ao abrir) e é reaplicado quando a engine é trocada (mudança de dispositivo).
//! A soma do sinal acontece na engine, fora do master (não entra em gravação nem exportação).

use crate::{AppError, AppResult, Studio};
use audio_engine::DeviceMonitor;
use serde::{Deserialize, Serialize};

/// Faixa de ganho aceita (dB).
pub const MONITOR_GAIN_MIN_DB: f64 = -60.0;
pub const MONITOR_GAIN_MAX_DB: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMonitorView {
    /// Canal de entrada (0-based); `right: None` = mono nos dois lados.
    pub left: usize,
    pub right: Option<usize>,
    pub gain_db: f64,
}

impl DeviceMonitorView {
    fn engine_route(&self) -> DeviceMonitor {
        let gain = if self.gain_db <= MONITOR_GAIN_MIN_DB {
            0.0
        } else {
            10f64.powf(self.gain_db / 20.0) as f32
        };
        DeviceMonitor {
            left: self.left,
            right: self.right,
            gain,
        }
    }

    fn fits(&self, input_channels: usize) -> bool {
        self.left < input_channels && self.right.is_none_or(|r| r < input_channels)
    }
}

impl Studio {
    /// Configuração atual do "Ouvir entrada" (`None` = desligado).
    pub fn device_monitor(&self) -> Option<DeviceMonitorView> {
        self.device_monitor
    }

    /// Liga (com rota e ganho validados) ou desliga o "Ouvir entrada".
    pub fn set_device_monitor(
        &mut self,
        req: Option<DeviceMonitorView>,
    ) -> AppResult<Option<DeviceMonitorView>> {
        if self.engine.is_none() {
            return Err(AppError::new(
                "app.no_engine",
                "Engine de áudio indisponível.",
            ));
        }
        if let Some(r) = req {
            if !r.gain_db.is_finite() {
                return Err(AppError::new(
                    "app.invalid",
                    "Volume de monitoração inválido.",
                ));
            }
            if !r.fits(self.input_channels) {
                return Err(AppError::new(
                    "app.input_unavailable",
                    "A entrada escolhida não existe no dispositivo de áudio atual.",
                ));
            }
        }
        let req = req.map(|r| DeviceMonitorView {
            gain_db: r.gain_db.clamp(MONITOR_GAIN_MIN_DB, MONITOR_GAIN_MAX_DB),
            ..r
        });
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| AppError::new("app.no_engine", "Engine de áudio indisponível."))?;
        engine
            .set_device_monitor(req.map(|r| r.engine_route()))
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        self.device_monitor = req;
        Ok(req)
    }

    /// Depois de trocar a engine: reaplica a monitoração se a rota ainda existir no novo dispositivo.
    pub(crate) fn reapply_device_monitor(&mut self) {
        let Some(m) = self.device_monitor else {
            return;
        };
        if !m.fits(self.input_channels) {
            self.device_monitor = None;
            return;
        }
        if let Some(e) = self.engine.as_mut() {
            if e.set_device_monitor(Some(m.engine_route())).is_err() {
                self.device_monitor = None;
            }
        }
    }
}
