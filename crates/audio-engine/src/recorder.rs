//! Gravação: o núcleo copia a entrada para filas lock-free; threads de disco gravam WAV float 32-bit.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Uma faixa armada para gravação (lado tempo real).
pub struct RecordTap {
    pub key: u128,
    pub left: usize,
    pub right: Option<usize>,
    pub producer: rtrb::Producer<f32>,
}

impl RecordTap {
    pub fn channels(&self) -> u16 {
        if self.right.is_some() {
            2
        } else {
            1
        }
    }
}

/// Lado tempo real da gravação.
pub struct RecorderRt {
    pub taps: Vec<RecordTap>,
}

impl RecorderRt {
    /// Copia `frames` quadros da entrada intercalada. Retorna quantas amostras foram descartadas
    /// por fila cheia (disco lento).
    pub fn capture(&mut self, input: &[f32], in_channels: usize, frames: usize) -> u64 {
        let mut dropped = 0u64;
        if in_channels == 0 {
            return 0;
        }
        for tap in &mut self.taps {
            let per = tap.channels() as usize;
            let need = frames * per;
            let Ok(mut chunk) = tap
                .producer
                .write_chunk_uninit(need.min(tap.producer.slots()))
            else {
                dropped += need as u64;
                continue;
            };
            let writable = chunk.len() / per;
            let (a, b) = chunk.as_mut_slices();
            let mut idx = 0usize;
            for f in 0..writable {
                let base = f * in_channels;
                let l = input.get(base + tap.left).copied().unwrap_or(0.0);
                let mut put = |v: f32| {
                    if idx < a.len() {
                        a[idx].write(v);
                    } else {
                        b[idx - a.len()].write(v);
                    }
                    idx += 1;
                };
                put(l);
                if let Some(r) = tap.right {
                    put(input.get(base + r).copied().unwrap_or(0.0));
                }
            }
            // SAFETY: exatamente `writable * per` posições foram inicializadas acima.
            unsafe { chunk.commit(writable * per) };
            dropped += ((frames - writable) * per) as u64;
        }
        dropped
    }
}

/// Configuração de uma faixa a gravar.
#[derive(Debug, Clone)]
pub struct TapConfig {
    pub key: u128,
    pub left: usize,
    pub right: Option<usize>,
    pub path: PathBuf,
}

/// Arquivo gravado.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedFile {
    pub key: u128,
    pub path: PathBuf,
    pub channels: u16,
    pub frames: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("falha ao gravar arquivo de áudio: {0}")]
    Io(String),
}

/// Lado de controle da gravação: threads que esvaziam as filas no disco.
pub struct RecordingSession {
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<Result<RecordedFile, RecordError>>>,
}

impl RecordingSession {
    /// Cria as filas (≈ `buffer_seconds` de folga) e inicia as threads de escrita.
    pub fn start(
        taps: &[TapConfig],
        sample_rate: u32,
        buffer_seconds: u32,
    ) -> Result<(RecorderRt, Self), RecordError> {
        // `session` é criada antes das threads: se um tap falhar no meio, o `Drop`
        // encerra as threads já iniciadas e apaga os arquivos parciais.
        let mut session = Self {
            stop: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
        };
        let mut rt = Vec::new();
        for cfg in taps {
            let channels: u16 = if cfg.right.is_some() { 2 } else { 1 };
            let cap = (sample_rate as usize * buffer_seconds.max(1) as usize) * channels as usize;
            let (producer, consumer) = rtrb::RingBuffer::<f32>::new(cap);
            let spec = hound::WavSpec {
                channels,
                sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            if let Some(dir) = cfg.path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| RecordError::Io(e.to_string()))?;
            }
            let writer = hound::WavWriter::create(&cfg.path, spec)
                .map_err(|e| RecordError::Io(e.to_string()))?;
            let stop_flag = session.stop.clone();
            let key = cfg.key;
            let path = cfg.path.clone();
            let spawned = std::thread::Builder::new()
                .name("ebiki-rec".into())
                .spawn(move || write_loop(consumer, writer, stop_flag, key, path, channels));
            match spawned {
                Ok(h) => session.workers.push(h),
                Err(e) => {
                    let _ = std::fs::remove_file(&cfg.path);
                    return Err(RecordError::Io(e.to_string()));
                }
            }
            rt.push(RecordTap {
                key: cfg.key,
                left: cfg.left,
                right: cfg.right,
                producer,
            });
        }
        Ok((RecorderRt { taps: rt }, session))
    }

    /// Finaliza (depois que o núcleo devolveu o `RecorderRt`) e retorna os arquivos.
    ///
    /// Cada faixa é independente: a falha de uma não descarta os takes das outras.
    /// Um arquivo cuja escrita falhou já foi apagado pela própria thread.
    pub fn finish(mut self) -> (Vec<RecordedFile>, Vec<RecordError>) {
        self.stop.store(true, Ordering::Release);
        let mut files = Vec::new();
        let mut errors = Vec::new();
        for w in std::mem::take(&mut self.workers) {
            match w.join() {
                Ok(Ok(f)) => files.push(f),
                Ok(Err(e)) => errors.push(e),
                Err(_) => errors.push(RecordError::Io("thread de gravação falhou".into())),
            }
        }
        (files, errors)
    }
}

impl Drop for RecordingSession {
    /// Gravação abandonada (sem `finish`): encerra as threads e apaga os arquivos parciais.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for w in std::mem::take(&mut self.workers) {
            if let Ok(Ok(f)) = w.join() {
                let _ = std::fs::remove_file(&f.path);
            }
        }
    }
}

fn write_loop(
    mut consumer: rtrb::Consumer<f32>,
    mut writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    stop: Arc<AtomicBool>,
    key: u128,
    path: PathBuf,
    channels: u16,
) -> Result<RecordedFile, RecordError> {
    let r = write_samples(&mut consumer, &mut writer, &stop, channels);
    let r = r.and_then(|frames| {
        writer
            .finalize()
            .map_err(|e| RecordError::Io(e.to_string()))
            .map(|()| frames)
    });
    match r {
        Ok(frames) => Ok(RecordedFile {
            key,
            path,
            channels,
            frames,
        }),
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Err(e)
        }
    }
}

fn write_samples(
    consumer: &mut rtrb::Consumer<f32>,
    writer: &mut hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    stop: &AtomicBool,
    channels: u16,
) -> Result<u64, RecordError> {
    let mut samples = 0u64;
    loop {
        let stopping = stop.load(Ordering::Acquire);
        let n = consumer.slots();
        if n > 0 {
            let chunk = consumer
                .read_chunk(n)
                .map_err(|e| RecordError::Io(e.to_string()))?;
            let (a, b) = chunk.as_slices();
            for &s in a.iter().chain(b) {
                writer
                    .write_sample(s)
                    .map_err(|e| RecordError::Io(e.to_string()))?;
            }
            samples += n as u64;
            chunk.commit_all();
        } else if stopping {
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    Ok(samples / channels as u64)
}

/// Lê um WAV gravado como canais não intercalados (utilitário para testes e importação de takes).
pub fn read_wav_f32(path: &Path) -> Result<(u32, Vec<Vec<f32>>), RecordError> {
    let mut r = hound::WavReader::open(path).map_err(|e| RecordError::Io(e.to_string()))?;
    let spec = r.spec();
    let ch = spec.channels as usize;
    let mut out = vec![Vec::new(); ch];
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for (i, s) in r.samples::<f32>().enumerate() {
                out[i % ch].push(s.map_err(|e| RecordError::Io(e.to_string()))?);
            }
        }
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            for (i, s) in r.samples::<i32>().enumerate() {
                out[i % ch].push(s.map_err(|e| RecordError::Io(e.to_string()))? as f32 * scale);
            }
        }
    }
    Ok((spec.sample_rate, out))
}
