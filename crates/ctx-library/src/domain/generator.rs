//! Gerador de bases (RF-4.2): acordes já realizados + padrão rítmico/estilo + bateria + baixo →
//! eventos MIDI por canal (General MIDI), em ticks (`shared_kernel::PPQ`).
//!
//! **Decisão (ADR-0019):** este contexto não depende de Harmony. A entrada é uma lista de
//! [`ChordSlot`] (acordes já transformados em notas pela camada de aplicação, usando
//! `ctx_harmony::Progression::realize`). Assim cada contexto continua dependendo só do
//! `shared-kernel` e a composição Harmony + Library acontece no `app-core`.
//!
//! Os padrões são definidos numa grade de semicolcheias (4 passos por tempo). Programas GM são
//! numerados a partir de 0 (0 = Acoustic Grand Piano); a bateria usa o canal 10 (índice 9).

use super::base::Meter;
use super::error::LibraryError;
use serde::{Deserialize, Serialize};

/// Canal MIDI (base 0) da bateria General MIDI.
pub const DRUM_CHANNEL: u8 = 9;
const HARMONY_CHANNEL: u8 = 0;
const BASS_CHANNEL: u8 = 1;
const STEPS_PER_BEAT: i64 = 4;
/// Faixa do baixo: fundamental entre Dó 2 (36) e Si 2 (47).
const BASS_FLOOR: u8 = 36;

// Notas de percussão General MIDI.
const KICK: u8 = 36;
const SIDE_STICK: u8 = 37;
const SNARE: u8 = 38;
const LOW_FLOOR_TOM: u8 = 41;
const CLOSED_HH: u8 = 42;
const OPEN_HH: u8 = 46;
const TAMBOURINE: u8 = 54;
const SHAKER: u8 = 70; // Maracas
const RIDE: u8 = 51;

/// Acorde já realizado (entrada do gerador).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordSlot {
    pub start_tick: i64,
    pub duration_ticks: i64,
    /// Notas MIDI das vozes do acorde (qualquer ordem).
    pub notes: Vec<u8>,
    /// Fundamental (ou baixo da cifra) como nota MIDI em qualquer oitava.
    pub root: u8,
}

#[derive(Debug, Clone, Copy)]
enum Voicing {
    Full,
    Upper,
    Lower,
    Arp(u8),
}

#[derive(Debug, Clone, Copy)]
struct Hit {
    step: u8,
    len: u8,
    voicing: Voicing,
    vel: u8,
}

const fn h(step: u8, len: u8, voicing: Voicing, vel: u8) -> Hit {
    Hit {
        step,
        len,
        voicing,
        vel,
    }
}

use Voicing::{Arp, Full, Lower, Upper};

/// Padrão de acompanhamento harmônico (estilo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StylePattern {
    PopStrum,
    BalladArpeggio,
    BossaComping,
    SambaComping,
    RockEighths,
    ReggaeSkank,
    FunkStabs,
    Waltz,
    Guarania,
}

impl StylePattern {
    pub const ALL: [StylePattern; 9] = [
        Self::PopStrum,
        Self::BalladArpeggio,
        Self::BossaComping,
        Self::SambaComping,
        Self::RockEighths,
        Self::ReggaeSkank,
        Self::FunkStabs,
        Self::Waltz,
        Self::Guarania,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::PopStrum => "pop_strum",
            Self::BalladArpeggio => "ballad_arpeggio",
            Self::BossaComping => "bossa_comping",
            Self::SambaComping => "samba_comping",
            Self::RockEighths => "rock_eighths",
            Self::ReggaeSkank => "reggae_skank",
            Self::FunkStabs => "funk_stabs",
            Self::Waltz => "waltz",
            Self::Guarania => "guarania",
        }
    }

    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::PopStrum => "Pop – batida de violão",
            Self::BalladArpeggio => "Balada – arpejo",
            Self::BossaComping => "Bossa nova – levada",
            Self::SambaComping => "Samba – partido alto",
            Self::RockEighths => "Rock – colcheias",
            Self::ReggaeSkank => "Reggae – skank",
            Self::FunkStabs => "Funk – ataques",
            Self::Waltz => "Valsa (3/4)",
            Self::Guarania => "Guarânia / sertanejo (3/4)",
        }
    }

    /// Tempos por compasso exigidos (denominador semínima).
    pub const fn meter_numerator(self) -> u8 {
        match self {
            Self::Waltz | Self::Guarania => 3,
            _ => 4,
        }
    }

    /// Programa General MIDI (base 0) do instrumento harmônico.
    pub const fn program(self) -> u8 {
        match self {
            Self::PopStrum => 25,      // Acoustic Guitar (steel)
            Self::BalladArpeggio => 0, // Acoustic Grand Piano
            Self::BossaComping => 24,  // Acoustic Guitar (nylon)
            Self::SambaComping => 24,  // Acoustic Guitar (nylon)
            Self::RockEighths => 29,   // Overdriven Guitar
            Self::ReggaeSkank => 16,   // Drawbar Organ
            Self::FunkStabs => 28,     // Electric Guitar (muted)
            Self::Waltz => 21,         // Accordion
            Self::Guarania => 24,      // Acoustic Guitar (nylon)
        }
    }

    pub const fn default_drums(self) -> DrumPattern {
        match self {
            Self::PopStrum | Self::RockEighths => DrumPattern::PopRock,
            Self::BalladArpeggio => DrumPattern::Ballad,
            Self::BossaComping => DrumPattern::Bossa,
            Self::SambaComping => DrumPattern::Samba,
            Self::ReggaeSkank => DrumPattern::Reggae,
            Self::FunkStabs => DrumPattern::Funk,
            Self::Waltz => DrumPattern::Waltz,
            Self::Guarania => DrumPattern::Guarania,
        }
    }

    pub const fn default_bass(self) -> BassPattern {
        match self {
            Self::PopStrum => BassPattern::RootFifth,
            Self::BalladArpeggio => BassPattern::RootWhole,
            Self::BossaComping | Self::SambaComping => BassPattern::BossaRootFifth,
            Self::RockEighths => BassPattern::RootEighths,
            Self::ReggaeSkank => BassPattern::ReggaeOneDrop,
            Self::FunkStabs => BassPattern::FunkSyncopated,
            Self::Waltz | Self::Guarania => BassPattern::WaltzRootFifth,
        }
    }

    pub fn check_meter(self, meter: Meter) -> Result<(), LibraryError> {
        check_fixed_meter(self.code(), Some(self.meter_numerator()), meter)
    }

    fn hits(self) -> &'static [Hit] {
        const POP: &[Hit] = &[
            h(0, 4, Full, 100),
            h(4, 2, Full, 85),
            h(6, 2, Upper, 70),
            h(10, 2, Upper, 70),
            h(12, 2, Full, 85),
            h(14, 2, Upper, 70),
        ];
        const BALLAD: &[Hit] = &[
            h(0, 2, Arp(0), 85),
            h(2, 2, Arp(1), 70),
            h(4, 2, Arp(2), 70),
            h(6, 2, Arp(1), 65),
            h(8, 2, Arp(3), 75),
            h(10, 2, Arp(2), 65),
            h(12, 2, Arp(1), 70),
            h(14, 2, Arp(2), 65),
        ];
        const BOSSA: &[Hit] = &[
            h(0, 2, Upper, 80),
            h(3, 2, Upper, 70),
            h(6, 2, Upper, 75),
            h(10, 2, Upper, 70),
            h(13, 2, Upper, 75),
        ];
        const SAMBA: &[Hit] = &[
            h(0, 1, Upper, 90),
            h(3, 1, Upper, 70),
            h(5, 1, Upper, 80),
            h(7, 1, Upper, 70),
            h(8, 1, Upper, 90),
            h(11, 1, Upper, 70),
            h(13, 1, Upper, 80),
            h(14, 1, Upper, 70),
        ];
        const ROCK: &[Hit] = &[
            h(0, 2, Lower, 105),
            h(2, 2, Lower, 90),
            h(4, 2, Lower, 100),
            h(6, 2, Lower, 90),
            h(8, 2, Lower, 105),
            h(10, 2, Lower, 90),
            h(12, 2, Lower, 100),
            h(14, 2, Lower, 90),
        ];
        const REGGAE: &[Hit] = &[h(4, 2, Upper, 90), h(12, 2, Upper, 90)];
        const FUNK: &[Hit] = &[
            h(0, 1, Upper, 100),
            h(3, 1, Upper, 80),
            h(6, 1, Upper, 90),
            h(8, 1, Upper, 70),
            h(10, 1, Upper, 95),
            h(11, 1, Upper, 75),
            h(14, 1, Upper, 90),
        ];
        const WALTZ: &[Hit] = &[h(4, 3, Upper, 80), h(8, 3, Upper, 75)];
        const GUARANIA: &[Hit] = &[h(0, 4, Full, 85), h(6, 2, Upper, 70), h(8, 4, Upper, 75)];
        match self {
            Self::PopStrum => POP,
            Self::BalladArpeggio => BALLAD,
            Self::BossaComping => BOSSA,
            Self::SambaComping => SAMBA,
            Self::RockEighths => ROCK,
            Self::ReggaeSkank => REGGAE,
            Self::FunkStabs => FUNK,
            Self::Waltz => WALTZ,
            Self::Guarania => GUARANIA,
        }
    }
}

fn check_fixed_meter(
    pattern: &'static str,
    numerator: Option<u8>,
    meter: Meter,
) -> Result<(), LibraryError> {
    match numerator {
        Some(n) if n != meter.numerator() || meter.denominator() != 4 => {
            Err(LibraryError::PatternMeterMismatch {
                pattern,
                expected: n,
                actual: meter.numerator(),
            })
        }
        _ => Ok(()),
    }
}

/// Padrão de bateria (General MIDI, canal 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrumPattern {
    None,
    PopRock,
    Ballad,
    Bossa,
    Samba,
    Reggae,
    Funk,
    Waltz,
    Guarania,
}

impl DrumPattern {
    pub const ALL: [DrumPattern; 9] = [
        Self::None,
        Self::PopRock,
        Self::Ballad,
        Self::Bossa,
        Self::Samba,
        Self::Reggae,
        Self::Funk,
        Self::Waltz,
        Self::Guarania,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PopRock => "pop_rock",
            Self::Ballad => "ballad",
            Self::Bossa => "bossa",
            Self::Samba => "samba",
            Self::Reggae => "reggae_one_drop",
            Self::Funk => "funk",
            Self::Waltz => "waltz",
            Self::Guarania => "guarania",
        }
    }

    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::None => "Sem bateria",
            Self::PopRock => "Pop/rock",
            Self::Ballad => "Balada",
            Self::Bossa => "Bossa nova",
            Self::Samba => "Samba",
            Self::Reggae => "Reggae (one drop)",
            Self::Funk => "Funk",
            Self::Waltz => "Valsa",
            Self::Guarania => "Guarânia",
        }
    }

    pub const fn meter_numerator(self) -> Option<u8> {
        match self {
            Self::None => None,
            Self::Waltz | Self::Guarania => Some(3),
            _ => Some(4),
        }
    }

    pub fn check_meter(self, meter: Meter) -> Result<(), LibraryError> {
        check_fixed_meter(self.code(), self.meter_numerator(), meter)
    }

    /// (passo, nota GM, velocidade).
    fn hits(self) -> Vec<(u8, u8, u8)> {
        let mut v: Vec<(u8, u8, u8)> = Vec::new();
        let eighths = |v: &mut Vec<(u8, u8, u8)>, note: u8, vel: u8, steps: u8| {
            for s in (0..steps).step_by(2) {
                v.push((s, note, vel));
            }
        };
        match self {
            Self::None => {}
            Self::PopRock => {
                v.extend([(0, KICK, 110), (8, KICK, 105), (10, KICK, 80)]);
                v.extend([(4, SNARE, 110), (12, SNARE, 110)]);
                eighths(&mut v, CLOSED_HH, 75, 16);
            }
            Self::Ballad => {
                v.extend([(0, KICK, 95), (10, KICK, 70)]);
                v.extend([(4, SIDE_STICK, 80), (12, SIDE_STICK, 80)]);
                v.extend([
                    (0, CLOSED_HH, 60),
                    (4, CLOSED_HH, 55),
                    (8, CLOSED_HH, 60),
                    (12, CLOSED_HH, 55),
                ]);
            }
            Self::Bossa => {
                v.extend([(0, KICK, 90), (6, KICK, 70), (8, KICK, 90), (14, KICK, 70)]);
                v.extend([
                    (0, SIDE_STICK, 80),
                    (3, SIDE_STICK, 75),
                    (6, SIDE_STICK, 75),
                    (10, SIDE_STICK, 80),
                    (13, SIDE_STICK, 75),
                ]);
                eighths(&mut v, CLOSED_HH, 55, 16);
            }
            Self::Samba => {
                v.extend([
                    (0, LOW_FLOOR_TOM, 70),
                    (4, LOW_FLOOR_TOM, 115),
                    (8, LOW_FLOOR_TOM, 70),
                    (12, LOW_FLOOR_TOM, 115),
                ]);
                for s in 0..16u8 {
                    v.push((s, TAMBOURINE, if s % 4 == 0 { 90 } else { 60 }));
                }
                v.extend([
                    (0, SIDE_STICK, 85),
                    (3, SIDE_STICK, 80),
                    (6, SIDE_STICK, 80),
                    (8, SIDE_STICK, 85),
                    (10, SIDE_STICK, 80),
                    (13, SIDE_STICK, 80),
                ]);
            }
            Self::Reggae => {
                v.extend([(8, KICK, 110), (8, SIDE_STICK, 100)]);
                eighths(&mut v, CLOSED_HH, 70, 16);
                v.push((14, OPEN_HH, 60));
            }
            Self::Funk => {
                v.extend([
                    (0, KICK, 115),
                    (3, KICK, 90),
                    (7, KICK, 85),
                    (10, KICK, 100),
                ]);
                v.extend([
                    (4, SNARE, 115),
                    (12, SNARE, 115),
                    (9, SNARE, 40),
                    (15, SNARE, 40),
                ]);
                for s in 0..16u8 {
                    v.push((s, CLOSED_HH, if s % 2 == 0 { 80 } else { 50 }));
                }
            }
            Self::Waltz => {
                v.push((0, KICK, 100));
                v.extend([(4, SIDE_STICK, 70), (8, SIDE_STICK, 70)]);
                v.extend([(0, RIDE, 60), (4, RIDE, 55), (8, RIDE, 55)]);
            }
            Self::Guarania => {
                v.extend([(0, KICK, 100), (6, KICK, 75)]);
                v.push((8, SIDE_STICK, 75));
                for s in (0..12u8).step_by(2) {
                    v.push((s, SHAKER, if s % 4 == 0 { 70 } else { 50 }));
                }
            }
        }
        v
    }
}

#[derive(Debug, Clone, Copy)]
enum BassNote {
    Root,
    Fifth,
    Octave,
    /// Fundamental nos compassos pares, quinta nos ímpares.
    RootOrFifthByBar,
}

/// Padrão de linha de baixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BassPattern {
    None,
    RootWhole,
    RootFifth,
    RootEighths,
    BossaRootFifth,
    ReggaeOneDrop,
    FunkSyncopated,
    WaltzRootFifth,
}

impl BassPattern {
    pub const ALL: [BassPattern; 8] = [
        Self::None,
        Self::RootWhole,
        Self::RootFifth,
        Self::RootEighths,
        Self::BossaRootFifth,
        Self::ReggaeOneDrop,
        Self::FunkSyncopated,
        Self::WaltzRootFifth,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RootWhole => "root_whole",
            Self::RootFifth => "root_fifth",
            Self::RootEighths => "root_eighths",
            Self::BossaRootFifth => "bossa_root_fifth",
            Self::ReggaeOneDrop => "reggae_one_drop",
            Self::FunkSyncopated => "funk_syncopated",
            Self::WaltzRootFifth => "waltz_root_fifth",
        }
    }

    pub const fn name_pt(self) -> &'static str {
        match self {
            Self::None => "Sem baixo",
            Self::RootWhole => "Fundamental (nota longa)",
            Self::RootFifth => "Fundamental e quinta",
            Self::RootEighths => "Fundamental em colcheias",
            Self::BossaRootFifth => "Bossa/samba (fundamental–quinta)",
            Self::ReggaeOneDrop => "Reggae",
            Self::FunkSyncopated => "Funk sincopado",
            Self::WaltzRootFifth => "Valsa (fundamental/quinta por compasso)",
        }
    }

    /// `None` = padrão genérico (adapta-se a qualquer compasso).
    pub const fn meter_numerator(self) -> Option<u8> {
        match self {
            Self::BossaRootFifth | Self::ReggaeOneDrop | Self::FunkSyncopated => Some(4),
            Self::WaltzRootFifth => Some(3),
            _ => None,
        }
    }

    pub fn check_meter(self, meter: Meter) -> Result<(), LibraryError> {
        check_fixed_meter(self.code(), self.meter_numerator(), meter)
    }

    /// Programa General MIDI (base 0): baixo acústico nos estilos brasileiros/valsa.
    pub const fn program(self) -> u8 {
        match self {
            Self::BossaRootFifth | Self::WaltzRootFifth => 32,
            _ => 33,
        }
    }

    /// (passo, duração em passos, nota).
    fn hits(self, steps_per_bar: u8) -> Vec<(u8, u8, BassNote)> {
        use BassNote::*;
        let half = steps_per_bar / 2;
        match self {
            Self::None => vec![],
            Self::RootWhole => vec![(0, steps_per_bar, Root)],
            Self::RootFifth => vec![(0, half, Root), (half, steps_per_bar - half, Fifth)],
            Self::RootEighths => (0..steps_per_bar)
                .step_by(2)
                .map(|s| (s, 2, Root))
                .collect(),
            Self::BossaRootFifth => vec![(0, 6, Root), (6, 2, Root), (8, 6, Fifth), (14, 2, Fifth)],
            Self::ReggaeOneDrop => vec![
                (0, 3, Root),
                (3, 1, Root),
                (6, 2, Fifth),
                (10, 2, Octave),
                (12, 4, Root),
            ],
            Self::FunkSyncopated => vec![
                (0, 2, Root),
                (3, 1, Octave),
                (6, 2, Root),
                (10, 1, Fifth),
                (11, 1, Octave),
                (14, 2, Root),
            ],
            Self::WaltzRootFifth => vec![(0, 4, RootOrFifthByBar)],
        }
    }
}

/// Pedido de geração.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
    /// Acordes realizados; repetidos ciclicamente se a progressão for mais curta que `bars`.
    pub chords: Vec<ChordSlot>,
    pub meter: Meter,
    pub bars: u16,
    pub style: StylePattern,
    pub drums: DrumPattern,
    pub bass: BassPattern,
}

/// Evento de nota MIDI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEvent {
    pub start_tick: i64,
    pub duration_ticks: i64,
    pub note: u8,
    pub velocity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackRole {
    Harmony,
    Bass,
    Drums,
}

/// Faixa gerada: um canal MIDI com programa GM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedTrack {
    pub role: TrackRole,
    pub channel: u8,
    pub program: u8,
    pub events: Vec<NoteEvent>,
}

/// Resultado da geração.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedBase {
    pub total_ticks: i64,
    pub tracks: Vec<GeneratedTrack>,
}

impl GeneratedBase {
    pub fn track(&self, role: TrackRole) -> Option<&GeneratedTrack> {
        self.tracks.iter().find(|t| t.role == role)
    }
}

fn validate_slots(chords: &[ChordSlot]) -> Result<i64, LibraryError> {
    if chords.is_empty() {
        return Err(LibraryError::NoChords);
    }
    let mut period = 0;
    for (i, c) in chords.iter().enumerate() {
        let bad = |why: &str| {
            Err(LibraryError::InvalidChordSlot(format!(
                "acorde {}: {why}",
                i + 1
            )))
        };
        if c.start_tick < 0 {
            return bad("início negativo");
        }
        if c.duration_ticks <= 0 {
            return bad("duração deve ser positiva");
        }
        if c.notes.is_empty() {
            return bad("sem notas");
        }
        if c.notes.iter().any(|n| *n > 127) || c.root > 127 {
            return bad("nota MIDI fora de 0–127");
        }
        period = period.max(c.start_tick + c.duration_ticks);
    }
    Ok(period)
}

/// Acorde ativo no tick absoluto `t` e o fim absoluto desse acorde.
fn chord_at(chords: &[ChordSlot], period: i64, t: i64) -> Option<(&ChordSlot, i64)> {
    let cycle_start = t - t.rem_euclid(period);
    let tp = t.rem_euclid(period);
    chords
        .iter()
        .find(|c| c.start_tick <= tp && tp < c.start_tick + c.duration_ticks)
        .map(|c| (c, cycle_start + c.start_tick + c.duration_ticks))
}

fn bass_root(root: u8) -> u8 {
    BASS_FLOOR + root % 12
}

fn voiced(notes: &[u8], v: Voicing) -> Vec<u8> {
    let mut sorted = notes.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    match v {
        Full => sorted,
        Upper if sorted.len() > 1 => sorted[1..].to_vec(),
        Upper => sorted,
        Lower => sorted.into_iter().take(2).collect(),
        Arp(i) => vec![sorted[i as usize % sorted.len()]],
    }
}

/// Gera a base MIDI.
pub fn generate(req: &GenerationRequest) -> Result<GeneratedBase, LibraryError> {
    if !(1..=256).contains(&req.bars) {
        return Err(LibraryError::InvalidBars);
    }
    req.style.check_meter(req.meter)?;
    req.drums.check_meter(req.meter)?;
    req.bass.check_meter(req.meter)?;
    let period = validate_slots(&req.chords)?;
    let bar_ticks = req.meter.bar_ticks();
    let step_ticks = req.meter.beat_ticks() / STEPS_PER_BEAT;
    let steps_per_bar = (req.meter.numerator() as i64 * STEPS_PER_BEAT) as u8;
    let total = bar_ticks * req.bars as i64;

    let mut harmony = Vec::new();
    let mut bass = Vec::new();
    let mut drums = Vec::new();
    let bass_hits = req.bass.hits(steps_per_bar);
    let drum_hits = req.drums.hits();

    for bar in 0..req.bars as i64 {
        let bar_start = bar * bar_ticks;
        let in_bar = |step: u8| (step as i64) < steps_per_bar as i64;
        for hit in req.style.hits().iter().filter(|h| in_bar(h.step)) {
            let t = bar_start + hit.step as i64 * step_ticks;
            let Some((slot, slot_end)) = chord_at(&req.chords, period, t) else {
                continue;
            };
            let dur = (hit.len as i64 * step_ticks)
                .min(slot_end - t)
                .min(total - t);
            if dur <= 0 {
                continue;
            }
            for note in voiced(&slot.notes, hit.voicing) {
                harmony.push(NoteEvent {
                    start_tick: t,
                    duration_ticks: dur,
                    note,
                    velocity: hit.vel.clamp(1, 127),
                });
            }
        }
        for (step, len, which) in bass_hits.iter().copied().filter(|(s, _, _)| in_bar(*s)) {
            let t = bar_start + step as i64 * step_ticks;
            let Some((slot, slot_end)) = chord_at(&req.chords, period, t) else {
                continue;
            };
            let dur = (len as i64 * step_ticks).min(slot_end - t).min(total - t);
            if dur <= 0 {
                continue;
            }
            let root = bass_root(slot.root);
            let note = match which {
                BassNote::Root => root,
                BassNote::Fifth => root + 7,
                BassNote::Octave => root + 12,
                BassNote::RootOrFifthByBar if bar % 2 == 0 => root,
                BassNote::RootOrFifthByBar => root + 7,
            };
            bass.push(NoteEvent {
                start_tick: t,
                duration_ticks: dur,
                note,
                velocity: 95,
            });
        }
        for (step, note, vel) in drum_hits.iter().copied().filter(|(s, _, _)| in_bar(*s)) {
            let t = bar_start + step as i64 * step_ticks;
            drums.push(NoteEvent {
                start_tick: t,
                duration_ticks: step_ticks.min(total - t),
                note,
                velocity: vel.clamp(1, 127),
            });
        }
    }

    let mut tracks = Vec::new();
    let mut push = |role, channel, program, mut events: Vec<NoteEvent>| {
        if !events.is_empty() {
            events.sort_by_key(|e| (e.start_tick, e.note));
            tracks.push(GeneratedTrack {
                role,
                channel,
                program,
                events,
            });
        }
    };
    push(
        TrackRole::Harmony,
        HARMONY_CHANNEL,
        req.style.program(),
        harmony,
    );
    push(TrackRole::Bass, BASS_CHANNEL, req.bass.program(), bass);
    push(TrackRole::Drums, DRUM_CHANNEL, 0, drums);
    Ok(GeneratedBase {
        total_ticks: total,
        tracks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use shared_kernel::PPQ;

    /// Acordes de um compasso cada: C, F, G7, C (notas fechadas a partir do Dó 4).
    fn slots(bar_ticks: i64) -> Vec<ChordSlot> {
        let chords: [(&[u8], u8); 4] = [
            (&[60, 64, 67], 48),
            (&[60, 65, 69], 53),
            (&[59, 62, 65, 67], 55),
            (&[60, 64, 67], 48),
        ];
        chords
            .iter()
            .enumerate()
            .map(|(i, (n, r))| ChordSlot {
                start_tick: i as i64 * bar_ticks,
                duration_ticks: bar_ticks,
                notes: n.to_vec(),
                root: *r,
            })
            .collect()
    }

    fn request(style: StylePattern, meter: Meter, bars: u16) -> GenerationRequest {
        GenerationRequest {
            chords: slots(meter.bar_ticks()),
            meter,
            bars,
            style,
            drums: style.default_drums(),
            bass: style.default_bass(),
        }
    }

    fn assert_inside(g: &GeneratedBase) {
        for t in &g.tracks {
            for e in &t.events {
                assert!(e.start_tick >= 0 && e.start_tick < g.total_ticks, "{e:?}");
                assert!(e.duration_ticks > 0, "{e:?}");
                assert!(e.start_tick + e.duration_ticks <= g.total_ticks, "{e:?}");
                assert!((1..=127).contains(&e.velocity));
                assert!(e.note <= 127);
            }
        }
    }

    #[test]
    fn waltz_respects_three_four() {
        let meter = Meter::THREE_FOUR;
        let g = generate(&request(StylePattern::Waltz, meter, 4)).unwrap();
        let bar = 3 * PPQ;
        assert_eq!(g.total_ticks, 4 * bar);
        assert_inside(&g);
        let drums = g.track(TrackRole::Drums).unwrap();
        assert_eq!(drums.channel, DRUM_CHANNEL);
        let kicks: Vec<i64> = drums
            .events
            .iter()
            .filter(|e| e.note == KICK)
            .map(|e| e.start_tick)
            .collect();
        assert_eq!(
            kicks,
            vec![0, bar, 2 * bar, 3 * bar],
            "bumbo no 1 de cada compasso"
        );
        let harmony = g.track(TrackRole::Harmony).unwrap();
        assert_eq!(harmony.program, 21);
        let offsets: std::collections::BTreeSet<i64> =
            harmony.events.iter().map(|e| e.start_tick % bar).collect();
        assert_eq!(offsets, [PPQ, 2 * PPQ].into(), "\"pá-pá\" nos tempos 2 e 3");
        let bass = g.track(TrackRole::Bass).unwrap();
        let notes: Vec<(i64, u8)> = bass.events.iter().map(|e| (e.start_tick, e.note)).collect();
        assert_eq!(
            notes,
            vec![(0, 36), (bar, 41 + 7), (2 * bar, 43), (3 * bar, 36 + 7)],
            "fundamental nos compassos pares, quinta nos ímpares"
        );
    }

    #[test]
    fn waltz_rejects_four_four() {
        let err = generate(&request(StylePattern::Waltz, Meter::FOUR_FOUR, 4)).unwrap_err();
        assert!(matches!(
            err,
            LibraryError::PatternMeterMismatch {
                expected: 3,
                actual: 4,
                ..
            }
        ));
        let mut req = request(StylePattern::PopStrum, Meter::FOUR_FOUR, 4);
        req.drums = DrumPattern::Waltz;
        assert!(generate(&req).is_err());
        let mut req = request(StylePattern::PopStrum, Meter::new(4, 8).unwrap(), 4);
        req.drums = DrumPattern::None;
        req.bass = BassPattern::None;
        assert!(
            generate(&req).is_err(),
            "padrões fixos exigem denominador 4"
        );
    }

    #[test]
    fn harmony_events_never_cross_chord_changes() {
        let g = generate(&request(StylePattern::PopStrum, Meter::FOUR_FOUR, 4)).unwrap();
        let bar = 4 * PPQ;
        for e in &g.track(TrackRole::Harmony).unwrap().events {
            assert_eq!(
                e.start_tick / bar,
                (e.start_tick + e.duration_ticks - 1) / bar
            );
        }
        assert_eq!(g.track(TrackRole::Harmony).unwrap().program, 25);
    }

    #[test]
    fn short_progression_cycles_over_bars() {
        let meter = Meter::FOUR_FOUR;
        let mut req = request(StylePattern::RockEighths, meter, 8);
        req.chords.truncate(2);
        let g = generate(&req).unwrap();
        let bar = meter.bar_ticks();
        let h = &g.track(TrackRole::Harmony).unwrap().events;
        let bar_notes = |b: i64| -> Vec<(i64, u8)> {
            h.iter()
                .filter(|e| e.start_tick / bar == b)
                .map(|e| (e.start_tick - b * bar, e.note))
                .collect()
        };
        assert_eq!(bar_notes(0), bar_notes(2));
        assert_eq!(bar_notes(1), bar_notes(7));
        assert_ne!(bar_notes(0), bar_notes(1));
    }

    #[test]
    fn invalid_requests() {
        let mut req = request(StylePattern::PopStrum, Meter::FOUR_FOUR, 0);
        assert_eq!(generate(&req), Err(LibraryError::InvalidBars));
        req.bars = 4;
        req.chords.clear();
        assert_eq!(generate(&req), Err(LibraryError::NoChords));
        req.chords = vec![ChordSlot {
            start_tick: 0,
            duration_ticks: 0,
            notes: vec![60],
            root: 48,
        }];
        assert!(matches!(
            generate(&req),
            Err(LibraryError::InvalidChordSlot(_))
        ));
        req.chords[0].duration_ticks = 10;
        req.chords[0].notes = vec![200];
        assert!(matches!(
            generate(&req),
            Err(LibraryError::InvalidChordSlot(_))
        ));
    }

    #[test]
    fn none_patterns_omit_tracks_and_drums_use_gm_range() {
        let mut req = request(StylePattern::BalladArpeggio, Meter::FOUR_FOUR, 2);
        req.bass = BassPattern::None;
        let g = generate(&req).unwrap();
        assert!(g.track(TrackRole::Bass).is_none());
        for e in &g.track(TrackRole::Drums).unwrap().events {
            assert!((35..=81).contains(&e.note));
        }
        req.drums = DrumPattern::None;
        assert!(generate(&req).unwrap().track(TrackRole::Drums).is_none());
    }

    #[test]
    fn codes_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for s in StylePattern::ALL {
            assert!(seen.insert(("s", s.code())));
            assert!(!s.name_pt().is_empty());
        }
        for d in DrumPattern::ALL {
            assert!(seen.insert(("d", d.code())));
        }
        for b in BassPattern::ALL {
            assert!(seen.insert(("b", b.code())));
        }
        assert!(StylePattern::ALL.len() >= 8);
    }

    proptest! {
        #[test]
        fn prop_every_style_generates_inside_range(si in 0usize..9, bars in 1u16..12) {
            let style = StylePattern::ALL[si];
            let meter = Meter::new(style.meter_numerator(), 4).unwrap();
            let g = generate(&request(style, meter, bars)).unwrap();
            prop_assert_eq!(g.total_ticks, bars as i64 * meter.bar_ticks());
            prop_assert!(g.track(TrackRole::Harmony).is_some());
            prop_assert!(g.track(TrackRole::Bass).is_some());
            prop_assert!(g.track(TrackRole::Drums).is_some());
            assert_inside(&g);
            for t in &g.tracks {
                prop_assert!(t.events.windows(2).all(|w| w[0].start_tick <= w[1].start_tick));
            }
        }
    }
}
