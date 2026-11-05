//! Casos de uso de composição que tocam o projeto aberto (`impl Studio`):
//!
//! - **Audição** (RF-5.5, RF-4.3): renderiza notas pela porta `MidiRenderer` (ou decodifica o áudio
//!   da base) e devolve um WAV PCM 16 bits estéreo **em memória**, limitado a
//!   [`MAX_AUDITION_SECONDS`].
//! - **Arrastar para a timeline** (RF-5.6): renderiza na taxa e no mapa de tempo do projeto a
//!   partir do tick de destino, grava `audio/generated/<id>.wav` (porta `MediaPort::encode`) e
//!   registra fonte + clipe (+ faixa nova, se preciso) como **um único** comando desfazível.
//! - **Inserir base** (RF-4.5, ADR-0026): calcula o `MismatchReport` e exige decisão explícita
//!   (`Keep` | `Adapt`) quando há divergência de BPM/tom.

use super::bases::generated_events;
use super::harmony::RealizeRequest;
use super::library::Composer;
use super::wav::encode_wav_pcm16;
use super::{invalid, library_err};
use crate::ports::{ExportFormatKind, ExportSettings, MidiNoteEvent};
use crate::session::{on_mixer, on_project, Session};
use crate::studio::{safe_join, AppError, AppResult, Studio};
use crate::view::SessionView;
use audio_engine::SourceBuffer;
use ctx_library::{mismatch, BaseId, BaseKey, BaseKind, MismatchReport};
use ctx_mixing::commands::AddStrip;
use ctx_mixing::ChannelStrip;
use ctx_project::commands::{AddClip, AddSource, CreateTrack};
use ctx_project::{AudioSource, Clip, ClipId, RelPath, Track};
use dsp_pitch::stretch::STRETCH_RANGE;
use dsp_pitch::{pitch_shift_offline, time_stretch};
use serde::{Deserialize, Serialize};
use shared_kernel::command::{Command, CommandError, Composite};
use shared_kernel::id::{AudioSourceId, TrackId};
use shared_kernel::{Db, MeterEvent, TempoEvent, TempoMap, Tick};

/// Cauda (release) acrescentada à audição.
pub const AUDITION_TAIL_SECONDS: f64 = 1.0;
/// Cauda acrescentada ao áudio inserido na timeline.
pub const INSERT_TAIL_SECONDS: f64 = 1.0;
/// Duração máxima de uma audição (o excedente é cortado).
pub const MAX_AUDITION_SECONDS: f64 = 120.0;
/// Taxa da audição sem projeto aberto.
pub const DEFAULT_AUDITION_RATE: u32 = 48_000;
/// Pasta (relativa ao projeto) dos áudios renderizados por composição.
pub const GENERATED_AUDIO_DIR: &str = "audio/generated";

/// Decisão do usuário quando a base diverge do projeto (RF-4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BaseDecision {
    /// Mantém BPM e tom originais da base.
    Keep,
    /// Ajusta andamento (time-stretch) e tom (transposição) ao projeto.
    Adapt,
}

/// Clipe criado na timeline.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertedClip {
    pub track_id: TrackId,
    pub clip_id: ClipId,
    pub source_id: AudioSourceId,
    pub rel_path: String,
    /// Duração do clipe em quadros (taxa do projeto).
    pub frames: u64,
    pub view: SessionView,
}

/// Pedido de inserção de base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertBaseRequest {
    pub base_id: BaseId,
    pub at: Tick,
    /// Faixa de destino (`None` = nova faixa com o nome da base).
    #[serde(default)]
    pub track: Option<TrackId>,
    /// Tom do projeto, se conhecido (o projeto não guarda tom; a UI informa).
    #[serde(default)]
    pub project_key: Option<BaseKey>,
    /// Obrigatória quando `report.needsDecision`.
    #[serde(default)]
    pub decision: Option<BaseDecision>,
}

/// Resultado: ou falta a decisão, ou a base foi inserida.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum InsertBaseOutcome {
    NeedsDecision {
        report: MismatchReport,
    },
    Inserted {
        report: MismatchReport,
        decision: BaseDecision,
        clip: Box<InsertedClip>,
    },
}

enum SourcePlan {
    New(AudioSource, SourceBuffer),
    Existing(AudioSourceId, u64),
}

fn no_project() -> AppError {
    AppError::new("app.no_project", "Nenhum projeto aberto.")
}

fn io(e: std::io::Error) -> AppError {
    AppError::new("app.io", format!("Erro de arquivo: {e}"))
}

fn project_err(e: ctx_project::ProjectError) -> AppError {
    AppError::from(CommandError::from(e))
}

fn track_name(name: &str) -> String {
    let n: String = name.chars().filter(|c| !c.is_control()).take(64).collect();
    let n = n.trim();
    if n.is_empty() {
        "Composição".into()
    } else {
        n.to_owned()
    }
}

fn truncate(mut channels: Vec<Vec<f32>>, max_frames: usize) -> Vec<Vec<f32>> {
    for c in &mut channels {
        c.truncate(max_frames);
    }
    channels
}

/// Ajusta áudio não intercalado ao projeto: time-stretch pela razão de BPM e transposição.
/// A duração resultante é `frames × bpm_base / bpm_projeto`.
pub fn adapt_audio(channels: Vec<Vec<f32>>, report: &MismatchReport) -> AppResult<Vec<Vec<f32>>> {
    let ch = channels.len();
    let frames = channels.iter().map(Vec::len).min().unwrap_or(0);
    if ch == 0 || frames == 0 {
        return Ok(channels);
    }
    let mut inter = Vec::with_capacity(frames * ch);
    for i in 0..frames {
        for c in &channels {
            inter.push(c[i]);
        }
    }
    let ratio = 1.0 / report.bpm_ratio;
    if !ratio.is_finite() {
        return Err(invalid(
            "adapt_out_of_range",
            "Razão de andamento inválida.",
        ));
    }
    if (ratio - 1.0).abs() > 1e-9 {
        if !(STRETCH_RANGE.0..=STRETCH_RANGE.1).contains(&ratio) {
            return Err(invalid(
                "adapt_out_of_range",
                "Diferença de andamento grande demais para ajustar (até 4× mais lento ou mais rápido).",
            ));
        }
        inter = time_stretch(&inter, ch, ratio);
    }
    if let Some(s) = report.semitone_shift.filter(|s| *s != 0) {
        inter = pitch_shift_offline(&inter, ch, s as f64);
    }
    let out_frames = inter.len() / ch;
    Ok((0..ch)
        .map(|c| (0..out_frames).map(|i| inter[i * ch + c]).collect())
        .collect())
}

impl Studio {
    fn rate(&self) -> u32 {
        self.session()
            .map_or(DEFAULT_AUDITION_RATE, |s| s.project.format.sample_rate.hz())
    }

    /// BPM do projeto no tick dado.
    pub fn project_bpm_at(&self, at: Tick) -> Option<f64> {
        self.session().map(|s| s.project.tempo().bpm_at(at))
    }

    /// Mapa de tempo do projeto a partir de `at`, rebaseado para o tick 0 (mudanças de BPM depois
    /// de `at` são respeitadas na renderização).
    pub fn tempo_from(&self, at: Tick) -> AppResult<TempoMap> {
        let s = self.session().ok_or_else(no_project)?;
        let t = s.project.tempo();
        let mut tempos = vec![TempoEvent {
            tick: Tick::ZERO,
            bpm: t.bpm_at(at),
        }];
        tempos.extend(
            t.tempos()
                .iter()
                .filter(|e| e.tick > at)
                .map(|e| TempoEvent {
                    tick: Tick(e.tick.0 - at.0),
                    bpm: e.bpm,
                }),
        );
        let m = t.meter_at(at);
        TempoMap::from_events(
            tempos,
            vec![MeterEvent {
                tick: Tick::ZERO,
                numerator: m.numerator,
                denominator: m.denominator,
            }],
        )
        .map_err(|e| AppError::new("app.tempo", e.to_string()))
    }

    /// Notas → WAV PCM 16 bits estéreo (taxa do projeto ou 48 kHz), cortado em
    /// [`MAX_AUDITION_SECONDS`].
    pub fn audition_midi(
        &self,
        events: &[MidiNoteEvent],
        tempo: &TempoMap,
        tail_seconds: f64,
    ) -> AppResult<Vec<u8>> {
        let sr = self.rate();
        let limit = tempo.seconds_to_tick(MAX_AUDITION_SECONDS).0;
        let clipped: Vec<MidiNoteEvent> = events
            .iter()
            .filter(|e| e.start_tick < limit)
            .map(|e| MidiNoteEvent {
                duration_ticks: e.duration_ticks.min(limit - e.start_tick),
                ..*e
            })
            .collect();
        let buf = self
            .deps()
            .synth
            .render(&clipped, tempo, sr, tail_seconds)?;
        let max_frames = (MAX_AUDITION_SECONDS * sr as f64) as usize;
        Ok(encode_wav_pcm16(&truncate(buf.channels, max_frames), sr))
    }

    /// Audição de uma progressão realizada (tempo constante = `realization.bpm`).
    pub fn audition_realization(&self, r: &super::Realization) -> AppResult<Vec<u8>> {
        let tempo =
            TempoMap::new(r.bpm, 4, 4).map_err(|e| invalid("invalid_bpm", e.to_string()))?;
        self.audition_midi(&r.events, &tempo, AUDITION_TAIL_SECONDS)
    }

    /// "Arrastar progressão para a timeline": um único desfazer remove fonte, clipe e faixa nova.
    pub fn insert_progression(
        &mut self,
        composer: &Composer,
        req: &RealizeRequest,
        track: Option<TrackId>,
        at: Tick,
    ) -> AppResult<InsertedClip> {
        if at.0 < 0 {
            return Err(invalid("invalid_position", "Posição negativa."));
        }
        let bpm = self.project_bpm_at(at).ok_or_else(no_project)?;
        let mut req = req.clone();
        req.bpm = Some(bpm);
        let r = composer.realize(&req, bpm)?;
        let tempo = self.tempo_from(at)?;
        let buf = self
            .deps()
            .synth
            .render(&r.events, &tempo, self.rate(), INSERT_TAIL_SECONDS)?;
        self.add_rendered_clip(
            buf.channels,
            &format!("Progressão – {}", r.name),
            track,
            at,
            "Inserir progressão",
        )
    }

    /// Divergência entre a base e o projeto no tick `at`.
    pub fn base_mismatch(
        &self,
        composer: &Composer,
        base_id: BaseId,
        at: Tick,
        project_key: Option<BaseKey>,
    ) -> AppResult<MismatchReport> {
        let base = composer.base(base_id)?;
        let bpm = self.project_bpm_at(at).ok_or_else(no_project)?;
        mismatch(base.bpm, base.key, bpm, project_key).map_err(library_err)
    }

    /// Audição de uma base (gerada: sintetizada no BPM dela; áudio: arquivo decodificado).
    pub fn audition_base(&self, composer: &Composer, base_id: BaseId) -> AppResult<Vec<u8>> {
        let base = composer.base(base_id)?;
        match &base.kind {
            BaseKind::Generated(_) => {
                let g = composer.generated_base_events(base, 0)?;
                let tempo =
                    TempoMap::new(base.bpm, base.meter.numerator(), base.meter.denominator())
                        .map_err(|e| invalid("invalid_bpm", e.to_string()))?;
                self.audition_midi(&generated_events(&g), &tempo, AUDITION_TAIL_SECONDS)
            }
            _ => {
                let file = composer
                    .base_audio_file(base)
                    .ok_or_else(|| invalid("internal", "Base sem arquivo."))?;
                let sr = self.rate();
                let buf = self.deps().media.decode(&file, sr)?;
                let max_frames = (MAX_AUDITION_SECONDS * sr as f64) as usize;
                Ok(encode_wav_pcm16(&truncate(buf.channels, max_frames), sr))
            }
        }
    }

    /// Insere uma base no projeto. Com divergência e sem `decision`, **não altera nada** e devolve
    /// `NeedsDecision { report }` para a UI perguntar.
    pub fn insert_base(
        &mut self,
        composer: &Composer,
        req: &InsertBaseRequest,
    ) -> AppResult<InsertBaseOutcome> {
        if req.at.0 < 0 {
            return Err(invalid("invalid_position", "Posição negativa."));
        }
        let report = self.base_mismatch(composer, req.base_id, req.at, req.project_key)?;
        let decision = match (report.needs_decision, req.decision) {
            (true, None) => return Ok(InsertBaseOutcome::NeedsDecision { report }),
            (true, Some(d)) => d,
            (false, _) => BaseDecision::Keep,
        };
        let base = composer.base(req.base_id)?.clone();
        let label = "Inserir base";
        let sr = self.rate();
        let clip = match (&base.kind, decision) {
            (BaseKind::Generated(_), BaseDecision::Keep) => {
                let g = composer.generated_base_events(&base, 0)?;
                let tempo =
                    TempoMap::new(base.bpm, base.meter.numerator(), base.meter.denominator())
                        .map_err(|e| invalid("invalid_bpm", e.to_string()))?;
                let buf = self.deps().synth.render(
                    &generated_events(&g),
                    &tempo,
                    sr,
                    INSERT_TAIL_SECONDS,
                )?;
                self.add_rendered_clip(buf.channels, &base.name, req.track, req.at, label)?
            }
            (BaseKind::Generated(_), BaseDecision::Adapt) => {
                // base gerada: re-sintetiza no tempo do projeto e no tom transposto (sem perdas)
                let g =
                    composer.generated_base_events(&base, report.semitone_shift.unwrap_or(0))?;
                let tempo = self.tempo_from(req.at)?;
                let buf = self.deps().synth.render(
                    &generated_events(&g),
                    &tempo,
                    sr,
                    INSERT_TAIL_SECONDS,
                )?;
                self.add_rendered_clip(
                    buf.channels,
                    &format!("{} (ajustada)", base.name),
                    req.track,
                    req.at,
                    label,
                )?
            }
            (_, BaseDecision::Keep) => {
                let file = composer
                    .base_audio_file(&base)
                    .ok_or_else(|| invalid("internal", "Base sem arquivo."))?;
                let root = self.project_root().ok_or_else(no_project)?.to_path_buf();
                let m = self.deps().media.import(&file, &root, sr)?;
                if m.frames == 0 {
                    return Err(invalid("empty_audio", "O arquivo da base não tem áudio."));
                }
                let existing = self
                    .session()
                    .and_then(|s| s.project.source_by_hash(&m.content_hash).map(|s| s.id));
                let plan = match existing {
                    Some(id) => SourcePlan::Existing(id, m.frames),
                    None => {
                        let rel = RelPath::new(m.rel_path.clone()).map_err(project_err)?;
                        let buf = self.deps().media.decode(&safe_join(&root, &rel)?, sr)?;
                        SourcePlan::New(
                            AudioSource {
                                id: AudioSourceId::new(),
                                content_hash: m.content_hash,
                                rel_path: rel,
                                original_name: m.original_name,
                                channels: m.channels,
                                length_frames: m.frames,
                            },
                            buf,
                        )
                    }
                };
                self.place_clip(plan, &base.name, req.track, req.at, label)?
            }
            (_, BaseDecision::Adapt) => {
                let file = composer
                    .base_audio_file(&base)
                    .ok_or_else(|| invalid("internal", "Base sem arquivo."))?;
                let buf = self.deps().media.decode(&file, sr)?;
                let planes = adapt_audio(buf.channels, &report)?;
                self.add_rendered_clip(
                    planes,
                    &format!("{} (ajustada)", base.name),
                    req.track,
                    req.at,
                    label,
                )?
            }
        };
        Ok(InsertBaseOutcome::Inserted {
            report,
            decision,
            clip: Box::new(clip),
        })
    }

    /// Grava `audio/generated/<id>.wav` e registra fonte + clipe.
    fn add_rendered_clip(
        &mut self,
        planes: Vec<Vec<f32>>,
        name: &str,
        track: Option<TrackId>,
        at: Tick,
        label: &str,
    ) -> AppResult<InsertedClip> {
        let root = self.project_root().ok_or_else(no_project)?.to_path_buf();
        let sr = self.rate();
        let frames = planes.iter().map(Vec::len).min().unwrap_or(0) as u64;
        if frames == 0 {
            return Err(invalid("empty_audio", "Nada para inserir (áudio vazio)."));
        }
        std::fs::create_dir_all(root.join(GENERATED_AUDIO_DIR)).map_err(io)?;
        let id = AudioSourceId::new();
        let rel = RelPath::new(format!("{GENERATED_AUDIO_DIR}/{id}.wav")).map_err(project_err)?;
        let file = safe_join(&root, &rel)?;
        let settings = ExportSettings {
            format: ExportFormatKind::Wav,
            bit_depth: 24,
            normalize: false,
            ..ExportSettings::default()
        };
        self.deps().media.encode(&file, &settings, sr, &planes)?;
        let hash = self.deps().media.hash_file(&file)?;
        // mesmo conteúdo já no projeto (ex.: a mesma progressão inserida de novo): reutiliza
        let existing = self
            .session()
            .and_then(|s| s.project.source_by_hash(&hash).map(|s| s.id));
        if let Some(id) = existing {
            let _ = std::fs::remove_file(&file);
            return self.place_clip(SourcePlan::Existing(id, frames), name, track, at, label);
        }
        let src = AudioSource {
            id,
            content_hash: hash,
            rel_path: rel,
            original_name: format!("{}.wav", track_name(name)),
            channels: planes.len() as u16,
            length_frames: frames,
        };
        let planes = truncate(planes, frames as usize);
        self.place_clip(
            SourcePlan::New(src, SourceBuffer { channels: planes }),
            name,
            track,
            at,
            label,
        )
    }

    /// Fonte (nova ou existente) + faixa (dada ou nova) + clipe: um único comando composto.
    fn place_clip(
        &mut self,
        plan: SourcePlan,
        name: &str,
        track: Option<TrackId>,
        at: Tick,
        label: &str,
    ) -> AppResult<InsertedClip> {
        let mut steps: Vec<Box<dyn Command<Session>>> = Vec::new();
        let mut buffers = Vec::new();
        let (source_id, frames, rel_path) = match plan {
            SourcePlan::New(src, buf) => {
                let out = (src.id, src.length_frames, src.rel_path.as_str().to_owned());
                buffers.push((src.id, buf));
                steps.push(on_project(AddSource::new(src)));
                out
            }
            SourcePlan::Existing(id, frames) => {
                let s = self.session().ok_or_else(no_project)?;
                let rel = s
                    .project
                    .source(id)
                    .map_err(project_err)?
                    .rel_path
                    .as_str()
                    .to_owned();
                (id, frames, rel)
            }
        };
        let target = match track {
            Some(t) => {
                let s = self.session().ok_or_else(no_project)?;
                s.project.track(t).map_err(project_err)?;
                t
            }
            None => {
                let t = Track::new_audio(&track_name(name)).map_err(project_err)?;
                let id = t.id;
                steps.push(on_project(CreateTrack::new(t, usize::MAX)));
                steps.push(on_mixer(AddStrip::new(id, ChannelStrip::default())));
                id
            }
        };
        let clip_id = ClipId::new();
        steps.push(on_project(AddClip::new(
            target,
            Clip {
                id: clip_id,
                source_id,
                start: at,
                offset_frames: 0,
                length_frames: frames,
                gain: Db::UNITY,
            },
        )));
        self.register_source_buffers(buffers);
        let view = self.execute(Box::new(Composite::new(label, steps)), None)?;
        Ok(InsertedClip {
            track_id: target,
            clip_id,
            source_id,
            rel_path,
            frames,
            view,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(ratio: f64, shift: Option<i8>) -> MismatchReport {
        MismatchReport {
            bpm_ratio: ratio,
            semitone_shift: shift,
            mode_differs: false,
            needs_decision: true,
        }
    }

    #[test]
    fn adapt_changes_length_by_bpm_ratio() {
        let sine: Vec<f32> = (0..48_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.5)
            .collect();
        for (base, project) in [(100.0, 120.0), (120.0, 90.0)] {
            let out = adapt_audio(
                vec![sine.clone(), sine.clone()],
                &report(project / base, Some(2)),
            )
            .unwrap();
            let expected = 48_000.0 * base / project;
            assert_eq!(out.len(), 2);
            assert!(((out[0].len() as f64) - expected).abs() / expected < 0.01);
            assert_eq!(out[0].len(), out[1].len());
        }
        assert!(adapt_audio(vec![sine.clone()], &report(10.0, None)).is_err());
        let same = adapt_audio(vec![sine.clone()], &report(1.0, Some(0))).unwrap();
        assert_eq!(same[0], sine);
    }
}
