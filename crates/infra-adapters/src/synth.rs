//! `MidiRenderer` sobre o crate `synth`.
//!
//! - [`SynthRenderer`]: sempre o [`synth::SimpleSynth`] embutido.
//! - [`ConfigurableSynthRenderer`]: usa um SoundFont quando configurado (preferência
//!   `synth.soundfont` no `library.db`, aplicada pela camada de entrada) e cai para o
//!   `SimpleSynth` caso contrário. Trocável em tempo de execução (ADR-0025).

use app_core::ports::{MidiNoteEvent, MidiRenderer, PortError};
use audio_engine::SourceBuffer;
use shared_kernel::TempoMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use synth::{SoundFontSynth, SynthError};

/// Chave da preferência (no `library.db`) com o caminho absoluto do SoundFont.
pub const SOUNDFONT_SETTING: &str = "synth.soundfont";

fn to_synth_events(events: &[MidiNoteEvent]) -> Vec<synth::NoteEvent> {
    events
        .iter()
        .map(|e| synth::NoteEvent {
            start_tick: e.start_tick,
            duration_ticks: e.duration_ticks,
            channel: e.channel,
            program: e.program,
            key: e.key,
            velocity: e.velocity,
        })
        .collect()
}

fn port_error(e: SynthError) -> PortError {
    PortError::new(e.code(), e.to_string())
}

fn render_simple(
    events: &[MidiNoteEvent],
    tempo: &TempoMap,
    sample_rate: u32,
    tail_seconds: f64,
) -> Result<SourceBuffer, PortError> {
    let [l, r] = synth::SimpleSynth
        .render(&to_synth_events(events), tempo, sample_rate, tail_seconds)
        .map_err(|m| PortError::new("synth.too_long", m))?;
    Ok(SourceBuffer {
        channels: vec![l, r],
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SynthRenderer;

impl MidiRenderer for SynthRenderer {
    fn render(
        &self,
        events: &[MidiNoteEvent],
        tempo: &TempoMap,
        sample_rate: u32,
        tail_seconds: f64,
    ) -> Result<SourceBuffer, PortError> {
        render_simple(events, tempo, sample_rate, tail_seconds)
    }
}

/// Estado do sintetizador exposto à UI.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthStatus {
    /// `"soundfont"` ou `"simple"`.
    pub engine: String,
    pub soundfont_path: Option<String>,
    pub bank_name: Option<String>,
    pub preset_count: usize,
}

/// Renderizador com SoundFont opcional (fallback: `SimpleSynth`).
#[derive(Debug, Default)]
pub struct ConfigurableSynthRenderer {
    soundfont: RwLock<Option<Arc<SoundFontSynth>>>,
}

impl ConfigurableSynthRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Carrega e passa a usar o SoundFont em `path`. Em caso de erro mantém o estado anterior.
    pub fn set_soundfont(&self, path: &Path) -> Result<SynthStatus, PortError> {
        let sf = SoundFontSynth::load(path).map_err(port_error)?;
        *self.soundfont.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(sf));
        Ok(self.status())
    }

    /// Volta ao sintetizador simples embutido.
    pub fn clear_soundfont(&self) -> SynthStatus {
        *self.soundfont.write().unwrap_or_else(|e| e.into_inner()) = None;
        self.status()
    }

    pub fn soundfont_path(&self) -> Option<PathBuf> {
        self.current().map(|s| s.path().to_path_buf())
    }

    fn current(&self) -> Option<Arc<SoundFontSynth>> {
        self.soundfont
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn status(&self) -> SynthStatus {
        match self.current() {
            Some(sf) => SynthStatus {
                engine: "soundfont".into(),
                soundfont_path: Some(sf.path().to_string_lossy().into_owned()),
                bank_name: Some(sf.bank_name().to_owned()),
                preset_count: sf.preset_count(),
            },
            None => SynthStatus {
                engine: "simple".into(),
                soundfont_path: None,
                bank_name: None,
                preset_count: 0,
            },
        }
    }
}

impl MidiRenderer for ConfigurableSynthRenderer {
    fn render(
        &self,
        events: &[MidiNoteEvent],
        tempo: &TempoMap,
        sample_rate: u32,
        tail_seconds: f64,
    ) -> Result<SourceBuffer, PortError> {
        match self.current() {
            Some(sf) => {
                let [l, r] = sf
                    .render(&to_synth_events(events), tempo, sample_rate, tail_seconds)
                    .map_err(port_error)?;
                Ok(SourceBuffer {
                    channels: vec![l, r],
                })
            }
            None => render_simple(events, tempo, sample_rate, tail_seconds),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note() -> MidiNoteEvent {
        MidiNoteEvent {
            start_tick: 0,
            duration_ticks: 960,
            channel: 0,
            program: 0,
            key: 69,
            velocity: 100,
        }
    }

    #[test]
    fn falls_back_to_simple_synth_and_keeps_state_on_bad_file() {
        let r = ConfigurableSynthRenderer::new();
        assert_eq!(r.status().engine, "simple");
        let buf = r
            .render(&[note()], &TempoMap::default(), 48_000, 0.25)
            .unwrap();
        assert_eq!(buf.channels.len(), 2);
        assert_eq!(buf.channels[0].len(), 36_000);
        assert!(buf.channels[0].iter().any(|x| x.abs() > 1e-3));
        let dir = tempfile::tempdir().unwrap();
        let err = r.set_soundfont(&dir.path().join("x.sf2")).unwrap_err();
        assert_eq!(err.code, "synth.io");
        assert_eq!(r.status().engine, "simple");
    }

    #[test]
    fn uses_soundfont_when_configured_and_can_clear() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.sf2");
        std::fs::write(&p, synth::soundfont::test_support::tiny_sf2()).unwrap();
        let r = ConfigurableSynthRenderer::new();
        let st = r.set_soundfont(&p).unwrap();
        assert_eq!(st.engine, "soundfont");
        assert_eq!(st.preset_count, 1);
        assert_eq!(r.soundfont_path().as_deref(), Some(p.as_path()));
        let sf = r
            .render(&[note()], &TempoMap::default(), 48_000, 0.25)
            .unwrap();
        let simple = SynthRenderer
            .render(&[note()], &TempoMap::default(), 48_000, 0.25)
            .unwrap();
        assert_eq!(sf.channels[0].len(), simple.channels[0].len());
        assert_ne!(sf.channels[0], simple.channels[0], "timbre do SoundFont");
        assert_eq!(r.clear_soundfont().engine, "simple");
    }
}
