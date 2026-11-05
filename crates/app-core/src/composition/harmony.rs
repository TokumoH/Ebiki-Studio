//! Casos de uso de Harmonia (RF-5.1 a RF-5.7): tons/modos, campo harmônico, sugestões,
//! progressões do usuário e realização em notas MIDI ([`MidiNoteEvent`]).

use super::library::{Composer, OnHarmony};
use super::{harmony_err, invalid};
use crate::ports::MidiNoteEvent;
use crate::studio::AppResult;
use ctx_harmony::commands::{AddUserProgression, EditUserProgression, RemoveUserProgression};
use ctx_harmony::{
    catalog::find_builtin, harmonic_field, Chord, Genre, Key, Mode, Notation, Progression,
    ProgressionId, ProgressionKey, ProgressionQuery, RomanChord, SuggestionReason, TimedChord,
    VoicingOptions,
};
use serde::{Deserialize, Serialize};
use shared_kernel::music::PitchClass;
use shared_kernel::tempo::{MAX_BPM, MIN_BPM};
use shared_kernel::{Tick, PPQ};
use std::collections::BTreeSet;

/// Compassos máximos numa realização (≈ 17 min a 60 BPM em 4/4).
pub const MAX_REALIZE_BARS: u16 = 256;
/// Canais/programas General MIDI padrão da realização.
pub const CHORD_CHANNEL: u8 = 0;
pub const BASS_CHANNEL: u8 = 1;

/// Como a UI se refere a uma progressão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum ProgressionRef {
    /// Do catálogo embutido.
    Builtin { slug: String },
    /// Do usuário (UUID).
    User { id: String },
    /// Graus romanos avulsos (`"ii7 V7 Imaj7:8"`) relativos ao `mode`.
    Roman {
        steps: String,
        mode: Mode,
        #[serde(default = "four")]
        beats_per_bar: u8,
    },
    /// Cifras avulsas (`"C:4 Am:4 F G7"`); sem `:tempos` dura um compasso.
    Cifra {
        chords: String,
        #[serde(default = "four")]
        beats_per_bar: u8,
    },
}

fn four() -> u8 {
    4
}

/// Progressão resolvida num tom: acordes absolutos com duração.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProgression {
    pub name: String,
    pub beats_per_bar: u8,
    pub chords: Vec<TimedChord>,
}

// ---------------------------------------------------------------- visões (IPC)

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TonicView {
    pub value: u8,
    pub sharp: &'static str,
    pub flat: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeView {
    /// Valor a enviar de volta na IPC (`Key.mode`).
    pub mode: Mode,
    pub code: &'static str,
    pub name_pt: &'static str,
    pub is_minor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreView {
    pub genre: Genre,
    pub code: &'static str,
    pub name_pt: &'static str,
}

/// Tônicas, modos e gêneros disponíveis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysCatalog {
    pub tonics: Vec<TonicView>,
    pub modes: Vec<ModeView>,
    pub genres: Vec<GenreView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordLabel {
    pub symbol: String,
    pub roman: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDegreeView {
    pub degree: u8,
    pub triad: ChordLabel,
    pub seventh: ChordLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarmonicFieldView {
    pub key: Key,
    pub label: String,
    pub prefer_flats: bool,
    pub degrees: Vec<FieldDegreeView>,
}

/// Progressão para listagem (embutida ou do usuário).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionView {
    #[serde(rename = "ref")]
    pub reference: ProgressionRef,
    pub name: String,
    /// `"builtin"` | `"user"`.
    pub origin: &'static str,
    pub mode: Mode,
    pub genres: Vec<Genre>,
    pub universal: bool,
    pub beats_per_bar: u8,
    pub steps_text: String,
    pub roman: Vec<String>,
    pub beats: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionSuggestionView {
    pub progression: ProgressionView,
    pub score: f32,
    /// Cifras no tom pedido.
    pub cifra: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordSuggestionView {
    pub symbol: String,
    pub roman: String,
    pub score: f32,
    pub reason: SuggestionReason,
    pub reason_label: &'static str,
}

/// Dados de uma progressão do usuário (cadastro/edição).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProgressionInput {
    pub name: String,
    pub mode: Mode,
    #[serde(default)]
    pub genres: BTreeSet<Genre>,
    #[serde(default = "four")]
    pub beats_per_bar: u8,
    /// Graus romanos `"I:4 V:4 vi IV"`.
    pub steps: String,
}

/// Opções de voicing/timbre da realização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VoicingRequest {
    /// Nota MIDI mais grave das vozes (padrão Mi3 = 52).
    pub low: u8,
    /// Nota MIDI mais aguda (padrão Mi5 = 76); `high − low ≥ 12`.
    pub high: u8,
    pub voice_leading: bool,
    /// Gera a linha de baixo (Dó2–Mi3).
    pub bass: bool,
    /// Programa GM das vozes (0 = piano).
    pub program: u8,
    /// Programa GM do baixo (33 = baixo elétrico).
    pub bass_program: u8,
    pub velocity: u8,
}

impl Default for VoicingRequest {
    fn default() -> Self {
        Self {
            low: 52,
            high: 76,
            voice_leading: true,
            bass: true,
            program: 0,
            bass_program: 33,
            velocity: 90,
        }
    }
}

/// Pedido de realização.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizeRequest {
    pub progression: ProgressionRef,
    pub key: Key,
    /// `None` = tempo do projeto (ou 120).
    #[serde(default)]
    pub bpm: Option<f64>,
    /// Repete a progressão até preencher `bars` compassos (`None` = uma vez).
    #[serde(default)]
    pub bars: Option<u16>,
    #[serde(default)]
    pub voicing: VoicingRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizedChordView {
    pub symbol: String,
    pub start_tick: i64,
    pub duration_ticks: i64,
    pub notes: Vec<u8>,
    pub bass: Option<u8>,
}

/// Resultado da realização (ticks com `PPQ` = 960; início em 0).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Realization {
    pub name: String,
    pub key: Key,
    pub bpm: f64,
    pub beats_per_bar: u8,
    pub total_ticks: i64,
    pub chords: Vec<RealizedChordView>,
    pub events: Vec<MidiNoteEvent>,
}

pub(crate) fn check_bpm(bpm: f64) -> AppResult<f64> {
    if bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm) {
        Ok(bpm)
    } else {
        Err(invalid("invalid_bpm", "BPM inválido: use de 20 a 999."))
    }
}

fn progression_view(p: &Progression) -> ProgressionView {
    let reference = match &p.key {
        ProgressionKey::Builtin(slug) => ProgressionRef::Builtin { slug: slug.clone() },
        ProgressionKey::User(id) => ProgressionRef::User { id: id.to_string() },
    };
    ProgressionView {
        reference,
        name: p.name.clone(),
        origin: if p.key.is_user() { "user" } else { "builtin" },
        mode: p.mode,
        genres: p.genres.iter().copied().collect(),
        universal: p.universal,
        beats_per_bar: p.beats_per_bar,
        steps_text: p.steps_text(),
        roman: p.steps.iter().map(|s| s.chord.format(p.mode)).collect(),
        beats: p.steps.iter().map(|s| s.beats).collect(),
    }
}

fn parse_user_id(id: &str) -> AppResult<ProgressionId> {
    id.parse()
        .map_err(|_| invalid("invalid_id", "Identificador de progressão inválido."))
}

/// Tônicas, modos (21) e gêneros (15).
pub fn keys_catalog() -> KeysCatalog {
    KeysCatalog {
        tonics: (0..12u8)
            .filter_map(PitchClass::new)
            .map(|pc| TonicView {
                value: pc.value(),
                sharp: pc.name(false),
                flat: pc.name(true),
            })
            .collect(),
        modes: Mode::ALL
            .iter()
            .map(|m| ModeView {
                mode: *m,
                code: m.code(),
                name_pt: m.name_pt(),
                is_minor: m.is_minor(),
            })
            .collect(),
        genres: Genre::ALL
            .iter()
            .map(|g| GenreView {
                genre: *g,
                code: g.code(),
                name_pt: g.name_pt(),
            })
            .collect(),
    }
}

/// Campo harmônico (tríades e tétrades) com cifra e grau romano.
pub fn field_view(key: Key) -> HarmonicFieldView {
    let flats = key.prefer_flats();
    let label = |c: &Chord| ChordLabel {
        symbol: c.symbol(flats),
        roman: RomanChord::analyze(c, key).format(key.mode),
    };
    HarmonicFieldView {
        key,
        label: key.to_string(),
        prefer_flats: flats,
        degrees: harmonic_field(key)
            .iter()
            .map(|d| FieldDegreeView {
                degree: d.degree,
                triad: label(&d.triad),
                seventh: label(&d.seventh),
            })
            .collect(),
    }
}

/// Expande a progressão ciclicamente até `bars` compassos (corta o último acorde no fim).
pub fn fill_bars(
    chords: &[TimedChord],
    beats_per_bar: u8,
    bars: u16,
) -> AppResult<Vec<TimedChord>> {
    if !(1..=MAX_REALIZE_BARS).contains(&bars) {
        return Err(invalid(
            "invalid_bars",
            format!("Número de compassos inválido: use de 1 a {MAX_REALIZE_BARS}."),
        ));
    }
    if chords.is_empty() || chords.iter().any(|c| c.beats == 0) {
        return Err(invalid("empty_progression", "Progressão vazia."));
    }
    let mut remaining = bars as u32 * beats_per_bar.max(1) as u32;
    let mut out = Vec::new();
    for c in chords.iter().cycle() {
        if remaining == 0 {
            break;
        }
        let beats = (c.beats as u32).min(remaining);
        out.push(TimedChord {
            chord: c.chord.clone(),
            beats: beats as u16,
        });
        remaining -= beats;
    }
    Ok(out)
}

fn parse_cifra(text: &str, beats_per_bar: u8) -> AppResult<Vec<TimedChord>> {
    let chords = text
        .split_whitespace()
        .filter(|t| *t != "|")
        .map(|tok| {
            let (sym, beats) = match tok.rsplit_once(':') {
                Some((s, b)) => (
                    s,
                    b.parse::<u16>()
                        .ok()
                        .filter(|b| (1..=64).contains(b))
                        .ok_or_else(|| invalid("invalid_duration", "Duração inválida."))?,
                ),
                None => (tok, beats_per_bar as u16),
            };
            Ok(TimedChord {
                chord: Chord::parse(sym).map_err(harmony_err)?,
                beats,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    if chords.is_empty() || chords.len() > 64 {
        return Err(invalid("empty_progression", "Informe de 1 a 64 acordes."));
    }
    Ok(chords)
}

fn timed(p: &Progression, key: Key) -> AppResult<Vec<TimedChord>> {
    Ok(p.chords_in(key)
        .map_err(harmony_err)?
        .into_iter()
        .zip(&p.steps)
        .map(|(chord, s)| TimedChord {
            chord,
            beats: s.beats,
        })
        .collect())
}

impl Composer {
    /// Resolve a referência no tom pedido (o modo de uma progressão do catálogo é o dela).
    pub fn resolve_progression(
        &self,
        r: &ProgressionRef,
        key: Key,
    ) -> AppResult<ResolvedProgression> {
        let from = |p: &Progression| -> AppResult<ResolvedProgression> {
            Ok(ResolvedProgression {
                name: p.name.clone(),
                beats_per_bar: p.beats_per_bar,
                chords: timed(p, key)?,
            })
        };
        match r {
            ProgressionRef::Builtin { slug } => from(
                find_builtin(slug)
                    .ok_or_else(|| harmony_err(ctx_harmony::HarmonyError::ProgressionNotFound))?,
            ),
            ProgressionRef::User { id } => from(
                self.library
                    .harmony
                    .get(parse_user_id(id)?)
                    .ok_or_else(|| harmony_err(ctx_harmony::HarmonyError::ProgressionNotFound))?,
            ),
            ProgressionRef::Roman {
                steps,
                mode,
                beats_per_bar,
            } => {
                let p = Progression {
                    key: ProgressionKey::User(ProgressionId::new()),
                    name: "Progressão".into(),
                    mode: *mode,
                    genres: BTreeSet::new(),
                    universal: false,
                    beats_per_bar: *beats_per_bar,
                    steps: Progression::parse_steps(steps, *mode, *beats_per_bar)
                        .map_err(harmony_err)?,
                };
                p.validate().map_err(harmony_err)?;
                from(&p)
            }
            ProgressionRef::Cifra {
                chords,
                beats_per_bar,
            } => {
                if !(1..=16).contains(beats_per_bar) {
                    return Err(harmony_err(ctx_harmony::HarmonyError::InvalidBeatsPerBar));
                }
                Ok(ResolvedProgression {
                    name: "Progressão".into(),
                    beats_per_bar: *beats_per_bar,
                    chords: parse_cifra(chords, *beats_per_bar)?,
                })
            }
        }
    }

    /// Progressões ranqueadas por tom + gênero (RF-5.3), com cifras no tom.
    pub fn suggest_progressions(
        &self,
        key: Key,
        genre: Option<Genre>,
        limit: usize,
    ) -> Vec<ProgressionSuggestionView> {
        let q = ProgressionQuery {
            key,
            genre,
            limit: limit.clamp(1, 100),
        };
        self.provider()
            .suggest_progressions(&q)
            .into_iter()
            .map(|s| ProgressionSuggestionView {
                cifra: s
                    .progression
                    .format(key, Notation::Cifra)
                    .unwrap_or_default(),
                progression: progression_view(&s.progression),
                score: s.score,
            })
            .collect()
    }

    /// Próximos acordes a partir do histórico em cifra (o último é o atual) — RF-5.4.
    pub fn suggest_next(
        &self,
        key: Key,
        history: &[String],
        limit: usize,
    ) -> AppResult<Vec<ChordSuggestionView>> {
        if history.len() > 64 {
            return Err(invalid(
                "history_too_long",
                "Histórico de acordes longo demais.",
            ));
        }
        let chords = history
            .iter()
            .map(|s| Chord::parse(s).map_err(harmony_err))
            .collect::<AppResult<Vec<_>>>()?;
        let flats = key.prefer_flats();
        Ok(self
            .provider()
            .suggest_next_chords(key, &chords, limit.clamp(1, 50))
            .into_iter()
            .map(|s| ChordSuggestionView {
                symbol: s.chord.symbol(flats),
                roman: s.roman.format(key.mode),
                score: s.score,
                reason: s.reason,
                reason_label: s.reason.label_pt(),
            })
            .collect())
    }

    /// Realiza a progressão em notas (vozes no canal 0, baixo no canal 1), início no tick 0.
    pub fn realize(&self, req: &RealizeRequest, default_bpm: f64) -> AppResult<Realization> {
        let bpm = check_bpm(req.bpm.unwrap_or(default_bpm))?;
        let v = req.voicing;
        if v.program > 127 || v.bass_program > 127 {
            return Err(invalid(
                "invalid_program",
                "Programa General MIDI inválido.",
            ));
        }
        let resolved = self.resolve_progression(&req.progression, req.key)?;
        let chords = match req.bars {
            Some(bars) => fill_bars(&resolved.chords, resolved.beats_per_bar, bars)?,
            None => resolved.chords,
        };
        let opts = VoicingOptions {
            low: v.low,
            high: v.high,
            voice_leading: v.voice_leading,
            bass_range: v.bass.then_some((36, 52)),
            start: Tick::ZERO,
        };
        let realized = self
            .provider()
            .realize(&chords, &opts)
            .map_err(harmony_err)?;
        let flats = req.key.prefer_flats();
        let vel = v.velocity.clamp(1, 127);
        let mut events = Vec::new();
        let mut views = Vec::with_capacity(realized.len());
        for rc in &realized {
            for n in &rc.notes {
                events.push(MidiNoteEvent {
                    start_tick: rc.start.0,
                    duration_ticks: rc.duration.0,
                    channel: CHORD_CHANNEL,
                    program: v.program,
                    key: n.0,
                    velocity: vel,
                });
            }
            if let Some(b) = rc.bass {
                events.push(MidiNoteEvent {
                    start_tick: rc.start.0,
                    duration_ticks: rc.duration.0,
                    channel: BASS_CHANNEL,
                    program: v.bass_program,
                    key: b.0,
                    velocity: vel.saturating_add(5).min(127),
                });
            }
            views.push(RealizedChordView {
                symbol: rc.chord.symbol(flats),
                start_tick: rc.start.0,
                duration_ticks: rc.duration.0,
                notes: rc.notes.iter().map(|n| n.0).collect(),
                bass: rc.bass.map(|b| b.0),
            });
        }
        let total_ticks = chords.iter().map(|c| c.beats as i64 * PPQ).sum();
        Ok(Realization {
            name: resolved.name,
            key: req.key,
            bpm,
            beats_per_bar: resolved.beats_per_bar,
            total_ticks,
            chords: views,
            events,
        })
    }

    // ------------------------------------------------ progressões do usuário (RF-5.2)

    pub fn user_progressions(&self) -> Vec<ProgressionView> {
        self.library
            .harmony
            .user_progressions()
            .iter()
            .map(progression_view)
            .collect()
    }

    pub fn add_user_progression(
        &mut self,
        input: &UserProgressionInput,
    ) -> AppResult<ProgressionView> {
        let p = Progression::new_user(
            &input.name,
            input.mode,
            input.genres.clone(),
            input.beats_per_bar,
            &input.steps,
        )
        .map_err(harmony_err)?;
        let view = progression_view(&p);
        self.execute(Box::new(OnHarmony(Box::new(AddUserProgression::new(
            p,
            usize::MAX,
        )))))?;
        Ok(view)
    }

    pub fn edit_user_progression(
        &mut self,
        id: &str,
        input: &UserProgressionInput,
    ) -> AppResult<ProgressionView> {
        let id = parse_user_id(id)?;
        let mut p = Progression::new_user(
            &input.name,
            input.mode,
            input.genres.clone(),
            input.beats_per_bar,
            &input.steps,
        )
        .map_err(harmony_err)?;
        p.key = ProgressionKey::User(id);
        let view = progression_view(&p);
        self.execute(Box::new(OnHarmony(Box::new(EditUserProgression::new(p)))))?;
        Ok(view)
    }

    pub fn remove_user_progression(&mut self, id: &str) -> AppResult<()> {
        let id = parse_user_id(id)?;
        self.execute(Box::new(OnHarmony(Box::new(RemoveUserProgression::new(
            id,
        )))))
    }
}
