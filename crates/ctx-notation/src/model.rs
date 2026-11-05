//! Agregado [`Score`] e suas entidades/value objects.
//!
//! Invariantes (verificadas por [`Score::check_invariants`]):
//! - todas as partes têm o mesmo número de compassos (≥ 1) e os mesmos atributos de compasso
//!   (armadura, fórmula, repetições) em cada índice;
//! - no máximo [`MAX_VOICES`] vozes por compasso; cada voz aponta para uma pauta existente da parte;
//! - ids de evento são únicos e menores que `next_event_id`; spanners apontam para eventos da parte;
//! - notas dentro da faixa MIDI; tablatura dentro da afinação da pauta.

use crate::duration::Duration;
use crate::error::NotationError;
use crate::frac::Frac;
use crate::pitch::{KeySignature, Pitch};
use serde::{Deserialize, Serialize};
use shared_kernel::tempo::TempoMap;
use shared_kernel::time::Tick;
use std::collections::{BTreeMap, HashSet};

pub const MAX_VOICES: usize = 4;
pub const MAX_STAVES: usize = 2;
pub const MAX_PARTS: usize = 64;
pub const MAX_MEASURES: usize = 10_000;
pub const MAX_EVENTS_PER_VOICE: usize = 1_024;
pub const MAX_NOTES_PER_CHORD: usize = 16;
pub const MAX_STRINGS: usize = 10;
pub const MAX_FRET: u8 = 36;

/// Identidade local (no agregado) de um evento. Estável entre edições; usada por spanners.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct EventId(pub u64);

/// Fórmula de compasso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meter {
    pub numerator: u8,
    pub denominator: u8,
}

impl Meter {
    pub fn new(numerator: u8, denominator: u8) -> Result<Self, NotationError> {
        if !(1..=32).contains(&numerator) || ![1, 2, 4, 8, 16, 32].contains(&denominator) {
            return Err(NotationError::InvalidMeter(numerator, denominator));
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const COMMON: Meter = Meter {
        numerator: 4,
        denominator: 4,
    };

    /// Capacidade do compasso em semibreves.
    pub fn capacity(&self) -> Frac {
        Frac::new(self.numerator as i64, self.denominator.max(1) as i64)
    }

    pub fn ticks(&self) -> i64 {
        self.capacity().to_ticks()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Clef {
    #[default]
    Treble,
    /// Clave de Sol oitava abaixo (violão/guitarra).
    Treble8vb,
    Bass,
    Alto,
    Tenor,
    Percussion,
}

/// Configuração de tablatura: cordas (1 = mais aguda) como notas MIDI soltas, capotraste e trastes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabConfig {
    pub tuning: Vec<u8>,
    #[serde(default)]
    pub capo: u8,
    pub frets: u8,
}

impl TabConfig {
    /// Violão/guitarra padrão (E2 A2 D3 G3 B3 E4), 24 trastes.
    pub fn standard_guitar() -> Self {
        Self {
            tuning: vec![64, 59, 55, 50, 45, 40],
            capo: 0,
            frets: 24,
        }
    }

    /// Baixo de 4 cordas (E1 A1 D2 G2).
    pub fn standard_bass() -> Self {
        Self {
            tuning: vec![43, 38, 33, 28],
            capo: 0,
            frets: 24,
        }
    }

    pub fn validate(&self) -> Result<(), NotationError> {
        if self.tuning.is_empty() || self.tuning.len() > MAX_STRINGS {
            return Err(NotationError::InvalidTab("número de cordas fora de 1–10"));
        }
        if self.tuning.iter().any(|&n| n > 127) {
            return Err(NotationError::InvalidTab("afinação fora da faixa MIDI"));
        }
        if self.frets == 0 || self.frets > MAX_FRET || self.capo >= self.frets {
            return Err(NotationError::InvalidTab("trastes/capotraste inválidos"));
        }
        Ok(())
    }
}

/// Pauta: notação padrão e/ou tablatura (mesmas notas, duas vistas).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Staff {
    pub clef: Clef,
    /// Exibe a pauta de notação padrão.
    pub standard: bool,
    /// Exibe a tablatura (com a afinação).
    pub tab: Option<TabConfig>,
}

impl Staff {
    pub fn standard(clef: Clef) -> Self {
        Self {
            clef,
            standard: true,
            tab: None,
        }
    }

    pub fn standard_and_tab(clef: Clef, tab: TabConfig) -> Self {
        Self {
            clef,
            standard: true,
            tab: Some(tab),
        }
    }

    pub fn tab_only(tab: TabConfig) -> Self {
        Self {
            clef: Clef::Treble8vb,
            standard: false,
            tab: Some(tab),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Articulation {
    Staccato,
    Accent,
    Tenuto,
    Marcato,
    Fermata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Dynamic {
    Ppp,
    Pp,
    P,
    Mp,
    Mf,
    F,
    Ff,
    Fff,
}

impl Dynamic {
    pub const ALL: [Dynamic; 8] = [
        Dynamic::Ppp,
        Dynamic::Pp,
        Dynamic::P,
        Dynamic::Mp,
        Dynamic::Mf,
        Dynamic::F,
        Dynamic::Ff,
        Dynamic::Fff,
    ];

    /// Velocity MIDI equivalente.
    pub const fn velocity(self) -> u8 {
        match self {
            Self::Ppp => 16,
            Self::Pp => 33,
            Self::P => 49,
            Self::Mp => 64,
            Self::Mf => 80,
            Self::F => 96,
            Self::Ff => 112,
            Self::Fff => 127,
        }
    }

    /// Dinâmica mais próxima de uma velocity.
    pub fn from_velocity(v: u8) -> Self {
        Self::ALL
            .into_iter()
            .min_by_key(|d| (d.velocity() as i32 - v as i32).abs())
            .unwrap_or(Dynamic::Mf)
    }

    /// Texto (ppp … fff), igual ao usado em MusicXML.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Ppp => "ppp",
            Self::Pp => "pp",
            Self::P => "p",
            Self::Mp => "mp",
            Self::Mf => "mf",
            Self::F => "f",
            Self::Ff => "ff",
            Self::Fff => "fff",
        }
    }

    pub fn from_symbol(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.symbol() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HarmonicKind {
    Natural,
    Artificial,
}

/// Técnicas de cordas (tablatura).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Technique {
    HammerOn,
    PullOff,
    Slide,
    /// Bend em cents (100 = meio tom, 200 = um tom).
    Bend {
        cents: u16,
    },
    Vibrato,
    PalmMute,
    Harmonic {
        harmonic: HarmonicKind,
    },
    LetRing,
}

impl Technique {
    /// Mesmo tipo (ignora parâmetros), para adicionar/remover.
    pub fn same_kind(&self, other: &Technique) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// Posição na tablatura: corda (1 = mais aguda) e traste relativo ao capotraste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabPosition {
    pub string: u8,
    pub fret: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub pitch: Pitch,
    /// Ligada (tie) à próxima nota de mesma altura na mesma voz.
    #[serde(default)]
    pub tie: bool,
    #[serde(default)]
    pub tab: Option<TabPosition>,
    #[serde(default)]
    pub techniques: Vec<Technique>,
}

impl Note {
    pub fn new(pitch: Pitch) -> Self {
        Self {
            pitch,
            tie: false,
            tab: None,
            techniques: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "notes")]
pub enum EventKind {
    Rest,
    /// Uma nota é um acorde de um elemento.
    Chord(Vec<Note>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraceKind {
    /// Com barra (rápida, antes do tempo).
    Acciaccatura,
    Appoggiatura,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Syllabic {
    Single,
    Begin,
    Middle,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyric {
    pub text: String,
    pub syllabic: Syllabic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: EventId,
    pub duration: Duration,
    pub kind: EventKind,
    #[serde(default)]
    pub articulations: Vec<Articulation>,
    #[serde(default)]
    pub dynamic: Option<Dynamic>,
    /// Apojatura (não ocupa tempo no compasso).
    #[serde(default)]
    pub grace: Option<GraceKind>,
    #[serde(default)]
    pub lyric: Option<Lyric>,
}

impl Event {
    pub fn rest(duration: Duration) -> Self {
        Self {
            id: EventId(0),
            duration,
            kind: EventKind::Rest,
            articulations: Vec::new(),
            dynamic: None,
            grace: None,
            lyric: None,
        }
    }

    pub fn chord(duration: Duration, notes: Vec<Note>) -> Self {
        Self {
            kind: EventKind::Chord(notes),
            ..Self::rest(duration)
        }
    }

    pub fn note(duration: Duration, pitch: Pitch) -> Self {
        Self::chord(duration, vec![Note::new(pitch)])
    }

    pub fn is_rest(&self) -> bool {
        matches!(self.kind, EventKind::Rest)
    }

    pub fn notes(&self) -> &[Note] {
        match &self.kind {
            EventKind::Rest => &[],
            EventKind::Chord(n) => n,
        }
    }

    pub fn notes_mut(&mut self) -> Option<&mut Vec<Note>> {
        match &mut self.kind {
            EventKind::Rest => None,
            EventKind::Chord(n) => Some(n),
        }
    }

    /// Tempo ocupado no compasso (zero para apojaturas).
    pub fn time_value(&self) -> Frac {
        if self.grace.is_some() {
            Frac::ZERO
        } else {
            self.duration.fraction()
        }
    }

    pub fn validate(&self) -> Result<(), NotationError> {
        self.duration.validate()?;
        if let EventKind::Chord(notes) = &self.kind {
            if notes.is_empty() {
                return Err(NotationError::Inconsistent("acorde sem notas"));
            }
            if notes.len() > MAX_NOTES_PER_CHORD {
                return Err(NotationError::LimitExceeded("notas por acorde"));
            }
            for n in notes {
                if !(-2..=2).contains(&n.pitch.alter) {
                    return Err(NotationError::InvalidPitch("acidente fora de −2…+2"));
                }
                let m = n.pitch.midi();
                if !(0..=127).contains(&m) {
                    return Err(NotationError::PitchOutOfRange(m));
                }
            }
        }
        if self.is_rest() && self.grace.is_some() {
            return Err(NotationError::Inconsistent("pausa não pode ser apojatura"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    /// Índice da pauta (0 = superior) na parte.
    #[serde(default)]
    pub staff: u8,
    pub events: Vec<Event>,
}

impl Voice {
    pub fn new(staff: u8) -> Self {
        Self {
            staff,
            events: Vec::new(),
        }
    }

    /// Soma das durações (sem apojaturas).
    pub fn filled(&self) -> Frac {
        self.events
            .iter()
            .fold(Frac::ZERO, |acc, e| acc + e.time_value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub key: KeySignature,
    /// Mudança de fórmula neste compasso (vale até a próxima). `None` herda.
    #[serde(default)]
    pub meter: Option<Meter>,
    #[serde(default)]
    pub repeat_start: bool,
    /// Barra de repetição final: número total de execuções (≥ 2).
    #[serde(default)]
    pub repeat_end: Option<u8>,
    pub voices: Vec<Voice>,
}

impl Measure {
    pub fn new(key: KeySignature) -> Self {
        Self {
            key,
            meter: None,
            repeat_start: false,
            repeat_end: None,
            voices: vec![Voice::new(0)],
        }
    }

    /// Mesmos atributos (armadura, fórmula, repetições) sem o conteúdo.
    pub fn same_attributes(&self, o: &Measure) -> bool {
        self.key == o.key
            && self.meter == o.meter
            && self.repeat_start == o.repeat_start
            && self.repeat_end == o.repeat_end
    }

    /// Cópia dos atributos com uma voz por pauta preenchida por pausas.
    pub fn empty_like(&self, capacity: Frac, staves: usize) -> Measure {
        let mut m = Measure {
            voices: Vec::new(),
            ..self.clone()
        };
        for s in 0..staves.clamp(1, MAX_STAVES) {
            let mut v = Voice::new(s as u8);
            v.events = Duration::decompose(capacity)
                .0
                .into_iter()
                .map(Event::rest)
                .collect();
            m.voices.push(v);
        }
        m
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpannerKind {
    Slur,
    Crescendo,
    Diminuendo,
}

/// Elemento que abrange vários eventos (ligadura de expressão, crescendo, diminuendo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spanner {
    pub kind: SpannerKind,
    pub start: EventId,
    pub end: EventId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub name: String,
    /// Programa General MIDI (0–127).
    pub program: u8,
    /// Parte de percussão (canal MIDI 10).
    #[serde(default)]
    pub percussion: bool,
    pub staves: Vec<Staff>,
    pub measures: Vec<Measure>,
    #[serde(default)]
    pub spanners: Vec<Spanner>,
}

impl Part {
    pub fn validate_header(&self) -> Result<(), NotationError> {
        let n = self.name.trim();
        if n.is_empty() || n.chars().count() > 64 || n.chars().any(char::is_control) {
            return Err(NotationError::InvalidName);
        }
        if self.program > 127 {
            return Err(NotationError::InvalidProgram(self.program));
        }
        if self.staves.is_empty() || self.staves.len() > MAX_STAVES {
            return Err(NotationError::Inconsistent(
                "parte precisa de 1 ou 2 pautas",
            ));
        }
        for s in &self.staves {
            if let Some(t) = &s.tab {
                t.validate()?;
            }
        }
        Ok(())
    }

    pub fn tab_for_staff(&self, staff: u8) -> Option<&TabConfig> {
        self.staves.get(staff as usize).and_then(|s| s.tab.as_ref())
    }

    pub fn events(&self) -> impl Iterator<Item = &Event> {
        self.measures
            .iter()
            .flat_map(|m| m.voices.iter().flat_map(|v| v.events.iter()))
    }
}

/// Localização de uma voz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceLoc {
    pub part: usize,
    pub measure: usize,
    pub voice: usize,
}

impl VoiceLoc {
    pub const fn new(part: usize, measure: usize, voice: usize) -> Self {
        Self {
            part,
            measure,
            voice,
        }
    }
}

/// Localização de um evento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventLoc {
    pub voice: VoiceLoc,
    pub index: usize,
}

/// Agregado raiz: partitura.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub title: String,
    #[serde(default)]
    pub composer: String,
    /// Andamentos (BPM). A fórmula inicial do mapa é o padrão quando o 1º compasso não define uma;
    /// as fórmulas efetivas vêm dos compassos (ver [`Score::tempo_map`]).
    pub tempo: TempoMap,
    pub parts: Vec<Part>,
    /// Próximo id de evento livre.
    pub next_event_id: u64,
}

impl Score {
    /// Partitura vazia (sem partes): a primeira parte adicionada define a estrutura de compassos.
    pub fn new(title: impl Into<String>, bpm: f64, meter: Meter) -> Result<Self, NotationError> {
        let tempo = TempoMap::new(bpm, meter.numerator, meter.denominator)
            .map_err(|_| NotationError::InvalidTempo)?;
        Ok(Self {
            title: title.into(),
            composer: String::new(),
            tempo,
            parts: Vec::new(),
            next_event_id: 1,
        })
    }

    pub fn measure_count(&self) -> usize {
        self.parts.first().map_or(0, |p| p.measures.len())
    }

    /// Compassos de referência (atributos) — da primeira parte.
    pub fn reference_measures(&self) -> &[Measure] {
        self.parts.first().map_or(&[], |p| &p.measures)
    }

    pub fn default_meter(&self) -> Meter {
        let m = self.tempo.meters()[0];
        Meter {
            numerator: m.numerator,
            denominator: m.denominator,
        }
    }

    /// Fórmula vigente em cada compasso.
    pub fn meters(&self) -> Vec<Meter> {
        let mut cur = self.default_meter();
        self.reference_measures()
            .iter()
            .map(|m| {
                if let Some(x) = m.meter {
                    cur = x;
                }
                cur
            })
            .collect()
    }

    pub fn meter_at(&self, measure: usize) -> Meter {
        let mut cur = self.default_meter();
        for m in self.reference_measures().iter().take(measure + 1) {
            if let Some(x) = m.meter {
                cur = x;
            }
        }
        cur
    }

    /// Tick inicial de cada compasso (sem expandir repetições) + tick final.
    pub fn measure_starts(&self) -> Vec<Tick> {
        let mut out = Vec::with_capacity(self.measure_count() + 1);
        let mut t = 0i64;
        out.push(Tick(0));
        for m in self.meters() {
            t += m.ticks();
            out.push(Tick(t));
        }
        out
    }

    /// Mapa de tempo com os andamentos de `tempo` e as fórmulas derivadas dos compassos.
    pub fn tempo_map(&self) -> TempoMap {
        let starts = self.measure_starts();
        let mut meters = Vec::new();
        let mut last: Option<Meter> = None;
        for (i, m) in self.meters().into_iter().enumerate() {
            if last != Some(m) {
                meters.push(shared_kernel::tempo::MeterEvent {
                    tick: starts[i],
                    numerator: m.numerator,
                    denominator: m.denominator,
                });
                last = Some(m);
            }
        }
        if meters.is_empty() {
            meters.push(self.tempo.meters()[0]);
        }
        TempoMap::from_events(self.tempo.tempos().to_vec(), meters)
            .unwrap_or_else(|_| self.tempo.clone())
    }

    /// Reserva um id de evento novo.
    pub fn alloc_event_id(&mut self) -> EventId {
        let id = EventId(self.next_event_id);
        self.next_event_id += 1;
        id
    }

    /// Atribui ids novos a todos os eventos da parte (antes de inseri-la), remapeando spanners.
    pub fn assign_ids(&mut self, part: &mut Part) {
        let mut map = BTreeMap::new();
        for m in &mut part.measures {
            for v in &mut m.voices {
                for e in &mut v.events {
                    let new = EventId(self.next_event_id);
                    self.next_event_id += 1;
                    map.insert(e.id, new);
                    e.id = new;
                }
            }
        }
        part.spanners
            .retain_mut(|s| match (map.get(&s.start), map.get(&s.end)) {
                (Some(a), Some(b)) => {
                    s.start = *a;
                    s.end = *b;
                    true
                }
                _ => false,
            });
    }

    /// Renumera todos os eventos em ordem canônica (parte, compasso, voz, evento) a partir de 1.
    pub fn renumber_events(&mut self) {
        self.next_event_id = 1;
        let mut parts = std::mem::take(&mut self.parts);
        for p in &mut parts {
            self.assign_ids(p);
        }
        self.parts = parts;
    }

    /// Cria uma parte vazia (pausas) compatível com a estrutura de compassos atual.
    pub fn blank_part(&self, name: impl Into<String>, program: u8, staves: Vec<Staff>) -> Part {
        let meters = self.meters();
        let n_staves = staves.len();
        let measures = self
            .reference_measures()
            .iter()
            .zip(meters)
            .map(|(m, meter)| m.empty_like(meter.capacity(), n_staves))
            .collect();
        Part {
            name: name.into(),
            program,
            percussion: false,
            staves,
            measures,
            spanners: Vec::new(),
        }
    }

    /// Adiciona uma parte (validando e atribuindo ids). Se a partitura não tem partes, a parte define
    /// a estrutura de compassos.
    pub fn push_part(&mut self, mut part: Part) -> Result<usize, NotationError> {
        self.check_part_fits(&part)?;
        self.assign_ids(&mut part);
        self.parts.push(part);
        Ok(self.parts.len() - 1)
    }

    pub(crate) fn check_part_fits(&self, part: &Part) -> Result<(), NotationError> {
        part.validate_header()?;
        if self.parts.len() >= MAX_PARTS {
            return Err(NotationError::LimitExceeded("partes"));
        }
        if part.measures.is_empty() || part.measures.len() > MAX_MEASURES {
            return Err(NotationError::MeasureMismatch);
        }
        if let Some(first) = self.parts.first() {
            if first.measures.len() != part.measures.len()
                || first
                    .measures
                    .iter()
                    .zip(&part.measures)
                    .any(|(a, b)| !a.same_attributes(b))
            {
                return Err(NotationError::MeasureMismatch);
            }
        }
        for m in &part.measures {
            check_measure_content(part, m)?;
        }
        Ok(())
    }

    pub fn part(&self, i: usize) -> Result<&Part, NotationError> {
        self.parts.get(i).ok_or(NotationError::PartNotFound(i))
    }

    pub fn part_mut(&mut self, i: usize) -> Result<&mut Part, NotationError> {
        self.parts.get_mut(i).ok_or(NotationError::PartNotFound(i))
    }

    pub fn measure_mut(
        &mut self,
        part: usize,
        measure: usize,
    ) -> Result<&mut Measure, NotationError> {
        self.part_mut(part)?
            .measures
            .get_mut(measure)
            .ok_or(NotationError::MeasureNotFound(measure))
    }

    pub fn voice(&self, loc: VoiceLoc) -> Result<&Voice, NotationError> {
        self.part(loc.part)?
            .measures
            .get(loc.measure)
            .ok_or(NotationError::MeasureNotFound(loc.measure))?
            .voices
            .get(loc.voice)
            .ok_or(NotationError::VoiceNotFound(loc.voice))
    }

    pub fn voice_mut(&mut self, loc: VoiceLoc) -> Result<&mut Voice, NotationError> {
        self.measure_mut(loc.part, loc.measure)?
            .voices
            .get_mut(loc.voice)
            .ok_or(NotationError::VoiceNotFound(loc.voice))
    }

    pub fn event(&self, loc: EventLoc) -> Result<&Event, NotationError> {
        self.voice(loc.voice)?
            .events
            .get(loc.index)
            .ok_or(NotationError::EventNotFound(loc.index))
    }

    pub fn event_mut(&mut self, loc: EventLoc) -> Result<&mut Event, NotationError> {
        self.voice_mut(loc.voice)?
            .events
            .get_mut(loc.index)
            .ok_or(NotationError::EventNotFound(loc.index))
    }

    /// Localiza um evento pelo id.
    pub fn find_event(&self, id: EventId) -> Option<EventLoc> {
        for (pi, p) in self.parts.iter().enumerate() {
            for (mi, m) in p.measures.iter().enumerate() {
                for (vi, v) in m.voices.iter().enumerate() {
                    if let Some(ei) = v.events.iter().position(|e| e.id == id) {
                        return Some(EventLoc {
                            voice: VoiceLoc::new(pi, mi, vi),
                            index: ei,
                        });
                    }
                }
            }
        }
        None
    }

    /// Verifica todas as invariantes do agregado.
    pub fn check_invariants(&self) -> Result<(), NotationError> {
        let mut ids = HashSet::new();
        let reference = self.reference_measures();
        for p in &self.parts {
            p.validate_header()?;
            if p.measures.len() != reference.len() || p.measures.is_empty() {
                return Err(NotationError::MeasureMismatch);
            }
            for (a, b) in p.measures.iter().zip(reference) {
                if !a.same_attributes(b) {
                    return Err(NotationError::MeasureMismatch);
                }
                check_measure_content(p, a)?;
            }
            let mut part_ids = HashSet::new();
            for e in p.events() {
                if e.id.0 == 0 || e.id.0 >= self.next_event_id || !ids.insert(e.id) {
                    return Err(NotationError::Inconsistent("ids de evento"));
                }
                part_ids.insert(e.id);
            }
            for s in &p.spanners {
                if !part_ids.contains(&s.start) || !part_ids.contains(&s.end) {
                    return Err(NotationError::Inconsistent("spanner sem evento"));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn check_measure_content(part: &Part, m: &Measure) -> Result<(), NotationError> {
    if let Some(mt) = m.meter {
        Meter::new(mt.numerator, mt.denominator)?;
    }
    KeySignature::new(m.key.fifths, m.key.mode)?;
    if m.voices.len() > MAX_VOICES {
        return Err(NotationError::TooManyVoices);
    }
    if m.repeat_end.is_some_and(|t| !(2..=16).contains(&t)) {
        return Err(NotationError::Inconsistent(
            "repetição deve tocar 2–16 vezes",
        ));
    }
    for v in &m.voices {
        if v.staff as usize >= part.staves.len() {
            return Err(NotationError::Inconsistent("voz em pauta inexistente"));
        }
        if v.events.len() > MAX_EVENTS_PER_VOICE {
            return Err(NotationError::LimitExceeded("eventos por voz"));
        }
        let tab = part.tab_for_staff(v.staff);
        for e in &v.events {
            e.validate()?;
            for n in e.notes() {
                if let Some(pos) = n.tab {
                    let t = tab.ok_or(NotationError::InvalidTab("pauta sem tablatura"))?;
                    crate::tab::check_position(t, pos)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duration::NoteValue;
    use crate::pitch::Step;

    pub(crate) fn c4() -> Pitch {
        Pitch::new(Step::C, 0, 4).unwrap()
    }

    fn sample() -> Score {
        let mut s = Score::new("Teste", 120.0, Meter::COMMON).unwrap();
        let mut part = Part {
            name: "Piano".into(),
            program: 0,
            percussion: false,
            staves: vec![Staff::standard(Clef::Treble)],
            measures: vec![Measure::new(KeySignature::c_major()); 2],
            spanners: vec![],
        };
        part.measures[0].voices[0].events =
            vec![Event::note(Duration::new(NoteValue::Whole), c4())];
        part.measures[1].meter = Some(Meter::new(3, 4).unwrap());
        part.measures[1].voices[0].events =
            vec![Event::rest(Duration::new(NoteValue::Half).dotted(1))];
        s.push_part(part).unwrap();
        s
    }

    #[test]
    fn layout_and_tempo_map_follow_measures() {
        let s = sample();
        assert_eq!(
            s.measure_starts(),
            vec![Tick(0), Tick(3840), Tick(3840 + 2880)]
        );
        let tm = s.tempo_map();
        assert_eq!(tm.meters().len(), 2);
        assert_eq!(tm.meters()[1].numerator, 3);
        assert_eq!(tm.meters()[1].tick, Tick(3840));
        s.check_invariants().unwrap();
    }

    #[test]
    fn push_part_assigns_ids_and_rejects_mismatch() {
        let mut s = sample();
        assert_eq!(s.next_event_id, 3);
        let bad = Part {
            measures: vec![Measure::new(KeySignature::c_major())],
            ..s.parts[0].clone()
        };
        assert_eq!(s.push_part(bad), Err(NotationError::MeasureMismatch));
        let blank = s.blank_part("Baixo", 33, vec![Staff::standard(Clef::Bass)]);
        let idx = s.push_part(blank).unwrap();
        assert_eq!(idx, 1);
        s.check_invariants().unwrap();
        assert!(s.find_event(EventId(3)).is_some());
    }

    #[test]
    fn invariants_catch_bad_tab_and_voices() {
        let mut s = sample();
        s.parts[0].measures[0].voices[0].events[0]
            .notes_mut()
            .unwrap()[0]
            .tab = Some(TabPosition { string: 1, fret: 0 });
        assert!(s.check_invariants().is_err(), "pauta sem tablatura");
        let mut s = sample();
        for _ in 0..4 {
            s.parts[0].measures[0].voices.push(Voice::new(0));
        }
        assert_eq!(s.check_invariants(), Err(NotationError::TooManyVoices));
    }

    #[test]
    fn dynamics_velocity_mapping() {
        assert_eq!(Dynamic::from_velocity(80), Dynamic::Mf);
        assert_eq!(Dynamic::from_velocity(127), Dynamic::Fff);
        assert_eq!(Dynamic::from_symbol("pp"), Some(Dynamic::Pp));
        for d in Dynamic::ALL {
            assert_eq!(Dynamic::from_velocity(d.velocity()), d);
        }
    }

    #[test]
    fn serde_roundtrip() {
        let s = sample();
        let json = serde_json::to_string(&s).unwrap();
        let back: Score = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
