//! Backend de dispositivo real (cpal: ASIO/WASAPI, CoreAudio, ALSA/JACK). Feature `device`.
//!
//! Os streams do cpal não são `Send` em todas as plataformas, então ficam numa thread própria
//! ([`DeviceHost`]); a aplicação conversa com ela por canal.

use crate::core::EngineCore;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceList {
    pub host: String,
    pub outputs: Vec<DeviceInfo>,
    pub inputs: Vec<DeviceInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceRequest {
    pub output_id: Option<String>,
    pub input_id: Option<String>,
    /// `None` = padrão do driver.
    pub buffer_size: Option<u32>,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningInfo {
    pub output_name: String,
    pub input_name: Option<String>,
    pub sample_rate: u32,
    pub buffer_size: Option<u32>,
    pub out_channels: u16,
    pub in_channels: u16,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum DeviceError {
    #[error("nenhum dispositivo de saída de áudio encontrado")]
    NoOutput,
    #[error("dispositivo não encontrado: {0}")]
    NotFound(String),
    #[error(
        "o dispositivo não suporta {0} Hz em ponto flutuante; escolha outra taxa ou dispositivo"
    )]
    UnsupportedRate(u32),
    #[error("falha no driver de áudio: {0}")]
    Backend(String),
}

fn host() -> cpal::Host {
    // No Windows, prefere ASIO quando disponível (menor latência).
    #[cfg(all(target_os = "windows", feature = "asio"))]
    if let Ok(h) = cpal::host_from_id(cpal::HostId::Asio) {
        return h;
    }
    cpal::default_host()
}

fn info(d: &cpal::Device, default_id: &Option<String>) -> Option<DeviceInfo> {
    let id = d.id().ok()?.to_string();
    Some(DeviceInfo {
        name: d.to_string(),
        is_default: default_id.as_ref() == Some(&id),
        id,
    })
}

/// Lista dispositivos (nunca entra em pânico; sem dispositivos retorna listas vazias).
pub fn list_devices() -> DeviceList {
    let h = host();
    let def_out = h
        .default_output_device()
        .and_then(|d| d.id().ok())
        .map(|i| i.to_string());
    let def_in = h
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|i| i.to_string());
    let outputs = h
        .output_devices()
        .map(|it| it.filter_map(|d| info(&d, &def_out)).collect())
        .unwrap_or_default();
    let inputs = h
        .input_devices()
        .map(|it| it.filter_map(|d| info(&d, &def_in)).collect())
        .unwrap_or_default();
    DeviceList {
        host: format!("{:?}", h.id()),
        outputs,
        inputs,
    }
}

fn find(h: &cpal::Host, id: &str, output: bool) -> Result<cpal::Device, DeviceError> {
    let mut it: Box<dyn Iterator<Item = cpal::Device>> = if output {
        Box::new(
            h.output_devices()
                .map_err(|e| DeviceError::Backend(e.to_string()))?,
        )
    } else {
        Box::new(
            h.input_devices()
                .map_err(|e| DeviceError::Backend(e.to_string()))?,
        )
    };
    it.find(|d| d.id().map(|i| i.to_string() == id).unwrap_or(false))
        .ok_or_else(|| DeviceError::NotFound(id.to_owned()))
}

fn f32_config(
    configs: impl Iterator<Item = cpal::SupportedStreamConfigRange>,
    rate: u32,
    want_channels: u16,
) -> Option<cpal::SupportedStreamConfig> {
    let mut best: Option<cpal::SupportedStreamConfigRange> = None;
    for c in configs {
        if c.sample_format() != cpal::SampleFormat::F32
            || c.min_sample_rate() > rate
            || c.max_sample_rate() < rate
        {
            continue;
        }
        let better = match &best {
            None => true,
            Some(b) => {
                (c.channels() >= want_channels && b.channels() < want_channels)
                    || (c.channels() == want_channels)
            }
        };
        if better {
            best = Some(c);
        }
    }
    best.map(|c| c.with_sample_rate(rate))
}

/// Thread dona dos streams de áudio.
pub struct DeviceHost {
    stop: mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub info: RunningInfo,
}

impl DeviceHost {
    /// Abre os dispositivos e começa a chamar `core.process` no callback de saída.
    pub fn start(core: EngineCore, req: DeviceRequest) -> Result<Self, DeviceError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<RunningInfo, DeviceError>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("ebiki-audio-host".into())
            .spawn(move || match open_streams(core, &req) {
                Ok((streams, info)) => {
                    let _ = ready_tx.send(Ok(info));
                    let _ = stop_rx.recv();
                    drop(streams);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| DeviceError::Backend(e.to_string()))?;
        let info = ready_rx
            .recv()
            .map_err(|_| DeviceError::Backend("thread de áudio encerrou".into()))??;
        Ok(Self {
            stop: stop_tx,
            thread: Some(thread),
            info,
        })
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for DeviceHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn open_streams(
    mut core: EngineCore,
    req: &DeviceRequest,
) -> Result<(Vec<cpal::Stream>, RunningInfo), DeviceError> {
    let h = host();
    let out_dev = match &req.output_id {
        Some(id) => find(&h, id, true)?,
        None => h.default_output_device().ok_or(DeviceError::NoOutput)?,
    };
    let out_cfg = f32_config(
        out_dev
            .supported_output_configs()
            .map_err(|e| DeviceError::Backend(e.to_string()))?,
        req.sample_rate,
        2,
    )
    .ok_or(DeviceError::UnsupportedRate(req.sample_rate))?;
    let mut out_config: cpal::StreamConfig = out_cfg.into();
    if let Some(b) = req.buffer_size {
        out_config.buffer_size = cpal::BufferSize::Fixed(b);
    }
    let out_ch = out_config.channels as usize;
    let telemetry = core.telemetry();

    // entrada (opcional): callback empurra para fila lock-free; a saída consome
    let mut streams = Vec::new();
    let mut in_ch = 0usize;
    let mut input_name = None;
    let mut in_consumer: Option<rtrb::Consumer<f32>> = None;
    if let Some(id) = &req.input_id {
        let in_dev = find(&h, id, false)?;
        let cfg = f32_config(
            in_dev
                .supported_input_configs()
                .map_err(|e| DeviceError::Backend(e.to_string()))?,
            req.sample_rate,
            2,
        )
        .ok_or(DeviceError::UnsupportedRate(req.sample_rate))?;
        let mut in_config: cpal::StreamConfig = cfg.into();
        in_config.buffer_size = out_config.buffer_size;
        in_ch = in_config.channels as usize;
        let (mut prod, cons) = rtrb::RingBuffer::<f32>::new(req.sample_rate as usize * in_ch / 2);
        let tel = telemetry.clone();
        let stream = in_dev
            .build_input_stream(
                in_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // só quadros inteiros: um corte no meio do quadro trocaria L/R para sempre
                    let whole = (data.len().min(prod.slots()) / in_ch) * in_ch;
                    if let Ok(mut chunk) = prod.write_chunk_uninit(whole) {
                        let n = chunk.len();
                        let (a, b) = chunk.as_mut_slices();
                        for (dst, src) in a.iter_mut().chain(b.iter_mut()).zip(data) {
                            dst.write(*src);
                        }
                        // SAFETY: n posições escritas acima.
                        unsafe { chunk.commit(n) };
                        if n < data.len() {
                            tel.add_xrun();
                        }
                    }
                },
                move |_e| {},
                None,
            )
            .map_err(|e| DeviceError::Backend(e.to_string()))?;
        stream
            .play()
            .map_err(|e| DeviceError::Backend(e.to_string()))?;
        streams.push(stream);
        input_name = Some(in_dev.to_string());
        in_consumer = Some(cons);
    }

    let mut in_buf = vec![0.0f32; 16_384 * in_ch.max(1)];
    let sr = req.sample_rate as f64;
    let tel_out = telemetry.clone();
    let tel_err = telemetry;
    let stream = out_dev
        .build_output_stream(
            out_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let started = Instant::now();
                let frames = data.len() / out_ch.max(1);
                let mut n_in = 0;
                if let Some(c) = in_consumer.as_mut() {
                    let want = (frames * in_ch).min(in_buf.len());
                    // múltiplo do número de canais, para não desalinhar L/R
                    let avail = (c.slots().min(want) / in_ch.max(1)) * in_ch.max(1);
                    if let Ok(chunk) = c.read_chunk(avail) {
                        let (a, b) = chunk.as_slices();
                        in_buf[..a.len()].copy_from_slice(a);
                        in_buf[a.len()..a.len() + b.len()].copy_from_slice(b);
                        chunk.commit_all();
                    }
                    // completa com silêncio se a entrada atrasou
                    in_buf[avail..want].fill(0.0);
                    n_in = want;
                }
                // um pânico aqui encerraria o stream: em caso de falha, silêncio + registro
                let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    core.process(data, out_ch, &in_buf[..n_in], in_ch);
                }));
                if ok.is_err() {
                    data.fill(0.0);
                    tel_out.add_xrun();
                }
                let budget = frames as f64 / sr;
                if started.elapsed().as_secs_f64() > budget * 0.9 {
                    tel_out.add_xrun();
                }
            },
            move |_e| tel_err.add_xrun(),
            None,
        )
        .map_err(|e| DeviceError::Backend(e.to_string()))?;
    stream
        .play()
        .map_err(|e| DeviceError::Backend(e.to_string()))?;
    streams.push(stream);
    let info = RunningInfo {
        output_name: out_dev.to_string(),
        input_name,
        sample_rate: req.sample_rate,
        buffer_size: req.buffer_size,
        out_channels: out_ch as u16,
        in_channels: in_ch as u16,
    };
    Ok((streams, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_never_panics_even_without_hardware() {
        let l = list_devices();
        assert!(!l.host.is_empty());
    }
}
