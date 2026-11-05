//! Modelo do contexto Tuning (puro, sem I/O).
//!
//! Agregados:
//! - [`PitchCorrectionMap`]: por fonte de áudio — notas detectadas (value objects), edições
//!   manuais nota a nota e correção automática por escala.
//! - [`TunerSettings`]: referência do Lá4 do afinador.

use serde::{Deserialize, Serialize};
use shared_kernel::command::CommandError;
use shared_kernel::define_id;
use shared_kernel::id::AudioSourceId;
use shared_kernel::music::{MidiNote, PitchClass};
use std::collections::BTreeMap;

define_id!(
    /// Identificador de um mapa de correção de afinação.
    PitchCorrectionMapId
);
define_id!(
    /// Identificador de uma nota detectada (estável entre edições).
    DetectedNoteId
);

/// Limites do domínio.
pub const A4_MIN_HZ: f64 = 415.0;
pub const A4_MAX_HZ: f64 = 466.0;
pub const MAX_CENTS_OFFSET: f64 = 1200.0;
pub const MAX_FORMANT_SEMITONES: f64 = 12.0;
pub const MAX_RETUNE_MS: f64 = 1000.0;

/// Erros do contexto Tuning.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TuningError {
    #[error("intervalo de nota inválido (início deve ser menor que o fim)")]
    InvalidNoteRange,
    #[error("notas detectadas fora de ordem ou sobrepostas")]
    OverlappingNotes,
    #[error("identificador de nota repetido")]
    DuplicateNote,
    #[error("altura inválida: deve estar entre 0 e 127")]
    InvalidPitch,
    #[error("nota não encontrada")]
    NoteNotFound,
    #[error("deslocamento em cents fora do intervalo ±{MAX_CENTS_OFFSET}")]
    InvalidCentsOffset,
    #[error("valor de {0} fora do intervalo permitido")]
    InvalidAmount(&'static str),
    #[error("a escala precisa de ao menos uma nota e deve conter a tônica")]
    InvalidScale,
    #[error("velocidade de correção fora do intervalo 0–{MAX_RETUNE_MS} ms")]
    InvalidRetuneSpeed,
    #[error("referência do Lá4 fora do intervalo {A4_MIN_HZ}–{A4_MAX_HZ} Hz")]
    InvalidReference,
}

impl TuningError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidNoteRange => "tuning.invalid_note_range",
            Self::OverlappingNotes => "tuning.overlapping_notes",
            Self::DuplicateNote => "tuning.duplicate_note",
            Self::InvalidPitch => "tuning.invalid_pitch",
            Self::NoteNotFound => "tuning.note_not_found",
            Self::InvalidCentsOffset => "tuning.invalid_cents_offset",
            Self::InvalidAmount(_) => "tuning.invalid_amount",
            Self::InvalidScale => "tuning.invalid_scale",
            Self::InvalidRetuneSpeed => "tuning.invalid_retune_speed",
            Self::InvalidReference => "tuning.invalid_reference",
        }
    }
}

impl From<TuningError> for CommandError {
    fn from(e: TuningError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

fn in_range(v: f64, lo: f64, hi: f64) -> bool {
    v.is_finite() && (lo..=hi).contains(&v)
}

/// Nota detectada pela análise (value object; `end_frame` exclusivo, em quadros da fonte).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedNote {
    pub id: DetectedNoteId,
    pub start_frame: u64,
    pub end_frame: u64,
    /// Altura média detectada (MIDI fracionário).
    pub midi: f64,
}

impl DetectedNote {
    pub fn new(start_frame: u64, end_frame: u64, midi: f64) -> Result<Self, TuningError> {
        let n = Self {
            id: DetectedNoteId::new(),
            start_frame,
            end_frame,
            midi,
        };
        n.validate()?;
        Ok(n)
    }

    pub fn validate(&self) -> Result<(), TuningError> {
        if self.start_frame >= self.end_frame {
            return Err(TuningError::InvalidNoteRange);
        }
        if !in_range(self.midi, 0.0, 127.0) {
            return Err(TuningError::InvalidPitch);
        }
        Ok(())
    }

    /// Desvio em cents em relação à nota inteira mais próxima.
    pub fn deviation_cents(&self) -> f64 {
        (self.midi - self.midi.round()) * 100.0
    }

    pub fn nearest_note(&self) -> MidiNote {
        MidiNote(self.midi.round().clamp(0.0, 127.0) as u8)
    }
}

/// Edição manual de uma nota.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEdit {
    /// Nota-alvo (se definida, a nota é levada até ela).
    pub target: Option<MidiNote>,
    /// Ajuste fino adicional em cents (±1200).
    pub cents_offset: f64,
    /// 0–1: achatamento da deriva/vibrato em direção à média da nota.
    pub flatten: f64,
    /// Deslocamento de formante em semitons (±12; 0 = natural).
    pub formant_semitones: f64,
}

impl Default for NoteEdit {
    fn default() -> Self {
        Self {
            target: None,
            cents_offset: 0.0,
            flatten: 0.0,
            formant_semitones: 0.0,
        }
    }
}

impl NoteEdit {
    pub fn to_target(note: MidiNote) -> Self {
        Self {
            target: Some(note),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), TuningError> {
        if self.target.is_some_and(|t| t.0 > 127) {
            return Err(TuningError::InvalidPitch);
        }
        if !in_range(self.cents_offset, -MAX_CENTS_OFFSET, MAX_CENTS_OFFSET) {
            return Err(TuningError::InvalidCentsOffset);
        }
        if !in_range(self.flatten, 0.0, 1.0) {
            return Err(TuningError::InvalidAmount("achatamento"));
        }
        if !in_range(
            self.formant_semitones,
            -MAX_FORMANT_SEMITONES,
            MAX_FORMANT_SEMITONES,
        ) {
            return Err(TuningError::InvalidAmount("formante"));
        }
        Ok(())
    }

    /// Edição sem efeito (equivale a não ter edição).
    pub fn is_neutral(&self) -> bool {
        *self == Self::default()
    }
}

/// Correção automática por escala.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaleCorrection {
    pub enabled: bool,
    /// Tônica (referência da tonalidade).
    pub key: PitchClass,
    /// Classes de altura permitidas (ordenadas, sem repetição; contém a tônica).
    pub allowed: Vec<PitchClass>,
    /// Constante de tempo da correção (0 = instantânea).
    pub retune_speed_ms: f64,
    /// 0–1: fração do desvio natural preservada.
    pub humanize: f64,
}

const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const NATURAL_MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

impl Default for ScaleCorrection {
    fn default() -> Self {
        Self::chromatic(PitchClass::new(0).expect("0 é válido"))
    }
}

impl ScaleCorrection {
    /// Escala a partir da tônica e intervalos (semitons).
    pub fn from_intervals(key: PitchClass, intervals: &[i32]) -> Self {
        let mut allowed: Vec<PitchClass> = intervals.iter().map(|i| key.transpose(*i)).collect();
        allowed.sort();
        allowed.dedup();
        Self {
            enabled: false,
            key,
            allowed,
            retune_speed_ms: 50.0,
            humanize: 0.0,
        }
    }

    pub fn major(key: PitchClass) -> Self {
        Self::from_intervals(key, &MAJOR)
    }

    pub fn natural_minor(key: PitchClass) -> Self {
        Self::from_intervals(key, &NATURAL_MINOR)
    }

    pub fn chromatic(key: PitchClass) -> Self {
        Self::from_intervals(key, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])
    }

    pub fn validate(&self) -> Result<(), TuningError> {
        if self.allowed.is_empty()
            || !self.allowed.contains(&self.key)
            || self.allowed.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(TuningError::InvalidScale);
        }
        if !in_range(self.retune_speed_ms, 0.0, MAX_RETUNE_MS) {
            return Err(TuningError::InvalidRetuneSpeed);
        }
        if !in_range(self.humanize, 0.0, 1.0) {
            return Err(TuningError::InvalidAmount("humanização"));
        }
        Ok(())
    }

    pub fn contains(&self, pc: PitchClass) -> bool {
        self.allowed.binary_search(&pc).is_ok()
    }

    /// Máscara de 12 bits (bit n = classe n) — formato consumido pelo DSP.
    pub fn mask_bits(&self) -> u16 {
        self.allowed.iter().fold(0, |m, pc| m | (1 << pc.value()))
    }

    /// Nota permitida mais próxima de uma altura fracionária.
    pub fn nearest_allowed(&self, midi: f64) -> Option<MidiNote> {
        if !midi.is_finite() {
            return None;
        }
        let center = midi.round() as i32;
        (center - 6..=center + 6)
            .filter(|n| (0..=127).contains(n))
            .filter(|n| self.mask_bits() & (1 << n.rem_euclid(12)) != 0)
            .min_by(|a, b| {
                (*a as f64 - midi)
                    .abs()
                    .total_cmp(&(*b as f64 - midi).abs())
            })
            .map(|n| MidiNote(n as u8))
    }
}

/// Mapa de correção de afinação de uma fonte de áudio (agregado).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchCorrectionMap {
    pub id: PitchCorrectionMapId,
    pub source: AudioSourceId,
    notes: Vec<DetectedNote>,
    edits: BTreeMap<DetectedNoteId, NoteEdit>,
    scale: ScaleCorrection,
}

impl PitchCorrectionMap {
    pub fn new(source: AudioSourceId) -> Self {
        Self {
            id: PitchCorrectionMapId::new(),
            source,
            notes: Vec::new(),
            edits: BTreeMap::new(),
            scale: ScaleCorrection::default(),
        }
    }

    pub fn notes(&self) -> &[DetectedNote] {
        &self.notes
    }

    pub fn edits(&self) -> &BTreeMap<DetectedNoteId, NoteEdit> {
        &self.edits
    }

    pub fn scale(&self) -> &ScaleCorrection {
        &self.scale
    }

    pub fn note(&self, id: DetectedNoteId) -> Result<&DetectedNote, TuningError> {
        self.notes
            .iter()
            .find(|n| n.id == id)
            .ok_or(TuningError::NoteNotFound)
    }

    pub fn edit(&self, id: DetectedNoteId) -> Option<&NoteEdit> {
        self.edits.get(&id)
    }

    fn validate_notes(notes: &[DetectedNote]) -> Result<(), TuningError> {
        for n in notes {
            n.validate()?;
        }
        if notes.windows(2).any(|w| w[0].end_frame > w[1].start_frame) {
            return Err(TuningError::OverlappingNotes);
        }
        let mut ids: Vec<_> = notes.iter().map(|n| n.id).collect();
        ids.sort();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(TuningError::DuplicateNote);
        }
        Ok(())
    }

    /// Invariantes do agregado.
    pub fn validate(&self) -> Result<(), TuningError> {
        Self::validate_notes(&self.notes)?;
        for (id, e) in &self.edits {
            self.note(*id)?;
            e.validate()?;
        }
        self.scale.validate()
    }

    /// Substitui as notas detectadas (nova análise). Edições de notas que deixam de existir
    /// são descartadas. Retorna o estado anterior (notas, edições) para desfazer.
    pub fn replace_notes(
        &mut self,
        mut notes: Vec<DetectedNote>,
    ) -> Result<(Vec<DetectedNote>, BTreeMap<DetectedNoteId, NoteEdit>), TuningError> {
        notes.sort_by_key(|n| n.start_frame);
        Self::validate_notes(&notes)?;
        let kept: BTreeMap<_, _> = self
            .edits
            .iter()
            .filter(|(id, _)| notes.iter().any(|n| n.id == **id))
            .map(|(k, v)| (*k, *v))
            .collect();
        let old_notes = std::mem::replace(&mut self.notes, notes);
        let old_edits = std::mem::replace(&mut self.edits, kept);
        Ok((old_notes, old_edits))
    }

    /// Restaura notas e edições (usado por desfazer).
    pub(crate) fn restore(
        &mut self,
        notes: Vec<DetectedNote>,
        edits: BTreeMap<DetectedNoteId, NoteEdit>,
    ) {
        self.notes = notes;
        self.edits = edits;
    }

    /// Define a edição de uma nota (edição neutra remove). Retorna a anterior.
    pub fn set_edit(
        &mut self,
        id: DetectedNoteId,
        edit: NoteEdit,
    ) -> Result<Option<NoteEdit>, TuningError> {
        self.note(id)?;
        edit.validate()?;
        Ok(if edit.is_neutral() {
            self.edits.remove(&id)
        } else {
            self.edits.insert(id, edit)
        })
    }

    /// Restaura (ou remove) a edição de uma nota sem validação extra (desfazer).
    pub(crate) fn restore_edit(&mut self, id: DetectedNoteId, edit: Option<NoteEdit>) {
        match edit {
            Some(e) => self.edits.insert(id, e),
            None => self.edits.remove(&id),
        };
    }

    pub fn set_scale(&mut self, scale: ScaleCorrection) -> Result<ScaleCorrection, TuningError> {
        scale.validate()?;
        Ok(std::mem::replace(&mut self.scale, scale))
    }

    /// Correção manual efetiva em cents para a nota (alvo + ajuste fino).
    pub fn effective_offset_cents(&self, id: DetectedNoteId) -> Result<f64, TuningError> {
        let note = self.note(id)?;
        Ok(match self.edits.get(&id) {
            None => 0.0,
            Some(e) => e.target.map_or(0.0, |t| (t.0 as f64 - note.midi) * 100.0) + e.cents_offset,
        })
    }

    /// Nota sugerida pela correção por escala (se habilitada).
    pub fn scale_target(&self, id: DetectedNoteId) -> Result<Option<MidiNote>, TuningError> {
        let note = self.note(id)?;
        Ok(self
            .scale
            .enabled
            .then(|| self.scale.nearest_allowed(note.midi))
            .flatten())
    }
}

/// Ajustes do afinador (agregado pequeno, por usuário/projeto).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunerSettings {
    a4_hz: f64,
}

impl Default for TunerSettings {
    fn default() -> Self {
        Self { a4_hz: 440.0 }
    }
}

impl TunerSettings {
    pub fn new(a4_hz: f64) -> Result<Self, TuningError> {
        let mut s = Self::default();
        s.set_a4(a4_hz)?;
        Ok(s)
    }

    pub fn a4_hz(&self) -> f64 {
        self.a4_hz
    }

    /// Define a referência. Retorna a anterior.
    pub fn set_a4(&mut self, a4_hz: f64) -> Result<f64, TuningError> {
        if !in_range(a4_hz, A4_MIN_HZ, A4_MAX_HZ) {
            return Err(TuningError::InvalidReference);
        }
        Ok(std::mem::replace(&mut self.a4_hz, a4_hz))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(s: &str) -> PitchClass {
        PitchClass::parse(s).unwrap()
    }

    fn map_with_notes() -> (PitchCorrectionMap, Vec<DetectedNote>) {
        let mut m = PitchCorrectionMap::new(AudioSourceId::new());
        let notes = vec![
            DetectedNote::new(0, 1000, 60.3).unwrap(),
            DetectedNote::new(1000, 2000, 64.0).unwrap(),
            DetectedNote::new(2500, 4000, 66.8).unwrap(),
        ];
        m.replace_notes(notes.clone()).unwrap();
        (m, notes)
    }

    #[test]
    fn detected_note_validation() {
        assert_eq!(
            DetectedNote::new(10, 10, 60.0),
            Err(TuningError::InvalidNoteRange)
        );
        assert_eq!(
            DetectedNote::new(0, 10, f64::NAN),
            Err(TuningError::InvalidPitch)
        );
        assert_eq!(
            DetectedNote::new(0, 10, 128.0),
            Err(TuningError::InvalidPitch)
        );
        let n = DetectedNote::new(0, 10, 60.3).unwrap();
        assert!((n.deviation_cents() - 30.0).abs() < 1e-9);
        assert_eq!(n.nearest_note(), MidiNote(60));
    }

    #[test]
    fn replace_notes_rejects_overlap_and_duplicates() {
        let mut m = PitchCorrectionMap::new(AudioSourceId::new());
        let a = DetectedNote::new(0, 100, 60.0).unwrap();
        let b = DetectedNote::new(50, 150, 62.0).unwrap();
        assert_eq!(
            m.replace_notes(vec![a, b]),
            Err(TuningError::OverlappingNotes)
        );
        let mut dup = a;
        dup.start_frame = 200;
        dup.end_frame = 300;
        assert_eq!(
            m.replace_notes(vec![a, dup]),
            Err(TuningError::DuplicateNote)
        );
        // Ordena automaticamente.
        let c = DetectedNote::new(100, 200, 62.0).unwrap();
        m.replace_notes(vec![c, a]).unwrap();
        assert_eq!(m.notes()[0], a);
        assert!(m.validate().is_ok());
    }

    #[test]
    fn edits_compute_effective_offset() {
        let (mut m, notes) = map_with_notes();
        assert_eq!(m.effective_offset_cents(notes[0].id).unwrap(), 0.0);
        m.set_edit(notes[0].id, NoteEdit::to_target(MidiNote(60)))
            .unwrap();
        assert!((m.effective_offset_cents(notes[0].id).unwrap() + 30.0).abs() < 1e-9);
        m.set_edit(
            notes[1].id,
            NoteEdit {
                target: Some(MidiNote(65)),
                cents_offset: -10.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!((m.effective_offset_cents(notes[1].id).unwrap() - 90.0).abs() < 1e-9);
        // Edição neutra remove.
        m.set_edit(notes[0].id, NoteEdit::default()).unwrap();
        assert!(m.edit(notes[0].id).is_none());
        assert_eq!(
            m.set_edit(DetectedNoteId::new(), NoteEdit::default()),
            Err(TuningError::NoteNotFound)
        );
    }

    #[test]
    fn edit_validation() {
        let (mut m, notes) = map_with_notes();
        let id = notes[0].id;
        let bad = |e: NoteEdit| m.clone().set_edit(id, e).unwrap_err();
        assert_eq!(
            bad(NoteEdit {
                cents_offset: 1500.0,
                ..Default::default()
            }),
            TuningError::InvalidCentsOffset
        );
        assert_eq!(
            bad(NoteEdit {
                flatten: f64::NAN,
                ..Default::default()
            })
            .code(),
            "tuning.invalid_amount"
        );
        assert_eq!(
            bad(NoteEdit {
                formant_semitones: 13.0,
                ..Default::default()
            })
            .code(),
            "tuning.invalid_amount"
        );
        assert_eq!(
            bad(NoteEdit::to_target(MidiNote(200))),
            TuningError::InvalidPitch
        );
        assert!(m
            .set_edit(
                id,
                NoteEdit {
                    flatten: 1.0,
                    formant_semitones: -12.0,
                    ..Default::default()
                }
            )
            .is_ok());
    }

    #[test]
    fn replacing_notes_drops_orphan_edits() {
        let (mut m, notes) = map_with_notes();
        m.set_edit(notes[0].id, NoteEdit::to_target(MidiNote(60)))
            .unwrap();
        m.set_edit(notes[1].id, NoteEdit::to_target(MidiNote(65)))
            .unwrap();
        m.replace_notes(vec![notes[1]]).unwrap();
        assert_eq!(m.edits().len(), 1);
        assert!(m.edit(notes[1].id).is_some());
        assert!(m.validate().is_ok());
    }

    #[test]
    fn scale_correction_validation_and_target() {
        let (mut m, notes) = map_with_notes();
        let mut bad = ScaleCorrection::major(pc("C"));
        bad.allowed.retain(|p| p.value() != 0);
        assert_eq!(m.set_scale(bad), Err(TuningError::InvalidScale));
        let mut bad = ScaleCorrection::major(pc("C"));
        bad.retune_speed_ms = -1.0;
        assert_eq!(m.set_scale(bad), Err(TuningError::InvalidRetuneSpeed));
        let mut bad = ScaleCorrection::major(pc("C"));
        bad.humanize = 2.0;
        assert!(m.set_scale(bad).is_err());

        let mut s = ScaleCorrection::major(pc("C"));
        assert_eq!(s.mask_bits(), 0b1010_1011_0101);
        assert_eq!(m.scale_target(notes[2].id).unwrap(), None, "desabilitada");
        s.enabled = true;
        m.set_scale(s).unwrap();
        // 66.8 (F#+) em Dó maior → G (67).
        assert_eq!(m.scale_target(notes[2].id).unwrap(), Some(MidiNote(67)));
        assert_eq!(
            ScaleCorrection::natural_minor(pc("A")).allowed,
            ScaleCorrection::major(pc("C")).allowed
        );
        assert!(ScaleCorrection::chromatic(pc("D")).validate().is_ok());
    }

    #[test]
    fn tuner_reference_range() {
        assert!(TunerSettings::new(415.0).is_ok());
        assert!(TunerSettings::new(466.0).is_ok());
        assert_eq!(
            TunerSettings::new(414.9),
            Err(TuningError::InvalidReference)
        );
        assert!(TunerSettings::new(f64::NAN).is_err());
        let mut t = TunerSettings::default();
        assert_eq!(t.set_a4(432.0).unwrap(), 440.0);
        assert_eq!(t.a4_hz(), 432.0);
    }

    #[test]
    fn serde_roundtrip_and_error_codes() {
        let (mut m, notes) = map_with_notes();
        m.set_edit(notes[0].id, NoteEdit::to_target(MidiNote(60)))
            .unwrap();
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("startFrame") && json.contains("retuneSpeedMs"));
        let back: PitchCorrectionMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        let e: CommandError = TuningError::NoteNotFound.into();
        assert_eq!(e.code, "tuning.note_not_found");
        assert_eq!(e.message, "nota não encontrada");
    }
}
