#![allow(dead_code)]
//! Dublês de teste para as portas (mídia WAV simples, fábrica de ganho, relógio controlável).
use app_core::ports::*;
use app_core::{Studio, StudioDeps};
use audio_engine::{AudioProcessor, SourceBuffer};
use ctx_mixing::Effect;
use infra_persistence::SqliteSessionRepository;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

pub const SR: u32 = 48_000;

pub fn write_wav(path: &Path, channels: Vec<Vec<f32>>) {
    let spec = hound::WavSpec {
        channels: channels.len() as u16,
        sample_rate: SR,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..channels[0].len() {
        for c in &channels {
            w.write_sample(c[i]).unwrap();
        }
    }
    w.finalize().unwrap();
}

fn fake_hash(bytes: &[u8]) -> String {
    let mut out = String::new();
    for salt in 0..4u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        salt.hash(&mut h);
        bytes.hash(&mut h);
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

pub struct WavMedia;

impl MediaPort for WavMedia {
    fn import(&self, src: &Path, root: &Path, _sr: u32) -> Result<ImportedMedia, PortError> {
        let bytes = std::fs::read(src).map_err(|e| PortError::new("codec.io", e.to_string()))?;
        let hash = fake_hash(&bytes);
        let rel = format!("audio/imported/{hash}.wav");
        std::fs::write(root.join(&rel), &bytes)
            .map_err(|e| PortError::new("codec.io", e.to_string()))?;
        let (_, ch) = audio_engine::recorder::read_wav_f32(src)
            .map_err(|e| PortError::new("codec.corrupt", e.to_string()))?;
        Ok(ImportedMedia {
            content_hash: hash,
            rel_path: rel,
            original_name: src.file_name().unwrap().to_string_lossy().into(),
            channels: ch.len() as u16,
            frames: ch[0].len() as u64,
        })
    }
    fn decode(&self, file: &Path, _sr: u32) -> Result<SourceBuffer, PortError> {
        let (_, channels) = audio_engine::recorder::read_wav_f32(file)
            .map_err(|e| PortError::new("codec.corrupt", e.to_string()))?;
        Ok(SourceBuffer { channels })
    }
    fn hash_file(&self, file: &Path) -> Result<String, PortError> {
        Ok(fake_hash(
            &std::fs::read(file).map_err(|e| PortError::new("codec.io", e.to_string()))?,
        ))
    }
    fn encode(
        &self,
        file: &Path,
        _s: &ExportSettings,
        _sr: u32,
        planes: &[Vec<f32>],
    ) -> Result<(), PortError> {
        write_wav(file, planes.to_vec());
        Ok(())
    }
    fn available_formats(&self) -> Vec<ExportFormatKind> {
        vec![ExportFormatKind::Wav]
    }
}

/// Efeito de teste: ganho fixo de 0,5 para qualquer tipo.
pub struct HalfGainFactory;
struct Half;
impl AudioProcessor for Half {
    fn set_params(&mut self, _: &[f32]) {}
    fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for x in l.iter_mut().chain(r.iter_mut()) {
            *x *= 0.5;
        }
    }
    fn reset(&mut self) {}
}
impl ProcessorFactory for HalfGainFactory {
    fn type_tag(&self, _: &Effect) -> u32 {
        1
    }
    fn params(&self, _: &Effect) -> Vec<f32> {
        vec![]
    }
    fn create(&self, _: &Effect, _: u32) -> Box<dyn AudioProcessor> {
        Box::new(Half)
    }
}

pub struct FakeClock(pub AtomicI64);
impl Clock for FakeClock {
    fn now_unix(&self) -> i64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

pub fn studio() -> Studio {
    Studio::new(StudioDeps {
        repo: Arc::new(SqliteSessionRepository),
        media: Arc::new(WavMedia),
        factory: Arc::new(HalfGainFactory),
        clock: Arc::new(FakeClock(AtomicI64::new(1_000))),
        synth: Arc::new(SilentSynth),
    })
}

/// Arquivo WAV de teste com um impulso (valor 1) em `at` e DC 0,25 no resto do canal 2.
pub fn impulse_wav(dir: &Path, name: &str, at: usize, frames: usize) -> PathBuf {
    let mut a = vec![0.0; frames];
    a[at] = 1.0;
    let p = dir.join(name);
    write_wav(&p, vec![a]);
    p
}

pub fn dc_wav(dir: &Path, name: &str, value: f32, frames: usize) -> PathBuf {
    let p = dir.join(name);
    write_wav(&p, vec![vec![value; frames], vec![value; frames]]);
    p
}

/// Sintetizador de teste: um quadro de silêncio por tick de duração total (determinístico).
pub struct SilentSynth;
impl MidiRenderer for SilentSynth {
    fn render(
        &self,
        events: &[MidiNoteEvent],
        tempo: &shared_kernel::TempoMap,
        sr: u32,
        tail: f64,
    ) -> Result<SourceBuffer, PortError> {
        let end = events
            .iter()
            .map(|e| e.start_tick + e.duration_ticks)
            .max()
            .unwrap_or(0);
        let n = (tempo.tick_to_seconds(shared_kernel::Tick(end)) + tail)
            .mul_add(sr as f64, 0.0)
            .ceil() as usize;
        Ok(SourceBuffer {
            channels: vec![vec![0.0; n], vec![0.0; n]],
        })
    }
}

/// Studio com a fábrica de DSP real (efeitos de verdade, inclusive o corretor de afinação).
pub fn studio_dsp() -> Studio {
    Studio::new(StudioDeps {
        repo: Arc::new(SqliteSessionRepository),
        media: Arc::new(WavMedia),
        factory: Arc::new(infra_adapters::DspProcessorFactory),
        clock: Arc::new(FakeClock(AtomicI64::new(1_000))),
        synth: Arc::new(SilentSynth),
    })
}

/// WAV mono com um seno (amplitude 0,5).
pub fn sine_wav(dir: &Path, name: &str, hz: f32, frames: usize) -> PathBuf {
    let p = dir.join(name);
    let s: Vec<f32> = (0..frames)
        .map(|i| 0.5 * (std::f32::consts::TAU * hz * i as f32 / SR as f32).sin())
        .collect();
    write_wav(&p, vec![s]);
    p
}

/// Frequência mediana (Hz) dos quadros vozeados de um trecho (detector YIN do dsp-pitch).
pub fn median_pitch(samples: &[f32]) -> f32 {
    let mut f: Vec<f32> = dsp_pitch::pitch_track(samples, SR as f32, 480)
        .iter()
        .filter(|f| f.voiced)
        .map(|f| f.frequency_hz)
        .collect();
    assert!(!f.is_empty(), "nenhum quadro vozeado");
    f.sort_by(|a, b| a.total_cmp(b));
    f[f.len() / 2]
}
