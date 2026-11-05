//! Ciclo de vida do dispositivo de áudio: abre o hardware ou cai para o dispositivo nulo.

use app_core::Studio;
use audio_engine::device::{list_devices, DeviceHost, DeviceList, DeviceRequest};
use audio_engine::null_device::NullDevice;
use audio_engine::{create_engine, EngineConfig};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioStatus {
    /// "device" (hardware) ou "null" (sem som: hardware indisponível).
    pub mode: String,
    pub output_name: Option<String>,
    pub input_name: Option<String>,
    pub sample_rate: u32,
    pub buffer_size: Option<u32>,
    pub input_channels: u16,
    /// Latência estimada de ida e volta em ms (buffers de entrada + saída).
    pub estimated_latency_ms: Option<f64>,
    pub error: Option<String>,
}

enum Running {
    Device(DeviceHost),
    Null(NullDevice),
}

#[derive(Default)]
pub struct AudioRuntime {
    running: Option<Running>,
    pub request: DeviceRequest,
    pub status: AudioStatus,
}

impl AudioRuntime {
    pub fn devices() -> DeviceList {
        list_devices()
    }

    /// (Re)inicia o áudio com `request` e conecta a nova engine ao `studio`.
    pub fn start(&mut self, request: DeviceRequest, studio: &mut Studio) -> AudioStatus {
        self.stop(studio);
        self.request = request.clone();
        let (handle, core) = create_engine(EngineConfig {
            sample_rate: request.sample_rate,
            ..Default::default()
        });
        match DeviceHost::start(core, request.clone()) {
            Ok(host) => {
                let info = host.info.clone();
                let lat = info
                    .buffer_size
                    .map(|b| 2.0 * b as f64 * 1000.0 / info.sample_rate as f64);
                self.status = AudioStatus {
                    mode: "device".into(),
                    output_name: Some(info.output_name.clone()),
                    input_name: info.input_name.clone(),
                    sample_rate: info.sample_rate,
                    buffer_size: info.buffer_size,
                    input_channels: info.in_channels,
                    estimated_latency_ms: lat,
                    error: None,
                };
                let _ = studio.attach_engine(handle, info.in_channels as usize);
                self.running = Some(Running::Device(host));
            }
            Err(e) => {
                // sem hardware utilizável: mantém o app funcional com um dispositivo nulo
                let (handle, core) = create_engine(EngineConfig {
                    sample_rate: request.sample_rate,
                    ..Default::default()
                });
                let null = NullDevice::start(core, 512, 0, None);
                self.status = AudioStatus {
                    mode: "null".into(),
                    sample_rate: request.sample_rate,
                    buffer_size: Some(512),
                    error: Some(e.to_string()),
                    ..Default::default()
                };
                let _ = studio.attach_engine(handle, 0);
                self.running = Some(Running::Null(null));
            }
        }
        self.status.clone()
    }

    pub fn stop(&mut self, studio: &mut Studio) {
        studio.detach_engine();
        match self.running.take() {
            Some(Running::Device(d)) => d.stop(),
            Some(Running::Null(n)) => n.stop(),
            None => {}
        }
    }

    /// Garante a taxa de amostragem do projeto (reinicia o áudio se for diferente).
    pub fn ensure_rate(&mut self, sample_rate: u32, studio: &mut Studio) -> AudioStatus {
        if self.running.is_some() && self.request.sample_rate == sample_rate {
            return self.status.clone();
        }
        let req = DeviceRequest {
            sample_rate,
            ..self.request.clone()
        };
        self.start(req, studio)
    }
}
