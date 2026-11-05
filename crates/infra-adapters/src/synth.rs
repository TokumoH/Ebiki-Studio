//! `MidiRenderer` sobre o crate `synth`.

use app_core::ports::{MidiNoteEvent, MidiRenderer, PortError};
use audio_engine::SourceBuffer;
use shared_kernel::TempoMap;

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
        let evs: Vec<synth::NoteEvent> = events
            .iter()
            .map(|e| synth::NoteEvent {
                start_tick: e.start_tick,
                duration_ticks: e.duration_ticks,
                channel: e.channel,
                program: e.program,
                key: e.key,
                velocity: e.velocity,
            })
            .collect();
        let [l, r] = synth::SimpleSynth
            .render(&evs, tempo, sample_rate, tail_seconds)
            .map_err(|m| PortError::new("synth.too_long", m))?;
        Ok(SourceBuffer {
            channels: vec![l, r],
        })
    }
}
