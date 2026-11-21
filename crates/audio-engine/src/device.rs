//! Backend de dispositivo real (cpal: ASIO/WASAPI, CoreAudio, ALSA/JACK). Feature `device`.
//!
//! Os streams do cpal não são `Send` em todas as plataformas, então ficam numa thread própria
//! ([`DeviceHost`]); a aplicação conversa com ela por canal.

use crate::core::EngineCore;
use crate::resampler::{InputRateAdapter, OutputRateAdapter};
use crate::telemetry::{Telemetry, LOST_INPUT, LOST_OUTPUT};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::sync::mpsc;
use std::sync::Arc;
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
    /// Taxa do projeto (a engine sempre roda nela).
    pub sample_rate: u32,
    /// Taxa em que o fone/alto-falante abriu; diferente de `sample_rate` = conversão em tempo real.
    pub device_sample_rate: u32,
    /// Taxa em que o microfone abriu (`None` = sem entrada).
    pub input_device_rate: Option<u32>,
    pub buffer_size: Option<u32>,
    pub out_channels: u16,
    pub in_channels: u16,
    /// O microfone pedido não abriu (ausente ou incompatível): o áudio segue sem entrada.
    pub input_missing: bool,
    /// O fone pedido não foi encontrado: a saída usa o padrão do sistema.
    pub output_fallback: bool,
    /// Avisos para a pessoa (PT-BR), um por problema contornado.
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum DeviceError {
    #[error("nenhum dispositivo de saída de áudio encontrado")]
    NoOutput,
    #[error("dispositivo não encontrado: {0}")]
    NotFound(String),
    #[error(
        "o dispositivo não suporta {0} Hz; escolha outro dispositivo ou mude a taxa do projeto"
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

/// Formatos aceitos, do preferido ao menos preferido (o motor trabalha em f32; os inteiros são
/// convertidos no callback). Muitos microfones USB e placas no Linux não oferecem f32.
const FORMATS: [SampleFormat; 5] = [
    SampleFormat::F32,
    SampleFormat::I32,
    SampleFormat::I24,
    SampleFormat::I16,
    SampleFormat::U16,
];

/// Melhor configuração na taxa `rate`: formato mais preferido em [`FORMATS`] e, nele, o número de
/// canais igual a `want_channels` (senão o primeiro com mais canais, senão o maior disponível).
pub(crate) fn pick_config(
    configs: impl Iterator<Item = cpal::SupportedStreamConfigRange>,
    rate: u32,
    want_channels: u16,
) -> Option<cpal::SupportedStreamConfig> {
    let rank = |c: &cpal::SupportedStreamConfigRange| {
        let fmt = FORMATS.iter().position(|f| *f == c.sample_format())?;
        if c.min_sample_rate() > rate || c.max_sample_rate() < rate || c.channels() == 0 {
            return None;
        }
        // menor é melhor: formato, depois canais (exato < mais < menos)
        let ch = if c.channels() == want_channels {
            0
        } else if c.channels() > want_channels {
            1
        } else {
            2
        };
        Some((fmt, ch, u16::MAX - c.channels()))
    };
    configs
        .filter_map(|c| rank(&c).map(|r| (r, c)))
        .min_by_key(|(r, _)| *r)
        .map(|(_, c)| c.with_sample_rate(rate))
}

/// Taxas comuns, tentadas quando o dispositivo não aceita a do projeto.
const COMMON_RATES: [u32; 12] = [
    48_000, 44_100, 96_000, 88_200, 32_000, 192_000, 176_400, 24_000, 22_050, 16_000, 11_025, 8_000,
];

/// Taxa em que o dispositivo vai abrir: a do projeto, se alguma configuração utilizável (formato
/// conhecido, canais > 0) a aceita; senão a aceita mais próxima, preferindo as taxas comuns
/// ([`COMMON_RATES`]) às pontas de faixa incomuns e, no empate de distância, a maior. `None` = nenhuma
/// configuração utilizável. A diferença é convertida em tempo real ([`crate::resampler`]).
pub(crate) fn pick_rate(configs: &[cpal::SupportedStreamConfigRange], want: u32) -> Option<u32> {
    let usable: Vec<_> = configs
        .iter()
        .filter(|c| FORMATS.contains(&c.sample_format()) && c.channels() > 0)
        .collect();
    let accepts = |r: u32| {
        usable
            .iter()
            .any(|c| c.min_sample_rate() <= r && r <= c.max_sample_rate())
    };
    if accepts(want) {
        return Some(want);
    }
    let edges = usable
        .iter()
        .flat_map(|c| [c.min_sample_rate(), c.max_sample_rate()]);
    COMMON_RATES
        .iter()
        .copied()
        .chain(edges)
        .filter(|r| *r > 0 && accepts(*r))
        .min_by_key(|r| {
            (
                !COMMON_RATES.contains(r),
                r.abs_diff(want),
                std::cmp::Reverse(*r),
            )
        })
}

/// "44,1", "48", "22,05" (kHz no formato brasileiro).
fn khz(rate: u32) -> String {
    format!("{}", rate as f64 / 1000.0).replace('.', ",")
}

/// Aviso (PT-BR) de conversão de taxa em tempo real; `None` quando as taxas são iguais.
pub(crate) fn rate_warning(output: bool, device_rate: u32, project_rate: u32) -> Option<String> {
    (device_rate != project_rate).then(|| {
        if output {
            format!(
                "Fone a {} kHz: o áudio é convertido de {} kHz em tempo real.",
                khz(device_rate),
                khz(project_rate)
            )
        } else {
            format!(
                "Microfone a {} kHz: a entrada é convertida para {} kHz em tempo real.",
                khz(device_rate),
                khz(project_rate)
            )
        }
    })
}

/// Buffer pedido dentro da faixa que o dispositivo aceita (fora dela o stream não abre).
pub(crate) fn fit_buffer(range: &cpal::SupportedBufferSize, req: Option<u32>) -> cpal::BufferSize {
    match (req, range) {
        (None, _) => cpal::BufferSize::Default,
        (Some(b), cpal::SupportedBufferSize::Range { min, max }) => {
            cpal::BufferSize::Fixed(b.clamp(*min, (*max).max(*min)))
        }
        (Some(b), cpal::SupportedBufferSize::Unknown) => cpal::BufferSize::Fixed(b),
    }
}

/// Quadros por pedaço de conversão no callback (buffers pré-alocados, sem alocação no tempo real).
const CB_FRAMES: usize = 4096;

/// Callback de erro do driver: dispositivo sumiu → marca para a aplicação reabrir; resto → xrun.
fn on_stream_error(tel: Arc<Telemetry>, which: u8) -> impl FnMut(cpal::Error) + Send + 'static {
    move |e: cpal::Error| match e.kind() {
        cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::HostUnavailable => {
            tel.mark_device_lost(which)
        }
        _ => tel.add_xrun(),
    }
}

/// Stream de saída no formato nativo `T`: `render` preenche um rascunho f32, convertido para `T`.
fn build_output<T: SizedSample + FromSample<f32> + Send + 'static>(
    dev: &cpal::Device,
    config: cpal::StreamConfig,
    mut render: impl FnMut(&mut [f32]) + Send + 'static,
    err: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error> {
    let ch = (config.channels as usize).max(1);
    let mut buf = vec![0.0f32; CB_FRAMES * ch];
    dev.build_output_stream::<T, _, _>(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for chunk in data.chunks_mut(CB_FRAMES * ch) {
                let b = &mut buf[..chunk.len()];
                render(b);
                for (d, s) in chunk.iter_mut().zip(b.iter()) {
                    *d = T::from_sample(*s);
                }
            }
        },
        err,
        None,
    )
}

/// Stream de entrada no formato nativo `T`, convertido para f32 antes de `push`.
fn build_input<T: SizedSample + Send + 'static>(
    dev: &cpal::Device,
    config: cpal::StreamConfig,
    mut push: impl FnMut(&[f32]) + Send + 'static,
    err: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    f32: FromSample<T>,
{
    let ch = (config.channels as usize).max(1);
    let mut buf = vec![0.0f32; CB_FRAMES * ch];
    dev.build_input_stream::<T, _, _>(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            for chunk in data.chunks(CB_FRAMES * ch) {
                let b = &mut buf[..chunk.len()];
                for (d, s) in b.iter_mut().zip(chunk.iter()) {
                    *d = <f32 as FromSample<T>>::from_sample_(*s);
                }
                push(b);
            }
        },
        err,
        None,
    )
}

/// Escolhe o tipo de amostra do stream pelo formato da configuração.
macro_rules! by_format {
    ($fmt:expr, $f:ident, $($arg:expr),*) => {
        match $fmt {
            SampleFormat::F32 => $f::<f32>($($arg),*),
            SampleFormat::I32 => $f::<i32>($($arg),*),
            SampleFormat::I24 => $f::<cpal::I24>($($arg),*),
            SampleFormat::I16 => $f::<i16>($($arg),*),
            SampleFormat::U16 => $f::<u16>($($arg),*),
            other => Err(cpal::Error::with_message(
                cpal::ErrorKind::UnsupportedConfig,
                format!("formato {other:?} não suportado"),
            )),
        }
    };
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

/// Entrada aberta: stream (mantém o callback vivo), canais, nome e a fila lida pela saída.
struct OpenInput {
    stream: cpal::Stream,
    channels: usize,
    name: String,
    consumer: rtrb::Consumer<f32>,
    /// Taxa em que o microfone abriu (a fila sempre recebe a taxa do projeto).
    device_rate: u32,
}

/// Abre o microfone pedido. Falhar aqui **não** derruba o áudio: quem chama segue sem entrada.
fn open_input(
    h: &cpal::Host,
    id: &str,
    rate: u32,
    buffer: Option<u32>,
    telemetry: &Arc<Telemetry>,
) -> Result<OpenInput, String> {
    let dev =
        find(h, id, false).map_err(|_| "microfone não encontrado (desconectado?)".to_owned())?;
    let configs: Vec<_> = dev
        .supported_input_configs()
        .map_err(|e| e.to_string())?
        .collect();
    let no_rate = || format!("o microfone não funciona a {} kHz", khz(rate));
    let device_rate = pick_rate(&configs, rate).ok_or_else(no_rate)?;
    let cfg = pick_config(configs.into_iter(), device_rate, 2).ok_or_else(no_rate)?;
    let fmt = cfg.sample_format();
    let buffer_size = fit_buffer(cfg.buffer_size(), buffer);
    let mut config: cpal::StreamConfig = cfg.into();
    config.buffer_size = buffer_size;
    let ch = (config.channels as usize).max(1);
    let (mut prod, consumer) = rtrb::RingBuffer::<f32>::new(rate as usize * ch / 2);
    let tel = telemetry.clone();
    let mut write = move |data: &[f32]| {
        // só quadros inteiros: um corte no meio do quadro trocaria L/R para sempre
        let whole = (data.len().min(prod.slots()) / ch) * ch;
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
    };
    // microfone em outra taxa: converte para a do projeto antes da fila (gravação, medidores,
    // monitores e afinador sempre veem a taxa do projeto)
    let mut adapter =
        (device_rate != rate).then(|| InputRateAdapter::new(ch, device_rate, rate, CB_FRAMES));
    let push = move |data: &[f32]| match adapter.as_mut() {
        Some(a) => a.process(data, &mut write),
        None => write(data),
    };
    let err = on_stream_error(telemetry.clone(), LOST_INPUT);
    let stream =
        by_format!(fmt, build_input, &dev, config, push, err).map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(OpenInput {
        stream,
        channels: ch,
        name: dev.to_string(),
        consumer,
        device_rate,
    })
}

fn open_streams(
    mut core: EngineCore,
    req: &DeviceRequest,
) -> Result<(Vec<cpal::Stream>, RunningInfo), DeviceError> {
    let h = host();
    let mut warnings = Vec::new();
    // fone pedido que sumiu: usa o padrão do sistema em vez de ficar mudo
    let mut output_fallback = false;
    let out_dev = match req.output_id.as_deref().map(|id| find(&h, id, true)) {
        Some(Ok(d)) => d,
        Some(Err(_)) => {
            output_fallback = true;
            warnings.push(
                "O fone/alto-falante escolhido não foi encontrado: usando o padrão do sistema."
                    .to_owned(),
            );
            h.default_output_device().ok_or(DeviceError::NoOutput)?
        }
        None => h.default_output_device().ok_or(DeviceError::NoOutput)?,
    };
    let out_configs: Vec<_> = out_dev
        .supported_output_configs()
        .map_err(|e| DeviceError::Backend(e.to_string()))?
        .collect();
    // fone que não aceita a taxa do projeto (Bluetooth a 44,1/16 kHz…): abre na mais próxima e converte
    let device_rate = pick_rate(&out_configs, req.sample_rate)
        .ok_or(DeviceError::UnsupportedRate(req.sample_rate))?;
    let out_cfg = pick_config(out_configs.into_iter(), device_rate, 2)
        .ok_or(DeviceError::UnsupportedRate(req.sample_rate))?;
    warnings.extend(rate_warning(true, device_rate, req.sample_rate));
    let out_fmt = out_cfg.sample_format();
    let out_buffer = fit_buffer(out_cfg.buffer_size(), req.buffer_size);
    let mut out_config: cpal::StreamConfig = out_cfg.into();
    out_config.buffer_size = out_buffer;
    let out_ch = (out_config.channels as usize).max(1);
    let telemetry = core.telemetry();

    // entrada (opcional): se não abrir, o áudio segue sem ela — nunca derruba a saída
    let mut streams = Vec::new();
    let mut in_ch = 0usize;
    let mut input_name = None;
    let mut input_missing = false;
    let mut input_device_rate = None;
    let mut in_consumer: Option<rtrb::Consumer<f32>> = None;
    if let Some(id) = &req.input_id {
        match open_input(&h, id, req.sample_rate, req.buffer_size, &telemetry) {
            Ok(i) => {
                in_ch = i.channels;
                input_name = Some(i.name);
                in_consumer = Some(i.consumer);
                input_device_rate = Some(i.device_rate);
                warnings.extend(rate_warning(false, i.device_rate, req.sample_rate));
                streams.push(i.stream);
            }
            Err(why) => {
                input_missing = true;
                warnings.push(format!(
                    "Microfone indisponível ({why}): o áudio segue sem entrada."
                ));
            }
        }
    }

    let mut in_buf = vec![0.0f32; CB_FRAMES * in_ch.max(1)];
    let tel_out = telemetry.clone();
    // engine na taxa do projeto: `data` tem no máximo CB_FRAMES quadros (direto ou pelo conversor)
    let mut engine_render = move |data: &mut [f32]| {
        let frames = data.len() / out_ch;
        let mut n_in = 0;
        if let Some(c) = in_consumer.as_mut() {
            let want = (frames * in_ch).min(in_buf.len());
            // AU-03: o que se acumulou na fila (entrada aberta antes da saída, relógios diferentes)
            // vira latência fixa na monitoração e nos takes: acima de 2 callbacks, descarta o excesso
            let backlog = c.slots();
            if backlog > want * 2 {
                let drop = ((backlog - want) / in_ch.max(1)) * in_ch.max(1);
                if let Ok(chunk) = c.read_chunk(drop) {
                    chunk.commit_all();
                }
            }
            // múltiplo do número de canais, para não desalinhar L/R
            let avail = (c.slots().min(want) / in_ch.max(1)) * in_ch.max(1);
            if let Ok(chunk) = c.read_chunk(avail) {
                let (a, b) = chunk.as_slices();
                in_buf[..a.len()].copy_from_slice(a);
                in_buf[a.len()..a.len() + b.len()].copy_from_slice(b);
                chunk.commit_all();
            }
            // completa com silêncio se a entrada atrasou (ou parou: microfone desconectado)
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
    };
    // taxa igual: caminho direto (sem custo); diferente: engine → conversor → dispositivo
    let mut adapter = (device_rate != req.sample_rate)
        .then(|| OutputRateAdapter::new(out_ch, req.sample_rate, device_rate, CB_FRAMES));
    let dev_sr = device_rate as f64;
    let tel_cb = telemetry.clone();
    let render = move |data: &mut [f32]| {
        let started = Instant::now();
        // quadros e prazo do callback são os do dispositivo (latência e falhas de buffer)
        let frames = data.len() / out_ch;
        tel_cb.set_callback_frames(frames);
        match adapter.as_mut() {
            Some(a) => a.fill(data, &mut engine_render),
            None => engine_render(data),
        }
        if started.elapsed().as_secs_f64() > frames as f64 / dev_sr * 0.9 {
            tel_cb.add_xrun();
        }
    };
    let err = on_stream_error(telemetry, LOST_OUTPUT);
    let stream = by_format!(out_fmt, build_output, &out_dev, out_config, render, err)
        .map_err(|e| DeviceError::Backend(e.to_string()))?;
    stream
        .play()
        .map_err(|e| DeviceError::Backend(e.to_string()))?;
    streams.push(stream);
    let info = RunningInfo {
        output_name: out_dev.to_string(),
        input_name,
        sample_rate: req.sample_rate,
        device_sample_rate: device_rate,
        input_device_rate,
        buffer_size: match out_buffer {
            cpal::BufferSize::Fixed(b) => Some(b),
            cpal::BufferSize::Default => None,
        },
        out_channels: out_ch as u16,
        in_channels: in_ch as u16,
        input_missing,
        output_fallback,
        warnings,
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

    use cpal::{SupportedBufferSize, SupportedStreamConfigRange as R};

    fn r(ch: u16, min: u32, max: u32, fmt: SampleFormat) -> R {
        R::new(
            ch,
            min,
            max,
            SupportedBufferSize::Range { min: 64, max: 4096 },
            fmt,
        )
    }

    #[test]
    fn prefers_f32_but_accepts_integer_devices() {
        // microfone USB típico: só i16 (antes era recusado e derrubava o áudio inteiro)
        let only_i16 = vec![r(1, 44_100, 48_000, SampleFormat::I16)];
        let c = pick_config(only_i16.into_iter(), 48_000, 2).expect("aceito");
        assert_eq!(c.sample_format(), SampleFormat::I16);
        assert_eq!(c.channels(), 1);
        let mixed = vec![
            r(2, 48_000, 48_000, SampleFormat::I16),
            r(2, 48_000, 48_000, SampleFormat::F32),
            r(2, 48_000, 48_000, SampleFormat::I32),
        ];
        assert_eq!(
            pick_config(mixed.into_iter(), 48_000, 2)
                .unwrap()
                .sample_format(),
            SampleFormat::F32
        );
    }

    #[test]
    fn channels_exact_then_more_then_fewer() {
        let cfgs = || {
            vec![
                r(1, 48_000, 48_000, SampleFormat::F32),
                r(8, 48_000, 48_000, SampleFormat::F32),
                r(2, 48_000, 48_000, SampleFormat::F32),
            ]
            .into_iter()
        };
        assert_eq!(pick_config(cfgs(), 48_000, 2).unwrap().channels(), 2);
        let no_stereo = vec![
            r(1, 48_000, 48_000, SampleFormat::F32),
            r(8, 48_000, 48_000, SampleFormat::F32),
        ];
        assert_eq!(
            pick_config(no_stereo.into_iter(), 48_000, 2)
                .unwrap()
                .channels(),
            8
        );
    }

    #[test]
    fn rate_outside_range_or_unknown_format_is_refused() {
        let cfgs = vec![
            r(2, 44_100, 44_100, SampleFormat::F32),
            r(2, 48_000, 48_000, SampleFormat::U8),
        ];
        assert!(pick_config(cfgs.into_iter(), 48_000, 2).is_none());
    }

    #[test]
    fn rate_is_the_project_rate_when_supported() {
        let cfgs = vec![
            r(2, 44_100, 44_100, SampleFormat::F32),
            r(2, 8_000, 96_000, SampleFormat::I16),
        ];
        assert_eq!(pick_rate(&cfgs, 48_000), Some(48_000));
        assert_eq!(pick_rate(&cfgs, 44_100), Some(44_100));
    }

    #[test]
    fn unsupported_rate_picks_the_closest_supported_one() {
        // fone Bluetooth: só 44,1 kHz (antes: UnsupportedRate → modo silencioso)
        let bt = vec![r(2, 44_100, 44_100, SampleFormat::F32)];
        assert_eq!(pick_rate(&bt, 48_000), Some(44_100));
        // fone em modo chamada (microfone dele em uso): só 16 kHz, mono
        let hfp = vec![r(1, 16_000, 16_000, SampleFormat::I16)];
        assert_eq!(pick_rate(&hfp, 48_000), Some(16_000));
        // várias taxas fixas: a mais próxima do projeto
        let fixed = vec![
            r(2, 44_100, 44_100, SampleFormat::F32),
            r(2, 96_000, 96_000, SampleFormat::F32),
            r(2, 192_000, 192_000, SampleFormat::F32),
        ];
        assert_eq!(pick_rate(&fixed, 48_000), Some(44_100));
        assert_eq!(pick_rate(&fixed, 88_200), Some(96_000));
        // empate de distância (46 050 está a 1 950 de 44 100 e de 48 000): a maior
        let tie = vec![
            r(2, 44_100, 44_100, SampleFormat::F32),
            r(2, 48_000, 48_000, SampleFormat::F32),
        ];
        assert_eq!(pick_rate(&tie, 46_050), Some(48_000));
        // faixa contínua que não contém a taxa: taxa comum dentro da faixa, não a ponta incomum
        let range = vec![r(2, 8_000, 47_000, SampleFormat::F32)];
        assert_eq!(pick_rate(&range, 48_000), Some(44_100));
        // nenhuma taxa comum na faixa: a ponta mais próxima
        let odd = vec![r(2, 50_000, 60_000, SampleFormat::F32)];
        assert_eq!(pick_rate(&odd, 48_000), Some(50_000));
        // a taxa escolhida tem configuração (o resto da abertura segue igual)
        let c = pick_config(bt.into_iter(), 44_100, 2).expect("abre a 44,1 kHz");
        assert_eq!(c.sample_rate(), 44_100);
    }

    #[test]
    fn rate_ignores_unusable_configs() {
        // formato desconhecido ou 0 canais não contam (pick_config também as recusa)
        let cfgs = vec![
            r(2, 48_000, 48_000, SampleFormat::U8),
            r(0, 48_000, 48_000, SampleFormat::F32),
            r(2, 44_100, 44_100, SampleFormat::F32),
        ];
        assert_eq!(pick_rate(&cfgs, 48_000), Some(44_100));
        assert_eq!(pick_rate(&[], 48_000), None);
        assert_eq!(
            pick_rate(&[r(2, 48_000, 48_000, SampleFormat::U8)], 48_000),
            None
        );
    }

    #[test]
    fn conversion_warning_in_portuguese() {
        assert_eq!(rate_warning(true, 48_000, 48_000), None);
        assert_eq!(
            rate_warning(true, 44_100, 48_000).as_deref(),
            Some("Fone a 44,1 kHz: o áudio é convertido de 48 kHz em tempo real.")
        );
        assert_eq!(
            rate_warning(false, 16_000, 44_100).as_deref(),
            Some("Microfone a 16 kHz: a entrada é convertida para 44,1 kHz em tempo real.")
        );
        assert_eq!(khz(22_050), "22,05");
    }

    #[test]
    fn buffer_is_clamped_to_the_device_range() {
        let range = SupportedBufferSize::Range {
            min: 128,
            max: 1024,
        };
        assert_eq!(fit_buffer(&range, Some(32)), cpal::BufferSize::Fixed(128));
        assert_eq!(
            fit_buffer(&range, Some(4096)),
            cpal::BufferSize::Fixed(1024)
        );
        assert_eq!(fit_buffer(&range, Some(256)), cpal::BufferSize::Fixed(256));
        assert_eq!(fit_buffer(&range, None), cpal::BufferSize::Default);
        assert_eq!(
            fit_buffer(&SupportedBufferSize::Unknown, Some(256)),
            cpal::BufferSize::Fixed(256)
        );
    }
}
