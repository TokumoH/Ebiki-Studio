//! Ciclo de vida do dispositivo de áudio: abre o hardware ou cai para o dispositivo nulo.

use app_core::Studio;
use audio_engine::device::{list_devices, DeviceHost, DeviceList, DeviceRequest};
use audio_engine::null_device::NullDevice;
use audio_engine::telemetry::{LOST_INPUT, LOST_OUTPUT};
use audio_engine::{create_engine, EngineConfig, Telemetry};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// De quanto em quanto tempo procurar um dispositivo que voltou (microfone/fone religado).
const RECONNECT_POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioStatus {
    /// "device" (hardware) ou "null" (sem som: hardware indisponível).
    pub mode: String,
    pub output_name: Option<String>,
    pub input_name: Option<String>,
    /// Dispositivos **pedidos** (ids de `audio_devices`); `None` = padrão do sistema / sem entrada.
    /// A UI usa para mostrar a escolha atual nos seletores.
    pub output_id: Option<String>,
    pub input_id: Option<String>,
    pub sample_rate: u32,
    pub buffer_size: Option<u32>,
    pub input_channels: u16,
    /// Latência estimada de ida e volta em ms (buffers de entrada + saída).
    pub estimated_latency_ms: Option<f64>,
    /// Ganho de entrada do microfone/interface (dB, 0..+24), aplicado antes de gravação e medidores.
    pub input_gain_db: f64,
    pub error: Option<String>,
    /// Problema contornado (microfone indisponível, fone substituído pelo padrão…): o áudio segue.
    pub warning: Option<String>,
    /// Tamanho real do callback do dispositivo (quadros), quando já rodou. Com `buffer_size = None`
    /// ("Padrão do driver") é ele que define a latência.
    pub callback_frames: Option<u32>,
}

/// Latência de ida e volta estimada (ms): entrada + saída, cada uma com `frames` quadros.
pub fn round_trip_ms(frames: u32, sample_rate: u32) -> f64 {
    2.0 * frames as f64 * 1000.0 / sample_rate.max(1) as f64
}

/// Por que o áudio foi reaberto sozinho (para avisar a pessoa).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reopen {
    OutputLost,
    InputLost,
    DeviceBack,
}

impl Reopen {
    pub fn message(self) -> &'static str {
        match self {
            Reopen::OutputLost => {
                "O fone/alto-falante foi desconectado: o áudio foi reaberto na saída disponível."
            }
            Reopen::InputLost => {
                "O microfone foi desconectado: o áudio segue sem entrada até ele voltar."
            }
            Reopen::DeviceBack => "Dispositivo de áudio reconectado: voltou a ser usado.",
        }
    }
}

/// O que faltou na última abertura (para procurar a volta do dispositivo).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Missing {
    pub input: bool,
    pub output: bool,
    /// Nenhuma saída abriu (modo silencioso).
    pub silent: bool,
}

/// Ids dos dispositivos presentes quando o áudio foi aberto (saídas, entradas), ordenados.
pub type DeviceIds = (Vec<String>, Vec<String>);

pub fn device_ids(l: &DeviceList) -> DeviceIds {
    let ids = |v: &[audio_engine::device::DeviceInfo]| {
        let mut i: Vec<String> = v.iter().map(|d| d.id.clone()).collect();
        i.sort();
        i
    };
    (ids(&l.outputs), ids(&l.inputs))
}

/// Decide se o áudio deve ser reaberto.
/// - `lost`: bits da telemetria (dispositivo sumiu com o stream aberto) → reabre sempre.
/// - Falta algo (microfone não abriu, fone trocado pelo padrão, modo silencioso): só reabre quando a
///   **lista de dispositivos mudou** desde a última abertura e o que faltava está presente. Um
///   microfone presente que não abre (privacidade do sistema, uso exclusivo, taxa) não entra em
///   laço de reaberturas: a lista não muda.
/// - `devices` só é consultado quando falta algo (listar dispositivos é lento).
pub fn reopen_reason(
    lost: u8,
    missing: Missing,
    request: &DeviceRequest,
    opened: &DeviceIds,
    devices: impl FnOnce() -> DeviceList,
) -> Option<Reopen> {
    if lost & LOST_OUTPUT != 0 {
        return Some(Reopen::OutputLost);
    }
    if lost & LOST_INPUT != 0 {
        return Some(Reopen::InputLost);
    }
    if !(missing.input || missing.output || missing.silent) {
        return None;
    }
    let list = devices();
    let now = device_ids(&list);
    if &now == opened {
        return None;
    }
    let has = |id: &Option<String>, l: &[String]| id.as_ref().is_some_and(|id| l.contains(id));
    let back = (missing.input && has(&request.input_id, &now.1))
        || (missing.output && has(&request.output_id, &now.0))
        || (missing.silent && !now.0.is_empty());
    back.then_some(Reopen::DeviceBack)
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
    /// Telemetria da engine em uso (o driver marca nela os dispositivos que sumiram).
    telemetry: Option<Arc<Telemetry>>,
    missing: Missing,
    last_poll: Option<Instant>,
    /// Dispositivos presentes na última abertura (para só reabrir quando a lista mudar).
    opened_ids: DeviceIds,
}

impl AudioRuntime {
    pub fn devices() -> DeviceList {
        list_devices()
    }

    /// (Re)inicia o áudio com `request` e conecta a nova engine ao `studio`.
    pub fn start(&mut self, request: DeviceRequest, studio: &mut Studio) -> AudioStatus {
        self.stop(studio);
        self.request = request.clone();
        self.opened_ids = device_ids(&list_devices());
        let (handle, core) = create_engine(EngineConfig {
            sample_rate: request.sample_rate,
            ..Default::default()
        });
        self.telemetry = Some(core.telemetry());
        match DeviceHost::start(core, request.clone()) {
            Ok(host) => {
                let info = host.info.clone();
                let lat = info.buffer_size.map(|b| round_trip_ms(b, info.sample_rate));
                self.status = AudioStatus {
                    mode: "device".into(),
                    output_name: Some(info.output_name.clone()),
                    input_name: info.input_name.clone(),
                    output_id: request.output_id.clone(),
                    input_id: request.input_id.clone(),
                    sample_rate: info.sample_rate,
                    buffer_size: info.buffer_size,
                    input_channels: info.in_channels,
                    estimated_latency_ms: lat,
                    input_gain_db: studio.input_gain_db(),
                    error: None,
                    warning: (!info.warnings.is_empty()).then(|| info.warnings.join(" ")),
                    callback_frames: None,
                };
                self.missing = Missing {
                    input: info.input_missing,
                    output: info.output_fallback,
                    silent: false,
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
                self.telemetry = Some(core.telemetry());
                self.missing = Missing {
                    silent: true,
                    ..Default::default()
                };
                let null = NullDevice::start(core, 512, 0, None);
                self.status = AudioStatus {
                    mode: "null".into(),
                    output_id: request.output_id.clone(),
                    input_id: request.input_id.clone(),
                    sample_rate: request.sample_rate,
                    buffer_size: Some(512),
                    input_gain_db: studio.input_gain_db(),
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

    /// Estado com o tamanho real do callback (lido da telemetria agora). Com o buffer no padrão do
    /// driver, a latência estimada passa a usar esse tamanho (antes ficava vazia ou errada — AU-04).
    pub fn live_status(&self) -> AudioStatus {
        let mut s = self.status.clone();
        let cf = self.telemetry.as_ref().and_then(|t| t.callback_frames());
        s.callback_frames = cf;
        if s.mode == "device" && s.buffer_size.is_none() {
            s.estimated_latency_ms = cf.map(|f| round_trip_ms(f, s.sample_rate));
        }
        s
    }

    /// Fase 1 (só a trava do áudio): algum dispositivo sumiu ou um que faltava voltou?
    pub fn check_reopen(&mut self) -> Option<Reopen> {
        self.running.as_ref()?;
        let lost = self.telemetry.as_ref().map_or(0, |t| t.take_device_lost());
        let poll_due = self.last_poll.is_none_or(|t| t.elapsed() >= RECONNECT_POLL);
        if lost == 0 && !poll_due {
            return None;
        }
        if poll_due {
            self.last_poll = Some(Instant::now());
        }
        let missing = if poll_due {
            self.missing
        } else {
            Missing::default()
        };
        reopen_reason(lost, missing, &self.request, &self.opened_ids, list_devices)
    }

    /// Fase 2 (travas do áudio e do estúdio): reabre. A volta de um dispositivo espera a gravação
    /// terminar (reabrir finalizaria o take); a perda reabre na hora (a gravação já parou de receber).
    pub fn reopen(&mut self, reason: Reopen, studio: &mut Studio) -> bool {
        if reason == Reopen::DeviceBack && studio.is_recording() {
            return false; // lista guardada não muda: tenta de novo no próximo ciclo
        }
        let req = self.request.clone();
        self.start(req, studio);
        true
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

#[cfg(test)]
mod reopen_tests {
    use super::*;
    use audio_engine::device::DeviceInfo;

    fn dev(id: &str) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: id.into(),
            is_default: false,
        }
    }
    fn list(outs: &[&str], ins: &[&str]) -> DeviceList {
        DeviceList {
            host: "Teste".into(),
            outputs: outs.iter().map(|i| dev(i)).collect(),
            inputs: ins.iter().map(|i| dev(i)).collect(),
        }
    }
    fn ids0() -> DeviceIds {
        device_ids(&list(&["alto-falante"], &[]))
    }
    fn req() -> DeviceRequest {
        DeviceRequest {
            output_id: Some("fone".into()),
            input_id: Some("mic".into()),
            buffer_size: None,
            sample_rate: 48_000,
        }
    }

    #[test]
    fn a_lost_device_always_reopens_without_listing() {
        let never = || -> DeviceList { panic!("não deveria listar") };
        assert_eq!(
            reopen_reason(LOST_OUTPUT, Missing::default(), &req(), &ids0(), never),
            Some(Reopen::OutputLost)
        );
        assert_eq!(
            reopen_reason(
                LOST_INPUT,
                Missing::default(),
                &req(),
                &ids0(),
                || unreachable!()
            ),
            Some(Reopen::InputLost)
        );
        assert_eq!(
            reopen_reason(
                LOST_INPUT | LOST_OUTPUT,
                Missing::default(),
                &req(),
                &ids0(),
                || unreachable!()
            ),
            Some(Reopen::OutputLost)
        );
    }

    #[test]
    fn nothing_missing_means_no_listing_and_no_reopen() {
        assert_eq!(
            reopen_reason(0, Missing::default(), &req(), &ids0(), || panic!(
                "não deveria listar"
            )),
            None
        );
    }

    #[test]
    fn reopens_when_the_missing_microphone_comes_back() {
        let m = Missing {
            input: true,
            ..Default::default()
        };
        assert_eq!(
            reopen_reason(0, m, &req(), &ids0(), || list(&["fone"], &[])),
            None,
            "ainda ausente"
        );
        assert_eq!(
            reopen_reason(0, m, &req(), &ids0(), || list(&["fone"], &["outro", "mic"])),
            Some(Reopen::DeviceBack)
        );
    }

    #[test]
    fn reopens_when_the_chosen_headphones_or_any_output_come_back() {
        let m = Missing {
            output: true,
            ..Default::default()
        };
        assert_eq!(
            reopen_reason(0, m, &req(), &ids0(), || list(&["alto-falante"], &[])),
            None
        );
        assert_eq!(
            reopen_reason(0, m, &req(), &ids0(), || list(&["fone"], &[])),
            Some(Reopen::DeviceBack)
        );
        let silent = Missing {
            silent: true,
            ..Default::default()
        };
        assert_eq!(
            reopen_reason(0, silent, &req(), &ids0(), || list(&[], &["mic"])),
            None,
            "sem saída nenhuma"
        );
        assert_eq!(
            reopen_reason(0, silent, &req(), &ids0(), || list(&["qualquer"], &[])),
            Some(Reopen::DeviceBack)
        );
    }
}

#[cfg(test)]
mod reopen_loop_tests {
    use super::*;
    use audio_engine::device::DeviceInfo;

    #[test]
    fn a_present_but_failing_microphone_does_not_reopen_in_a_loop() {
        // AU-02: o microfone aparece na lista mas não abre (privacidade/uso exclusivo)
        let l = DeviceList {
            host: "T".into(),
            outputs: vec![DeviceInfo {
                id: "fone".into(),
                name: "fone".into(),
                is_default: true,
            }],
            inputs: vec![DeviceInfo {
                id: "mic".into(),
                name: "mic".into(),
                is_default: true,
            }],
        };
        let opened = device_ids(&l);
        let req = DeviceRequest {
            output_id: None,
            input_id: Some("mic".into()),
            buffer_size: None,
            sample_rate: 48_000,
        };
        let m = Missing {
            input: true,
            ..Default::default()
        };
        for _ in 0..5 {
            assert_eq!(
                reopen_reason(0, m, &req, &opened, || l.clone()),
                None,
                "lista igual: não reabre"
            );
        }
        // modo silencioso com saída presente que não abre (taxa): também não entra em laço
        let silent = Missing {
            silent: true,
            ..Default::default()
        };
        assert_eq!(reopen_reason(0, silent, &req, &opened, || l.clone()), None);
    }
}

#[cfg(test)]
mod latency_tests {
    use super::*;

    #[test]
    fn round_trip_counts_input_and_output() {
        assert!((round_trip_ms(480, 48_000) - 20.0).abs() < 1e-9);
        assert!((round_trip_ms(256, 48_000) - 10.666_666).abs() < 1e-3);
        assert_eq!(round_trip_ms(10, 0), 20_000.0, "taxa 0 não divide por zero");
    }

    #[test]
    fn driver_default_buffer_uses_the_real_callback_size() {
        let (_h, core) = create_engine(EngineConfig::default());
        let tel = core.telemetry();
        let rt = AudioRuntime {
            status: AudioStatus {
                mode: "device".into(),
                sample_rate: 48_000,
                buffer_size: None,
                ..Default::default()
            },
            telemetry: Some(tel.clone()),
            ..Default::default()
        };
        assert_eq!(
            rt.live_status().estimated_latency_ms,
            None,
            "dispositivo ainda não rodou"
        );
        tel.set_callback_frames(480);
        let s = rt.live_status();
        assert_eq!(s.callback_frames, Some(480));
        assert!((s.estimated_latency_ms.unwrap() - 20.0).abs() < 1e-9);
        // buffer fixo: mantém a estimativa pelo buffer pedido
        let rt2 = AudioRuntime {
            status: AudioStatus {
                mode: "device".into(),
                sample_rate: 48_000,
                buffer_size: Some(256),
                estimated_latency_ms: Some(10.7),
                ..Default::default()
            },
            telemetry: Some(tel),
            ..Default::default()
        };
        assert_eq!(rt2.live_status().estimated_latency_ms, Some(10.7));
    }
}
