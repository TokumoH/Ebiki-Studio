//! Casos de uso de notação (Fase 7): criar/editar/importar/exportar partituras, ouvir e renderizar
//! para uma faixa de áudio. Regras de notação ficam em `ctx-notation`; formatos em
//! `infra-notation-io` (mapeamento puro de bytes, sem I/O de sistema — ADR-0021); o arquivo é lido
//! e gravado aqui, com limite de tamanho e escrita atômica.

use crate::music_support::{cmd_err, read_limited, wav_bytes, write_atomic};
use crate::ports::MidiNoteEvent;
use crate::session::*;
use crate::studio::{AppError, AppResult, Studio};
use crate::view::SessionView;
use ctx_mixing::commands::AddStrip;
use ctx_mixing::ChannelStrip;
use ctx_notation::analysis::{
    out_of_key_notes, out_of_scale_notes, validate_measures, MeasureIssue, OutOfScaleNote,
};
use ctx_notation::arrange::{
    arrange_part, build_template, chords_to_score, ArrangeOptions, ChordSlot,
};
use ctx_notation::book::{AddScore, RemoveScore};
use ctx_notation::commands::{EditScore, EditVoice, ScoreEdit, VoiceEdit};
use ctx_notation::playback::{render, PlaybackOptions};
use ctx_notation::{Clef, KeySignature, Meter, Mode, Score, ScoreId, Staff, TabConfig, VoiceLoc};
use ctx_project::commands::{AddClip, AddSource, CreateTrack};
use ctx_project::{Clip, ClipId, Track};
use serde::{Deserialize, Serialize};
use shared_kernel::command::{Command, Composite};
use shared_kernel::id::TrackId;
use shared_kernel::music::PitchClass;
use shared_kernel::{Db, Tick};
use std::path::Path;

/// Limite de leitura de arquivos de partitura (igual ao de `infra-notation-io`).
pub const MAX_SCORE_FILE_BYTES: u64 = 32 * 1024 * 1024;
/// Limite de duração renderizada de uma partitura (audição/faixa).
pub const MAX_SCORE_RENDER_SECONDS: f64 = 30.0 * 60.0;
/// Cauda após a última nota (release do sintetizador).
pub const SCORE_TAIL_SECONDS: f64 = 1.5;

/// Instrumento de um modelo de partitura em branco.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstrumentKind {
    /// Piano em pauta dupla (programa GM 0).
    Piano,
    /// Melodia/voz em clave de Sol (programa GM 52, coro "aah").
    Lead,
    /// Violão com pauta + tablatura (programa GM 24).
    Guitar,
    /// Baixo com pauta + tablatura (programa GM 33).
    Bass,
    /// Bateria (canal 10).
    Drums,
}

impl InstrumentKind {
    fn default_name(self) -> &'static str {
        match self {
            Self::Piano => "Piano",
            Self::Lead => "Melodia",
            Self::Guitar => "Violão",
            Self::Bass => "Baixo",
            Self::Drums => "Bateria",
        }
    }

    fn program(self) -> u8 {
        match self {
            Self::Piano | Self::Drums => 0,
            Self::Lead => 52,
            Self::Guitar => 24,
            Self::Bass => 33,
        }
    }

    fn staves(self) -> Vec<Staff> {
        match self {
            Self::Piano => vec![Staff::standard(Clef::Treble), Staff::standard(Clef::Bass)],
            Self::Lead => vec![Staff::standard(Clef::Treble)],
            Self::Guitar => vec![Staff::standard_and_tab(
                Clef::Treble8vb,
                TabConfig::standard_guitar(),
            )],
            Self::Bass => vec![Staff::standard_and_tab(
                Clef::Bass,
                TabConfig::standard_bass(),
            )],
            Self::Drums => vec![Staff::standard(Clef::Percussion)],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentTemplate {
    pub kind: InstrumentKind,
    /// Nome da parte (padrão pelo tipo).
    #[serde(default)]
    pub name: Option<String>,
}

/// Modelo de partitura em branco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreTemplate {
    pub title: String,
    #[serde(default)]
    pub composer: String,
    pub bpm: f64,
    pub meter_numerator: u8,
    pub meter_denominator: u8,
    /// Armadura: −7…+7 (bemóis negativos).
    #[serde(default)]
    pub key_fifths: i8,
    #[serde(default)]
    pub key_mode: Mode,
    /// Número de compassos (1–1000).
    pub measures: u32,
    pub instruments: Vec<InstrumentTemplate>,
}

/// Formato de exportação de partitura (PDF é gerado na UI pelo alphaTab).
pub use crate::ports::ScoreFileFormat as ScoreExportFormat;

/// Relatório de validação de uma partitura.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreReport {
    /// Vozes com tempo faltando/sobrando.
    pub measure_issues: Vec<MeasureIssue>,
    /// Notas fora da escala informada (ou da armadura, sem escala).
    pub out_of_scale: Vec<OutOfScaleNote>,
}

/// Resultado de importação.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreImportResult {
    pub score_id: String,
    /// Avisos (PT-BR) do importador: o que foi aproximado/descartado.
    pub warnings: Vec<String>,
    pub view: SessionView,
}

/// Partitura → notas MIDI para o sintetizador (porta `MidiRenderer`).
pub fn score_midi_events(score: &Score) -> Vec<MidiNoteEvent> {
    render(score, &PlaybackOptions::default())
        .into_iter()
        .map(|e| MidiNoteEvent {
            start_tick: e.start_tick.0,
            duration_ticks: e.duration_ticks.0,
            channel: e.channel,
            program: e.program,
            key: e.key,
            velocity: e.velocity,
        })
        .collect()
}

/// Cria a partitura em branco de um modelo (pausas em todos os compassos).
pub fn score_from_template(t: &ScoreTemplate) -> AppResult<Score> {
    let meter = Meter::new(t.meter_numerator, t.meter_denominator).map_err(cmd_err)?;
    let key = KeySignature::new(t.key_fifths, t.key_mode).map_err(cmd_err)?;
    if !(1..=1000).contains(&t.measures) {
        return Err(AppError::new(
            "app.invalid_argument",
            "Número de compassos deve estar entre 1 e 1000.",
        ));
    }
    if t.instruments.is_empty() {
        return Err(AppError::new(
            "app.invalid_argument",
            "Escolha ao menos um instrumento.",
        ));
    }
    let mut score = Score::new(t.title.trim(), t.bpm, meter).map_err(cmd_err)?;
    score.composer = t.composer.trim().to_owned();
    let end = Tick(t.measures as i64 * meter.ticks());
    let template = build_template(meter, &[], &[(Tick(0), key)], end).map_err(cmd_err)?;
    for inst in &t.instruments {
        let name = inst
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .unwrap_or(inst.kind.default_name());
        let (mut part, _) = arrange_part(
            &template,
            meter,
            &[],
            name,
            inst.kind.program(),
            inst.kind.staves(),
            &ArrangeOptions::default(),
        )
        .map_err(cmd_err)?;
        part.percussion = inst.kind == InstrumentKind::Drums;
        score.push_part(part).map_err(cmd_err)?;
    }
    Ok(score)
}

impl Studio {
    fn add_score(&mut self, score: Score, label: &str) -> AppResult<(ScoreId, SessionView)> {
        let id = ScoreId::new();
        let v = self.execute(on_scores(AddScore::new(id, score, usize::MAX, label)), None)?;
        Ok((id, v))
    }

    /// Partitura completa (para o editor/alphaTab).
    pub fn score_get(&self, id: ScoreId) -> AppResult<Score> {
        Ok(self.op()?.session.scores.get(id).map_err(cmd_err)?.clone())
    }

    /// Nova partitura em branco a partir de um modelo (desfazível).
    pub fn score_create(&mut self, template: &ScoreTemplate) -> AppResult<(ScoreId, SessionView)> {
        self.op()?;
        let score = score_from_template(template)?;
        self.add_score(score, "Nova partitura")
    }

    /// Partitura de piano a partir de acordes realizados (ex.: progressão do contexto Harmony).
    pub fn score_create_from_chords(
        &mut self,
        title: &str,
        meter: Meter,
        key: KeySignature,
        slots: &[ChordSlot],
    ) -> AppResult<(ScoreId, SessionView)> {
        let bpm = self.op()?.session.project.tempo().bpm_at(Tick(0));
        if slots.is_empty() {
            return Err(AppError::new(
                "app.invalid_argument",
                "Nenhum acorde para gerar a partitura.",
            ));
        }
        let score = chords_to_score(title, bpm, meter, key, slots).map_err(cmd_err)?;
        self.add_score(score, "Partitura dos acordes")
    }

    pub fn score_delete(&mut self, id: ScoreId) -> AppResult<SessionView> {
        self.execute(on_scores(RemoveScore::new(id)), None)
    }

    /// Edição dentro de uma voz (desfazível; `gesture` agrupa arrastes contínuos).
    pub fn score_edit_voice(
        &mut self,
        id: ScoreId,
        loc: VoiceLoc,
        edit: VoiceEdit,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        self.execute(on_score(id, EditVoice::new(loc, edit)), gesture)
    }

    /// Edição estrutural (desfazível; `gesture` agrupa com edições de voz — ex.: notas digitadas
    /// em lote que criam compassos viram um único passo de desfazer).
    pub fn score_edit(
        &mut self,
        id: ScoreId,
        edit: ScoreEdit,
        gesture: Option<u64>,
    ) -> AppResult<SessionView> {
        self.execute(on_score(id, EditScore::new(edit)), gesture)
    }

    /// Importa MusicXML/MXL/MIDI/Guitar Pro 7 (detecção pelo conteúdo) como nova partitura.
    pub fn score_import(&mut self, path: &Path) -> AppResult<ScoreImportResult> {
        self.op()?;
        let bytes = read_limited(path, MAX_SCORE_FILE_BYTES)?;
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.score_import_bytes(&bytes, &stem)
    }

    /// Importa bytes já lidos pela UI (ex.: GP3–GP6 convertidos para GP7 pelo alphaTab —
    /// ADR-0020). `name` é o título usado quando o arquivo não traz um.
    pub fn score_import_bytes(&mut self, bytes: &[u8], name: &str) -> AppResult<ScoreImportResult> {
        self.op()?;
        if bytes.len() as u64 > MAX_SCORE_FILE_BYTES {
            return Err(AppError::new(
                "app.file_too_large",
                "Arquivo de partitura grande demais (máximo de 32 MiB).",
            ));
        }
        let imported = self.deps().notation.import(bytes)?;
        let mut score = imported.score;
        if score.title.trim().is_empty() {
            let clean: String = name.chars().filter(|c| !c.is_control()).take(120).collect();
            score.title = if clean.trim().is_empty() {
                "Partitura importada".to_owned()
            } else {
                clean.trim().to_owned()
            };
        }
        let (id, view) = self.add_score(score, "Importar partitura")?;
        Ok(ScoreImportResult {
            score_id: id.to_string(),
            warnings: imported.warnings,
            view,
        })
    }

    /// MusicXML (texto) da partitura, para o renderizador da UI (alphaTab) e a impressão em PDF.
    pub fn score_musicxml(&self, id: ScoreId) -> AppResult<String> {
        let s = self.op()?.session.scores.get(id).map_err(cmd_err)?;
        let bytes = self
            .deps()
            .notation
            .export(s, ScoreExportFormat::MusicXml)?;
        String::from_utf8(bytes).map_err(|_| {
            AppError::new(
                "notation.encoding",
                "MusicXML gerado com codificação inválida.",
            )
        })
    }

    /// Exporta para `path` (escrita atômica).
    pub fn score_export(
        &self,
        id: ScoreId,
        path: &Path,
        format: ScoreExportFormat,
    ) -> AppResult<()> {
        let s = self.op()?.session.scores.get(id).map_err(cmd_err)?;
        let bytes = self.deps().notation.export(s, format)?;
        write_atomic(path, &bytes)
    }

    /// Validação de compassos + notas fora da escala (`scale`: classes 0–11; sem escala, usa a
    /// armadura de cada compasso).
    pub fn score_validate(&self, id: ScoreId, scale: Option<&[u8]>) -> AppResult<ScoreReport> {
        let s = self.op()?.session.scores.get(id).map_err(cmd_err)?;
        let out_of_scale = match scale {
            Some(pcs) => {
                let set: Vec<PitchClass> = pcs
                    .iter()
                    .map(|v| {
                        PitchClass::new(*v).ok_or_else(|| {
                            AppError::new(
                                "app.invalid_argument",
                                "Classe de altura deve estar entre 0 e 11.",
                            )
                        })
                    })
                    .collect::<AppResult<_>>()?;
                out_of_scale_notes(s, &set)
            }
            None => out_of_key_notes(s),
        };
        Ok(ScoreReport {
            measure_issues: validate_measures(s),
            out_of_scale,
        })
    }

    /// Renderiza a partitura pelo sintetizador (taxa do projeto) → canais não intercalados.
    fn render_score_audio(&self, id: ScoreId) -> AppResult<Vec<Vec<f32>>> {
        let op = self.op()?;
        let sr = op.session.project.format.sample_rate.hz();
        let s = op.session.scores.get(id).map_err(cmd_err)?;
        let events = score_midi_events(s);
        if events.is_empty() {
            return Err(AppError::new(
                "app.empty_score",
                "A partitura não tem notas para tocar.",
            ));
        }
        let tempo = s.tempo_map();
        let end = events
            .iter()
            .map(|e| e.start_tick + e.duration_ticks)
            .max()
            .unwrap_or(0);
        if tempo.tick_to_seconds(Tick(end)) > MAX_SCORE_RENDER_SECONDS {
            return Err(AppError::new(
                "app.too_long",
                "A partitura excede 30 minutos de áudio.",
            ));
        }
        let buf = self
            .deps
            .synth
            .render(&events, &tempo, sr, SCORE_TAIL_SECONDS)?;
        Ok(buf.channels)
    }

    /// Áudio da partitura para audição imediata na UI (WAV float 32 em memória).
    pub fn score_audition(&self, id: ScoreId) -> AppResult<Vec<u8>> {
        let sr = self.sample_rate()?;
        Ok(wav_bytes(&self.render_score_audio(id)?, sr))
    }

    /// Renderiza a partitura e coloca o áudio numa faixa (nova, se `track` = `None`) em `at`.
    /// Fonte + faixa + canal + clipe num único desfazer (como `import_audio`).
    pub fn score_render_to_track(
        &mut self,
        id: ScoreId,
        track: Option<TrackId>,
        at: Tick,
    ) -> AppResult<SessionView> {
        let title = self.score_get(id)?.title;
        if let Some(t) = track {
            self.op()?.session.project.track(t).map_err(cmd_err)?;
        }
        let planes = self.render_score_audio(id)?;
        let name = if title.trim().is_empty() {
            "Partitura".to_owned()
        } else {
            title.trim().to_owned()
        };
        let rendered = self.write_rendered_source(planes, &format!("{name}.wav"))?;
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        let source_id = rendered.source.id;
        let frames = rendered.source.length_frames;
        if rendered.is_new {
            steps.push(on_project(AddSource::new(rendered.source.clone())));
        }
        let target = match track {
            Some(t) => t,
            None => {
                let label: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
                let t = Track::new_audio(&label).map_err(cmd_err)?;
                let tid = t.id;
                steps.push(on_project(CreateTrack::new(t, usize::MAX)));
                steps.push(on_mixer(AddStrip::new(tid, ChannelStrip::default())));
                tid
            }
        };
        steps.push(on_project(AddClip::new(
            target,
            Clip {
                id: ClipId::new(),
                source_id,
                start: Tick(at.0.max(0)),
                offset_frames: 0,
                length_frames: frames,
                gain: Db::UNITY,
                edit: ctx_project::ClipEdit::default(),
            },
        )));
        self.register_buffer(source_id, rendered.buffer);
        self.execute(
            Box::new(Composite::new("Renderizar partitura", steps)),
            None,
        )
    }
}
