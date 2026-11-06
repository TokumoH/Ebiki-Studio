//! Casos de uso de afinação (Fase 8) e time-stretch (Fase 6).
//!
//! - Pós-produção: analisar uma fonte (`dsp_pitch::analyze_notes`) → mapa de correção desfazível;
//!   editar notas/escala; aplicar = renderizar offline (`render_note_corrections`) para uma fonte
//!   NOVA e trocar o clipe (um único desfazer; o original fica intacto no projeto).
//! - Afinador: derivação lock-free do canal de entrada na engine (`attach_input_tap`) analisada na
//!   thread de controle por um `TunerProcessor` a cada leitura (~20 Hz pela UI) — ADR-0028.
//! - Time-stretch: WSOLA do trecho do clipe para encaixar o BPM de origem no do projeto.

use crate::music_support::{cmd_err, mono_mix};
use crate::session::*;
use crate::studio::{AppError, AppResult, Studio};
use crate::view::SessionView;
use audio_engine::input_tap::InputTapReader;
use ctx_project::commands::{AddClip, AddSource, DeleteClip};
use ctx_project::{Clip, ClipId};
use ctx_tuning::{
    DetectedNote, DetectedNoteId, NoteEdit, OnTuner, PitchCorrectionMap, PutCorrectionMap,
    ReplaceDetectedNotes, ScaleCorrection, SetNoteEdit, SetScaleCorrection, SetTunerReference,
};
use serde::Serialize;
use shared_kernel::command::{Command, Composite};
use shared_kernel::id::AudioSourceId;

/// Limite de duração analisada/renderizada por fonte (custo do YIN/PSOLA offline).
pub const MAX_PITCH_SECONDS: f64 = 20.0 * 60.0;

/// Leitura do afinador para a UI.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunerReadingView {
    /// Nota MIDI mais próxima (0–127).
    pub note: u8,
    /// Nome em PT-BR/cifra com sustenidos (ex.: "A").
    pub name: &'static str,
    pub octave: i32,
    /// Desvio em cents (−50…+50).
    pub cents: f32,
    pub frequency_hz: f32,
    /// 0–1.
    pub confidence: f32,
}

/// Afinador ativo: leitor da derivação + analisador (thread de controle).
pub struct TunerSession {
    reader: InputTapReader,
    processor: dsp_pitch::TunerProcessor,
    scratch: Vec<f32>,
    pub input_channel: usize,
    max_feed: usize,
}

impl TunerSession {
    /// Drena a derivação e analisa só as amostras mais recentes (no máximo ~250 ms).
    pub fn read(&mut self) -> Option<TunerReadingView> {
        self.scratch.clear();
        self.reader.drain(&mut self.scratch);
        let start = self.scratch.len().saturating_sub(self.max_feed);
        self.processor.process(&self.scratch[start..]);
        self.processor.reading().map(|r| {
            let (name, octave) = r.name(false);
            TunerReadingView {
                note: r.note.0,
                name,
                octave,
                cents: r.cents,
                frequency_hz: r.frequency_hz,
                confidence: r.confidence,
            }
        })
    }
}

fn not_analyzed() -> AppError {
    AppError::new(
        "tuning.not_analyzed",
        "Esta fonte de áudio ainda não foi analisada.",
    )
}

/// Notas detectadas pelo DSP → notas do domínio (descarta o que violaria os invariantes).
fn to_domain_notes(notes: &[dsp_pitch::DetectedNote]) -> Vec<DetectedNote> {
    let mut out: Vec<DetectedNote> = Vec::with_capacity(notes.len());
    for n in notes.iter().take(ctx_tuning::book::MAX_NOTES_PER_SOURCE) {
        if out
            .last()
            .is_some_and(|l| l.end_frame > n.start_frame as u64)
        {
            continue;
        }
        if let Ok(d) = DetectedNote::new(
            n.start_frame as u64,
            n.end_frame as u64,
            n.midi_float as f64,
        ) {
            out.push(d);
        }
    }
    out
}

/// Correções efetivas do mapa (edição manual tem prioridade; senão, escala se ativa).
fn corrections_of(
    map: &PitchCorrectionMap,
) -> (Vec<dsp_pitch::DetectedNote>, Vec<dsp_pitch::NoteCorrection>) {
    let scale = map.scale();
    let mut notes = Vec::with_capacity(map.notes().len());
    let mut corrections = Vec::new();
    for (i, n) in map.notes().iter().enumerate() {
        notes.push(dsp_pitch::DetectedNote {
            start_frame: n.start_frame as usize,
            end_frame: n.end_frame as usize,
            midi_float: n.midi as f32,
            mean_cents: n.deviation_cents() as f32,
        });
        let (offset, flatten) = match map.edit(n.id) {
            Some(e) => (map.effective_offset_cents(n.id).unwrap_or(0.0), e.flatten),
            None => match map.scale_target(n.id).ok().flatten() {
                Some(t) => ((t.0 as f64 - n.midi) * 100.0 * (1.0 - scale.humanize), 0.0),
                None => (0.0, 0.0),
            },
        };
        if offset.abs() > 1e-6 || flatten > 0.0 {
            corrections.push(dsp_pitch::NoteCorrection {
                note_index: i,
                offset_cents: offset as f32,
                flatten: flatten as f32,
            });
        }
    }
    (notes, corrections)
}

impl Studio {
    /// Mapa completo de uma fonte (notas, edições, escala) para o editor nota a nota.
    pub fn tuning_get(&self, source: AudioSourceId) -> AppResult<PitchCorrectionMap> {
        self.op()?
            .session
            .tuning
            .map(source)
            .cloned()
            .ok_or_else(not_analyzed)
    }

    /// Detecta as notas da fonte (mistura mono) e grava no mapa (desfazível). Reanálise mantém
    /// a escala e as edições das notas que continuarem existindo.
    pub fn tuning_analyze(&mut self, source: AudioSourceId) -> AppResult<SessionView> {
        let sr = self.sample_rate()?;
        self.op()?.session.project.source(source).map_err(cmd_err)?;
        let buf = self.source_buffer(source)?;
        if buf.frames() as f64 / sr as f64 > MAX_PITCH_SECONDS {
            return Err(AppError::new(
                "app.too_long",
                "Áudio longo demais para análise de afinação (máximo de 20 minutos).",
            ));
        }
        let mono = mono_mix(&buf);
        let notes = to_domain_notes(&dsp_pitch::analyze_notes(&mono, sr as f32));
        let existing = self.op()?.session.tuning.map(source).is_some();
        let cmd: Box<dyn Command<Session>> = if existing {
            on_correction_map(source, ReplaceDetectedNotes::new(notes))
        } else {
            let mut map = PitchCorrectionMap::new(source);
            map.replace_notes(notes).map_err(cmd_err)?;
            on_tuning(PutCorrectionMap::new(source, Some(map)))
        };
        self.execute(cmd, None)
    }

    /// Define (edição neutra = remove) a correção de uma nota detectada.
    pub fn tuning_edit_note(
        &mut self,
        source: AudioSourceId,
        note: DetectedNoteId,
        edit: NoteEdit,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        self.execute(
            on_correction_map(source, SetNoteEdit::new(note, edit)),
            gesture,
        )
    }

    /// Correção automática por escala da fonte.
    pub fn tuning_set_scale(
        &mut self,
        source: AudioSourceId,
        scale: ScaleCorrection,
    ) -> AppResult<SessionView> {
        self.execute(
            on_correction_map(source, SetScaleCorrection::new(scale)),
            None,
        )
    }

    /// Referência do Lá4 do afinador (415–466 Hz).
    pub fn tuning_set_reference(&mut self, a4_hz: f64) -> AppResult<SessionView> {
        let v = self.execute(on_tuning(OnTuner::new(SetTunerReference::new(a4_hz))), None)?;
        if let Some(t) = self.tuner.as_mut() {
            t.processor.set_a4(a4_hz as f32);
        }
        Ok(v)
    }

    /// Descarta a análise da fonte (desfazível).
    pub fn tuning_clear(&mut self, source: AudioSourceId) -> AppResult<SessionView> {
        if self.op()?.session.tuning.map(source).is_none() {
            return Err(not_analyzed());
        }
        self.execute(on_tuning(PutCorrectionMap::new(source, None)), None)
    }

    /// Renderiza as correções do mapa de `source` numa fonte nova e troca o clipe `clip` por um
    /// clipe equivalente dessa fonte (mesma posição, trecho e ganho). Um único desfazer; a fonte
    /// original e o mapa ficam intactos (reaplicar após novas edições parte do original).
    pub fn tuning_apply(&mut self, source: AudioSourceId, clip: ClipId) -> AppResult<SessionView> {
        let sr = self.sample_rate()?;
        let (old_clip, map) = {
            let s = &self.op()?.session;
            let c = s.project.clip(clip).map_err(cmd_err)?.clone();
            let map = s.tuning.map(source).cloned().ok_or_else(not_analyzed)?;
            (c, map)
        };
        let (notes, corrections) = corrections_of(&map);
        if corrections.is_empty() {
            return Err(AppError::new(
                "tuning.nothing_to_apply",
                "Não há correções para aplicar (edite notas ou ative a escala).",
            ));
        }
        let buf = self.source_buffer(source)?;
        if old_clip.offset_frames + old_clip.length_frames > buf.frames() {
            return Err(AppError::new(
                "tuning.clip_mismatch",
                "O clipe é mais longo que a fonte analisada.",
            ));
        }
        let planes: Vec<Vec<f32>> = buf
            .channels
            .iter()
            .map(|ch| dsp_pitch::render_note_corrections(ch, sr as f32, &notes, &corrections))
            .collect();
        let name = {
            let s = &self.op()?.session;
            let base = s
                .project
                .source(source)
                .map(|x| x.original_name.clone())
                .unwrap_or_default();
            format!("{} (afinado).wav", base.trim_end_matches(".wav"))
        };
        let rendered = self.write_rendered_source(planes, &name)?;
        self.swap_clip_source(old_clip, rendered, None, "Aplicar correção de afinação")
    }

    /// Troca o clipe por outro da fonte renderizada (mesma faixa/posição/ganho). `range`:
    /// (offset, comprimento) na nova fonte; `None` mantém os do clipe antigo.
    fn swap_clip_source(
        &mut self,
        old: Clip,
        rendered: crate::music_support::RenderedSource,
        range: Option<(u64, u64)>,
        label: &str,
    ) -> AppResult<SessionView> {
        let track = {
            let p = &self.op()?.session.project;
            let (ti, _) = p.find_clip(old.id).map_err(cmd_err)?;
            p.tracks()[ti].id
        };
        let (offset, length) = range.unwrap_or((old.offset_frames, old.length_frames));
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        let source_id = rendered.source.id;
        if rendered.is_new {
            steps.push(on_project(AddSource::new(rendered.source.clone())));
        }
        steps.push(on_project(DeleteClip::new(old.id)));
        steps.push(on_project(AddClip::new(
            track,
            Clip {
                id: ClipId::new(),
                source_id,
                start: old.start,
                offset_frames: offset,
                length_frames: length,
                gain: old.gain,
                edit: ctx_project::ClipEdit::default(),
            },
        )));
        self.register_buffer(source_id, rendered.buffer);
        self.execute(Box::new(Composite::new(label, steps)), None)
    }

    /// Ajusta o trecho do clipe ao andamento do projeto: `source_bpm` é o andamento original do
    /// áudio; `target_bpm` (padrão: andamento do projeto no início do clipe). Renderiza com WSOLA
    /// (altura preservada) numa fonte nova e troca o clipe (um único desfazer).
    pub fn clip_time_stretch(
        &mut self,
        clip: ClipId,
        source_bpm: f64,
        target_bpm: Option<f64>,
    ) -> AppResult<SessionView> {
        let sr = self.sample_rate()?;
        let (old, target, name) = {
            let s = &self.op()?.session;
            let c = s.project.clip(clip).map_err(cmd_err)?.clone();
            let target = target_bpm.unwrap_or_else(|| s.project.tempo().bpm_at(c.start));
            let name = s
                .project
                .source(c.source_id)
                .map(|x| x.original_name.clone())
                .unwrap_or_default();
            (c, target, name)
        };
        let ratio = ctx_sync::stretch_ratio_for_tempo(source_bpm, target).map_err(cmd_err)?;
        let (lo, hi) = dsp_pitch::stretch::STRETCH_RANGE;
        if !(lo..=hi).contains(&ratio) {
            return Err(AppError::new(
                "app.invalid_argument",
                "Diferença de andamento grande demais (razão entre 0,25 e 4).",
            ));
        }
        if (ratio - 1.0).abs() < 1e-9 {
            return Err(AppError::new(
                "app.nothing_to_do",
                "O clipe já está no andamento do projeto.",
            ));
        }
        let buf = self.source_buffer(old.source_id)?;
        let (a, b) = (
            old.offset_frames as usize,
            (old.offset_frames + old.length_frames) as usize,
        );
        if b > buf.frames() as usize {
            return Err(AppError::new(
                "app.source_unavailable",
                "O clipe ultrapassa o áudio da fonte.",
            ));
        }
        if (b - a) as f64 * ratio / sr as f64 > crate::render::MAX_RENDER_SECONDS {
            return Err(AppError::new("app.too_long", "O resultado excede 1 hora."));
        }
        let ch = buf.channels.len();
        let mut interleaved = Vec::with_capacity((b - a) * ch);
        for f in a..b {
            for c in &buf.channels {
                interleaved.push(c[f]);
            }
        }
        let out = dsp_pitch::time_stretch(&interleaved, ch, ratio);
        let frames = out.len() / ch.max(1);
        let planes: Vec<Vec<f32>> = (0..ch)
            .map(|c| (0..frames).map(|f| out[f * ch + c]).collect())
            .collect();
        let rendered = self.write_rendered_source(
            planes,
            &format!("{} ({:.0} BPM).wav", name.trim_end_matches(".wav"), target),
        )?;
        self.swap_clip_source(
            old,
            rendered,
            Some((0, frames as u64)),
            "Ajustar ao andamento",
        )
    }

    // ------------------------------------------------------------- afinador

    /// Liga o afinador no canal de entrada `input_channel` (0 = primeiro) do dispositivo atual.
    pub fn tuner_start(&mut self, input_channel: usize) -> AppResult<()> {
        if input_channel >= self.input_channels {
            return Err(AppError::new(
                "app.input_unavailable",
                "A entrada escolhida não existe no dispositivo de áudio atual.",
            ));
        }
        let a4 = self
            .open
            .as_ref()
            .map_or(440.0, |o| o.session.tuning.tuner.a4_hz());
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| AppError::new("app.no_engine", "Engine de áudio indisponível."))?;
        let sr = engine.sample_rate();
        let reader = engine
            .attach_input_tap(input_channel, sr as usize)
            .map_err(|e| AppError::new("app.engine", e.to_string()))?;
        self.tuner = Some(TunerSession {
            reader,
            processor: dsp_pitch::TunerProcessor::new(
                sr as f32,
                a4 as f32,
                dsp_pitch::YinConfig::default(),
                0,
            ),
            scratch: Vec::with_capacity(sr as usize),
            input_channel,
            max_feed: (sr / 4) as usize,
        });
        Ok(())
    }

    /// Última leitura (`None` em silêncio/ruído ou com o afinador desligado).
    pub fn tuner_read(&mut self) -> AppResult<Option<TunerReadingView>> {
        let t = self
            .tuner
            .as_mut()
            .ok_or_else(|| AppError::new("app.tuner_off", "O afinador não está ligado."))?;
        Ok(t.read())
    }

    pub fn tuner_stop(&mut self) -> AppResult<()> {
        if self.tuner.take().is_some() {
            if let Some(e) = self.engine.as_mut() {
                e.detach_input_tap()
                    .map_err(|e| AppError::new("app.engine", e.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn tuner_active(&self) -> bool {
        self.tuner.is_some()
    }
}
