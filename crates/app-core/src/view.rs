//! DTOs de leitura para a UI (projeções da sessão; o núcleo continua sendo a fonte da verdade).

use crate::compile::MeterMap;
use crate::session::Session;
use ctx_mixing::{Bus, ChannelStrip, StripTarget};
use ctx_project::{AudioSource, ClipEdit};
use ctx_recording::{MetronomeConfig, TrackRecording};
use ctx_sync::LyricTrack;
use ctx_tuning::ScaleCorrection;
use serde::Serialize;
use shared_kernel::{BarBeat, Db, TempoMap, Tick};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub id: String,
    pub name: String,
    pub root: String,
    pub sample_rate: u32,
    pub bit_depth: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipView {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub start: Tick,
    pub start_frame: u64,
    pub offset_frames: u64,
    pub length_frames: u64,
    pub end_tick: Tick,
    pub gain: Db,
    /// Duração total da fonte (limite para cortar/estender as bordas).
    pub source_frames: u64,
    /// Fades, polaridade e reverso (Fase 3).
    #[serde(flatten)]
    pub edit: ClipEdit,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackView {
    pub id: String,
    pub name: String,
    pub clips: Vec<ClipView>,
    pub strip: ChannelStrip,
    pub recording: TrackRecording,
    pub audible: bool,
    pub meter_slot: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BusView {
    #[serde(flatten)]
    pub bus: Bus,
    pub audible: bool,
    pub meter_slot: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingView {
    pub metronome: MetronomeConfig,
    pub punch: Option<(Tick, Tick)>,
    pub loop_range: Option<(Tick, Tick)>,
    pub latency_compensation_frames: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub project: ProjectView,
    pub tempo: TempoMap,
    pub tracks: Vec<TrackView>,
    pub buses: Vec<BusView>,
    pub master: ChannelStrip,
    pub sources: Vec<AudioSource>,
    pub recording: RecordingView,
    pub history: HistoryView,
    pub content_end_frame: u64,
    /// Letra sincronizada completa (linhas → palavras → sílabas com tempos em ticks).
    pub lyrics: LyricTrack,
    /// Resumo das partituras (a partitura completa vem de `score_get`).
    pub scores: Vec<ScoreSummary>,
    /// Resumo da afinação (o mapa completo vem de `tuning_get`).
    pub tuning: TuningView,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSummary {
    pub id: String,
    pub title: String,
    pub composer: String,
    pub parts: Vec<String>,
    pub measures: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionMapSummary {
    pub map_id: String,
    pub source_id: String,
    pub note_count: usize,
    pub edit_count: usize,
    pub scale: ScaleCorrection,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TuningView {
    pub a4_hz: f64,
    pub maps: Vec<CorrectionMapSummary>,
}

fn scores_view(s: &Session) -> Vec<ScoreSummary> {
    s.scores
        .entries()
        .iter()
        .map(|e| ScoreSummary {
            id: e.id.to_string(),
            title: e.score.title.clone(),
            composer: e.score.composer.clone(),
            parts: e.score.parts.iter().map(|p| p.name.clone()).collect(),
            measures: e.score.measure_count(),
        })
        .collect()
}

fn tuning_view(s: &Session) -> TuningView {
    TuningView {
        a4_hz: s.tuning.tuner.a4_hz(),
        maps: s
            .tuning
            .maps()
            .values()
            .map(|m| CorrectionMapSummary {
                map_id: m.id.to_string(),
                source_id: m.source.to_string(),
                note_count: m.notes().len(),
                edit_count: m.edits().len(),
                scale: m.scale().clone(),
            })
            .collect(),
    }
}

pub fn session_view(
    s: &Session,
    root: &str,
    meters: &MeterMap,
    history: HistoryView,
) -> SessionView {
    let p = &s.project;
    let sr = p.format.sample_rate.hz();
    let tempo = p.tempo();
    let slot_of = |id| {
        meters
            .tracks
            .iter()
            .find(|(t, _)| *t == id)
            .map_or(0, |(_, s)| *s)
    };
    let tracks = p
        .tracks()
        .iter()
        .map(|t| TrackView {
            id: t.id.to_string(),
            name: t.name.clone(),
            clips: t
                .clips
                .iter()
                .map(|c| {
                    let start_frame = tempo.tick_to_frame(c.start, sr);
                    ClipView {
                        id: c.id.to_string(),
                        source_id: c.source_id.to_string(),
                        source_name: p
                            .source(c.source_id)
                            .map(|s| s.original_name.clone())
                            .unwrap_or_default(),
                        start: c.start,
                        start_frame,
                        offset_frames: c.offset_frames,
                        length_frames: c.length_frames,
                        end_tick: tempo
                            .frame_to_tick(start_frame.saturating_add(c.length_frames), sr),
                        gain: c.gain,
                        source_frames: p
                            .source(c.source_id)
                            .map(|s| s.length_frames)
                            .unwrap_or(c.length_frames),
                        edit: c.edit,
                    }
                })
                .collect(),
            strip: s
                .mixer
                .strip(StripTarget::Track(t.id))
                .cloned()
                .unwrap_or_default(),
            recording: s.recording.track(t.id),
            audible: s.mixer.is_track_audible(t.id),
            meter_slot: slot_of(t.id),
        })
        .collect();
    let buses = s
        .mixer
        .buses()
        .iter()
        .map(|b| BusView {
            bus: b.clone(),
            audible: s.mixer.is_bus_audible(b.id),
            meter_slot: meters
                .buses
                .iter()
                .find(|(id, _)| *id == b.id)
                .map_or(0, |(_, s)| *s),
        })
        .collect();
    SessionView {
        project: ProjectView {
            id: p.id.to_string(),
            name: p.name().to_owned(),
            root: root.to_owned(),
            sample_rate: sr,
            bit_depth: p.format.bit_depth.bits(),
        },
        tempo: tempo.clone(),
        tracks,
        buses,
        master: s.mixer.master().clone(),
        sources: p.sources().to_vec(),
        recording: RecordingView {
            metronome: s.recording.metronome,
            punch: s.recording.punch(),
            loop_range: s.recording.loop_range(),
            latency_compensation_frames: s.recording.latency_compensation_frames,
        },
        history,
        content_end_frame: p.content_end_frame(),
        lyrics: s.lyrics.clone(),
        scores: scores_view(s),
        tuning: tuning_view(s),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransportView {
    pub state: String,
    pub frame: u64,
    pub tick: Tick,
    pub seconds: f64,
    pub bar_beat: BarBeat,
    pub bpm: f64,
    pub record_start: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeterView {
    pub id: String,
    pub peak: [f32; 2],
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetersView {
    pub master: [f32; 2],
    pub tracks: Vec<MeterView>,
    pub buses: Vec<MeterView>,
    pub xruns: u64,
    pub record_overflow: u64,
}
