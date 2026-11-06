//! Casos de uso de Bases (RF-4.1 a RF-4.5): busca, importação de áudio do usuário, geração
//! (Harmony realiza → Library gera — ADR-0019), receitas, manifesto CC0 e bases iniciais.

use super::harmony::{check_bpm, ProgressionRef};
use super::library::{Composer, OnBases};
use super::{harmony_err, invalid, library_err};
use crate::ports::MidiNoteEvent;
use crate::studio::{AppError, AppResult};
use ctx_harmony::{
    Key, Mode, Progression, RomanChord, SuggestionProvider, TimedChord, VoicingOptions,
};
use ctx_library::commands::{AddBase, RemoveBase};
use ctx_library::{
    generate, AudioBase, AudioPath, Base, BaseId, BaseKey, BaseKind, BaseQuery, BassPattern,
    ChordSlot, DrumPattern, GeneratedBase, GenerationRecipe, GenerationRequest, GenreCode,
    Instrument, KeyMode, Meter, StylePattern, Tag,
};
use serde::{Deserialize, Serialize};
use shared_kernel::music::PitchClass;
use shared_kernel::Tick;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Extensões de áudio aceitas para bases do usuário.
pub const BASE_AUDIO_EXTENSIONS: [&str; 6] = ["wav", "flac", "mp3", "ogg", "aiff", "aif"];
/// Versão das bases iniciais (receitas geradas) semeadas na primeira execução.
pub const STARTER_BASES_VERSION: u32 = 1;

/// Pedido de geração de base (RF-4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateBaseRequest {
    pub progression: ProgressionRef,
    pub key: Key,
    pub bpm: f64,
    pub bars: u16,
    pub style: StylePattern,
    /// `None` = bateria padrão do estilo.
    #[serde(default)]
    pub drums: Option<DrumPattern>,
    /// `None` = baixo padrão do estilo.
    #[serde(default)]
    pub bass: Option<BassPattern>,
}

/// Base gerada pronta para ouvir/salvar.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedBaseView {
    pub bpm: f64,
    pub meter: Meter,
    pub total_ticks: i64,
    /// Progressão canônica da receita (graus no modo maior/menor da base).
    pub recipe_progression: String,
    pub drums: DrumPattern,
    pub bass: BassPattern,
    pub events: Vec<MidiNoteEvent>,
}

/// Salvar uma base gerada no catálogo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGeneratedBaseRequest {
    pub name: String,
    pub generation: GenerateBaseRequest,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Metadados de uma base de áudio do usuário.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewUserBase {
    pub name: String,
    pub bpm: f64,
    #[serde(default)]
    pub key: Option<BaseKey>,
    #[serde(default)]
    pub meter: Option<Meter>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub instruments: BTreeSet<Instrument>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub progression: Option<String>,
}

/// Opções de geração (para a UI montar os seletores).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternView<T> {
    pub value: T,
    pub code: &'static str,
    pub name_pt: &'static str,
    /// Tempos por compasso exigidos (`None` = qualquer).
    pub meter_numerator: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseOptionsView {
    pub styles: Vec<PatternView<StylePattern>>,
    pub drums: Vec<PatternView<DrumPattern>>,
    pub basses: Vec<PatternView<BassPattern>>,
    pub instruments: Vec<PatternView<Instrument>>,
}

pub fn base_options() -> BaseOptionsView {
    BaseOptionsView {
        styles: StylePattern::ALL
            .iter()
            .map(|s| PatternView {
                value: *s,
                code: s.code(),
                name_pt: s.name_pt(),
                meter_numerator: Some(s.meter_numerator()),
            })
            .collect(),
        drums: DrumPattern::ALL
            .iter()
            .map(|s| PatternView {
                value: *s,
                code: s.code(),
                name_pt: s.name_pt(),
                meter_numerator: s.meter_numerator(),
            })
            .collect(),
        basses: BassPattern::ALL
            .iter()
            .map(|s| PatternView {
                value: *s,
                code: s.code(),
                name_pt: s.name_pt(),
                meter_numerator: s.meter_numerator(),
            })
            .collect(),
        instruments: Instrument::ALL
            .iter()
            .map(|s| PatternView {
                value: *s,
                code: s.code(),
                name_pt: s.name_pt(),
                meter_numerator: None,
            })
            .collect(),
    }
}

// ------------------------------------------------------------- composição Harmony + Library

/// Modo maior/menor de um tom (bases só guardam maior/menor).
pub fn base_key_of(key: Key) -> BaseKey {
    BaseKey::new(
        key.tonic,
        if key.mode.is_minor() {
            KeyMode::Minor
        } else {
            KeyMode::Major
        },
    )
}

pub fn harmony_key_of(key: BaseKey) -> Key {
    match key.mode {
        KeyMode::Major => Key::new(key.tonic, Mode::Ionian),
        KeyMode::Minor => Key::new(key.tonic, Mode::Aeolian),
    }
}

/// Acordes realizados (sem baixo — o gerador faz o baixo) → base MIDI.
pub fn generate_from_chords(
    provider: &dyn SuggestionProvider,
    chords: &[TimedChord],
    meter: Meter,
    bars: u16,
    style: StylePattern,
    drums: DrumPattern,
    bass: BassPattern,
) -> AppResult<GeneratedBase> {
    let opts = VoicingOptions {
        bass_range: None,
        start: Tick::ZERO,
        ..VoicingOptions::default()
    };
    let realized = provider.realize(chords, &opts).map_err(harmony_err)?;
    let slots: Vec<ChordSlot> = realized
        .iter()
        .map(|rc| ChordSlot {
            start_tick: rc.start.0,
            duration_ticks: rc.duration.0,
            notes: rc.notes.iter().map(|n| n.0).collect(),
            root: rc.chord.bass.unwrap_or(rc.chord.root).value() + 48,
        })
        .collect();
    generate(&GenerationRequest {
        chords: slots,
        meter,
        bars,
        style,
        drums,
        bass,
    })
    .map_err(library_err)
}

/// Eventos MIDI (canal/programa por faixa gerada).
pub fn generated_events(g: &GeneratedBase) -> Vec<MidiNoteEvent> {
    let mut out: Vec<MidiNoteEvent> = g
        .tracks
        .iter()
        .flat_map(|t| {
            t.events.iter().map(move |e| MidiNoteEvent {
                start_tick: e.start_tick,
                duration_ticks: e.duration_ticks,
                channel: t.channel,
                program: t.program,
                key: e.note,
                velocity: e.velocity,
            })
        })
        .collect();
    out.sort_by_key(|e| (e.start_tick, e.channel, e.key));
    out
}

/// Texto canônico da receita: cada acorde analisado no tom maior/menor da base (`"ii7:4 V7:4"`).
/// Garante ida e volta exata (mesmos acordes absolutos) ou falha.
pub fn recipe_text(chords: &[TimedChord], key: BaseKey, beats_per_bar: u8) -> AppResult<String> {
    let k = harmony_key_of(key);
    let text = chords
        .iter()
        .map(|tc| {
            format!(
                "{}:{}",
                RomanChord::analyze(&tc.chord, k).format(k.mode),
                tc.beats
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > 256 {
        return Err(invalid(
            "recipe_too_long",
            "Progressão longa demais para a receita (até 256 caracteres).",
        ));
    }
    let back = recipe_chords(&text, key, beats_per_bar)?;
    if back != chords {
        return Err(invalid(
            "recipe_roundtrip",
            "Não foi possível representar esta progressão na receita da base.",
        ));
    }
    Ok(text)
}

/// Acordes absolutos de uma receita.
pub fn recipe_chords(text: &str, key: BaseKey, beats_per_bar: u8) -> AppResult<Vec<TimedChord>> {
    let k = harmony_key_of(key);
    let steps = Progression::parse_steps(text, k.mode, beats_per_bar).map_err(harmony_err)?;
    if steps.is_empty() {
        return Err(invalid("empty_progression", "Receita sem acordes."));
    }
    steps
        .iter()
        .map(|s| {
            Ok(TimedChord {
                chord: s.chord.resolve(k).map_err(harmony_err)?,
                beats: s.beats,
            })
        })
        .collect()
}

fn style_instruments(
    style: StylePattern,
    drums: DrumPattern,
    bass: BassPattern,
) -> BTreeSet<Instrument> {
    let mut s = BTreeSet::new();
    s.insert(match style.program() {
        0..=7 => Instrument::Piano,
        16..=20 => Instrument::Organ,
        21..=23 => Instrument::Accordion,
        24 => Instrument::NylonGuitar,
        25 => Instrument::AcousticGuitar,
        26..=31 => Instrument::ElectricGuitar,
        _ => Instrument::Other,
    });
    if drums != DrumPattern::None {
        s.insert(Instrument::Drums);
    }
    if bass != BassPattern::None {
        s.insert(Instrument::Bass);
    }
    s
}

fn tags(list: &[String]) -> AppResult<BTreeSet<Tag>> {
    if list.len() > 32 {
        return Err(invalid("too_many_tags", "Use no máximo 32 tags."));
    }
    list.iter()
        .map(|t| Tag::new(t).map_err(library_err))
        .collect()
}

fn genre(g: &Option<String>) -> AppResult<Option<GenreCode>> {
    g.as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| GenreCode::new(s.trim()).map_err(library_err))
        .transpose()
}

impl Composer {
    pub fn search_bases(&self, q: &BaseQuery) -> AppResult<Vec<Base>> {
        Ok(self
            .library
            .bases
            .search(q)
            .map_err(library_err)?
            .into_iter()
            .cloned()
            .collect())
    }

    pub fn base(&self, id: BaseId) -> AppResult<&Base> {
        self.library
            .bases
            .get(id)
            .ok_or_else(|| library_err(ctx_library::LibraryError::BaseNotFound))
    }

    /// Arquivo de áudio de uma base (CC0 → pasta de recursos; usuário → `<dados>/bases`).
    pub fn base_audio_file(&self, base: &Base) -> Option<PathBuf> {
        match &base.kind {
            BaseKind::BundledCc0(a) => Some(self.roots().bundled.join(a.path.as_str())),
            BaseKind::UserAudio(a) => Some(self.roots().user.join(a.path.as_str())),
            BaseKind::Generated(_) => None,
        }
    }

    fn generation_parts(
        &self,
        req: &GenerateBaseRequest,
    ) -> AppResult<(Vec<TimedChord>, Meter, DrumPattern, BassPattern)> {
        check_bpm(req.bpm)?;
        let meter = Meter::new(req.style.meter_numerator(), 4).map_err(library_err)?;
        let resolved = self.resolve_progression(&req.progression, req.key)?;
        Ok((
            resolved.chords,
            meter,
            req.drums.unwrap_or(req.style.default_drums()),
            req.bass.unwrap_or(req.style.default_bass()),
        ))
    }

    /// Gera a base (não grava nada).
    pub fn generate_base(&self, req: &GenerateBaseRequest) -> AppResult<GeneratedBaseView> {
        let (chords, meter, drums, bass) = self.generation_parts(req)?;
        let recipe = recipe_text(&chords, base_key_of(req.key), meter.numerator())?;
        let g = generate_from_chords(
            self.provider(),
            &chords,
            meter,
            req.bars,
            req.style,
            drums,
            bass,
        )?;
        Ok(GeneratedBaseView {
            bpm: req.bpm,
            meter,
            total_ticks: g.total_ticks,
            recipe_progression: recipe,
            drums,
            bass,
            events: generated_events(&g),
        })
    }

    /// Salva a receita no catálogo (desfazível).
    pub fn save_generated_base(&mut self, req: &SaveGeneratedBaseRequest) -> AppResult<Base> {
        let g = &req.generation;
        let (chords, meter, drums, bass) = self.generation_parts(g)?;
        let key = base_key_of(g.key);
        let recipe = GenerationRecipe {
            progression: recipe_text(&chords, key, meter.numerator())?,
            bars: g.bars,
            style: g.style,
            drums,
            bass,
        };
        let mut base = Base::new(
            &req.name,
            BaseKind::Generated(recipe),
            g.bpm,
            Some(key),
            meter,
        )
        .map_err(library_err)?;
        base.genre = genre(&req.genre)?;
        base.tags = tags(&req.tags)?;
        base.instruments = style_instruments(g.style, drums, bass);
        base.validate().map_err(library_err)?;
        self.execute(Box::new(OnBases(Box::new(AddBase::new(
            base.clone(),
            usize::MAX,
        )))))?;
        Ok(base)
    }

    /// Eventos de uma base gerada, opcionalmente transposta (para "Ajustar" ao tom do projeto).
    pub fn generated_base_events(&self, base: &Base, transpose: i8) -> AppResult<GeneratedBase> {
        let BaseKind::Generated(r) = &base.kind else {
            return Err(invalid("not_generated", "A base não é gerada."));
        };
        let key = base
            .key
            .ok_or_else(|| library_err(ctx_library::LibraryError::GeneratedNeedsKey))?;
        let chords: Vec<TimedChord> = recipe_chords(&r.progression, key, base.meter.numerator())?
            .into_iter()
            .map(|tc| TimedChord {
                chord: tc.chord.transpose(transpose as i32),
                beats: tc.beats,
            })
            .collect();
        generate_from_chords(
            self.provider(),
            &chords,
            base.meter,
            r.bars,
            r.style,
            r.drums,
            r.bass,
        )
    }

    /// Importa um arquivo de áudio como base do usuário: **copia** para `<dados>/bases/<id>.<ext>`.
    pub fn add_user_base(&mut self, src: &Path, input: &NewUserBase) -> AppResult<Base> {
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .filter(|e| BASE_AUDIO_EXTENSIONS.contains(&e.as_str()))
            .ok_or_else(|| {
                invalid(
                    "unsupported_audio",
                    "Formato de áudio não suportado (use WAV, FLAC, MP3, OGG ou AIFF).",
                )
            })?;
        if !src.is_file() {
            return Err(invalid(
                "file_not_found",
                "Arquivo de áudio não encontrado.",
            ));
        }
        check_audio_file(src, &ext)?;
        let id = BaseId::new();
        let rel = AudioPath::new(format!("{id}.{ext}")).map_err(library_err)?;
        let mut base = Base::new(
            &input.name,
            BaseKind::UserAudio(AudioBase {
                path: rel,
                progression: input.progression.clone().filter(|p| !p.trim().is_empty()),
            }),
            input.bpm,
            input.key,
            input.meter.unwrap_or(Meter::FOUR_FOUR),
        )
        .map_err(library_err)?;
        base.id = id;
        base.genre = genre(&input.genre)?;
        base.tags = tags(&input.tags)?;
        base.instruments = input.instruments.clone();
        base.validate().map_err(library_err)?;
        let dest = self
            .base_audio_file(&base)
            .ok_or_else(|| invalid("internal", "Base sem arquivo."))?;
        std::fs::create_dir_all(&self.roots().user)
            .map_err(|e| AppError::new("app.io", format!("Erro de arquivo: {e}")))?;
        copy_atomically(src, &dest)?;
        if let Err(e) = self.execute(Box::new(OnBases(Box::new(AddBase::new(
            base.clone(),
            usize::MAX,
        ))))) {
            let _ = std::fs::remove_file(&dest);
            return Err(e);
        }
        Ok(base)
    }

    /// Remove uma base gerada/do usuário (o arquivo copiado fica em disco para permitir desfazer).
    pub fn remove_base(&mut self, id: BaseId) -> AppResult<()> {
        self.execute(Box::new(OnBases(Box::new(RemoveBase::new(id)))))
    }
}

/// Tamanho máximo de uma base do usuário (mesmo limite da importação de áudio).
pub const MAX_BASE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Verificação barata antes de copiar: tamanho e assinatura do contêiner condizente com a extensão.
fn check_audio_file(src: &Path, ext: &str) -> AppResult<()> {
    let io = |e: std::io::Error| AppError::new("app.io", format!("Erro de arquivo: {e}"));
    let len = std::fs::metadata(src).map_err(io)?.len();
    if len > MAX_BASE_BYTES {
        return Err(invalid("too_large", "Arquivo grande demais (máx. 2 GB)."));
    }
    let mut head = [0u8; 12];
    let n =
        std::io::Read::read(&mut std::fs::File::open(src).map_err(io)?, &mut head).map_err(io)?;
    if !audio_signature_matches(&head[..n], ext) {
        return Err(invalid(
            "unsupported_audio",
            "O conteúdo do arquivo não é um áudio válido para a extensão.",
        ));
    }
    Ok(())
}

/// Assinatura mágica dos formatos aceitos.
fn audio_signature_matches(h: &[u8], ext: &str) -> bool {
    match ext {
        "wav" => h.len() >= 12 && &h[0..4] == b"RIFF" && &h[8..12] == b"WAVE",
        "flac" => h.starts_with(b"fLaC"),
        "ogg" => h.starts_with(b"OggS"),
        "aiff" | "aif" => {
            h.len() >= 12 && &h[0..4] == b"FORM" && (&h[8..12] == b"AIFF" || &h[8..12] == b"AIFC")
        }
        "mp3" => h.starts_with(b"ID3") || (h.len() >= 2 && h[0] == 0xFF && h[1] & 0xE0 == 0xE0),
        _ => false,
    }
}

/// Copia para um temporário ao lado do destino e renomeia: nunca deixa base pela metade.
fn copy_atomically(src: &Path, dest: &Path) -> AppResult<()> {
    let io = |e: std::io::Error| AppError::new("app.io", format!("Erro de arquivo: {e}"));
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    if let Err(e) = std::fs::copy(src, &tmp).and_then(|_| std::fs::rename(&tmp, dest)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(e));
    }
    Ok(())
}

impl Composer {
    /// Apaga arquivos da pasta de bases do usuário que nenhuma base referencia
    /// (bases removidas em sessões anteriores, cópias `.tmp` interrompidas).
    /// Chamado ao abrir: o histórico de desfazer está vazio.
    pub fn collect_unreferenced_user_bases(&self) {
        let referenced: BTreeSet<PathBuf> = self
            .library
            .bases
            .bases()
            .iter()
            .filter_map(|b| self.base_audio_file(b))
            .collect();
        let Ok(rd) = std::fs::read_dir(&self.roots().user) else {
            return;
        };
        for p in rd.filter_map(Result::ok).map(|e| e.path()) {
            if p.is_file() && !referenced.contains(&p) {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

// ------------------------------------------------------------- manifesto CC0 e bases iniciais

/// Manifesto das bases CC0 empacotadas (`assets/bases/manifest.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledManifest {
    pub version: u32,
    #[serde(default)]
    pub bases: Vec<BundledBaseEntry>,
}

/// Uma base CC0: o arquivo fica em `assets/bases/<file>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledBaseEntry {
    pub id: BaseId,
    pub name: String,
    pub file: String,
    pub bpm: f64,
    #[serde(default)]
    pub key: Option<BaseKey>,
    #[serde(default)]
    pub meter: Option<Meter>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub instruments: BTreeSet<Instrument>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub progression: Option<String>,
    /// Licença SPDX; só `CC0-1.0` é aceita.
    pub license: String,
    /// Origem/autor (atribuição opcional).
    #[serde(default)]
    pub source: Option<String>,
}

impl BundledManifest {
    pub fn parse(json: &str) -> AppResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            invalid(
                "invalid_manifest",
                format!("Manifesto de bases inválido: {e}"),
            )
        })
    }

    /// Valida e converte em bases CC0 (somente leitura no catálogo).
    pub fn bases(&self) -> AppResult<Vec<Base>> {
        self.bases
            .iter()
            .map(|e| {
                if e.license != "CC0-1.0" {
                    return Err(invalid(
                        "invalid_manifest",
                        format!("A base \"{}\" não é CC0-1.0.", e.name),
                    ));
                }
                let mut b = Base::new(
                    &e.name,
                    BaseKind::BundledCc0(AudioBase {
                        path: AudioPath::new(e.file.clone()).map_err(library_err)?,
                        progression: e.progression.clone(),
                    }),
                    e.bpm,
                    e.key,
                    e.meter.unwrap_or(Meter::FOUR_FOUR),
                )
                .map_err(library_err)?;
                b.id = e.id;
                b.genre = genre(&e.genre)?;
                b.instruments = e.instruments.clone();
                b.tags = tags(&e.tags)?;
                b.validate().map_err(library_err)?;
                Ok(b)
            })
            .collect()
    }
}

struct Starter {
    n: u128,
    name: &'static str,
    tonic: u8,
    minor: bool,
    bpm: f64,
    progression: &'static str,
    bars: u16,
    style: StylePattern,
    genre: &'static str,
    tags: &'static [&'static str],
}

#[rustfmt::skip]
const STARTERS: &[Starter] = &[
    Starter { n: 1, name: "Pop em Dó (I–V–vi–IV)", tonic: 0, minor: false, bpm: 100.0, progression: "I:4 V:4 vi:4 IV:4", bars: 8, style: StylePattern::PopStrum, genre: "pop", tags: &["inicial", "animada"] },
    Starter { n: 2, name: "Balada em Sol", tonic: 7, minor: false, bpm: 72.0, progression: "I:4 vi:4 IV:4 V:4", bars: 8, style: StylePattern::BalladArpeggio, genre: "ballad", tags: &["inicial", "calma"] },
    Starter { n: 3, name: "Bossa em Ré (ii–V–I)", tonic: 2, minor: false, bpm: 130.0, progression: "ii7:4 V7:4 Imaj7:8", bars: 8, style: StylePattern::BossaComping, genre: "bossa_nova", tags: &["inicial", "suave"] },
    Starter { n: 4, name: "Samba em Lá", tonic: 9, minor: false, bpm: 100.0, progression: "I:4 vi7:4 ii7:4 V7:4", bars: 8, style: StylePattern::SambaComping, genre: "samba", tags: &["inicial", "brasileira"] },
    Starter { n: 5, name: "Rock em Mi", tonic: 4, minor: false, bpm: 120.0, progression: "I:4 IV:4 V:4 IV:4", bars: 8, style: StylePattern::RockEighths, genre: "rock", tags: &["inicial", "energia"] },
    Starter { n: 6, name: "Reggae em Sol", tonic: 7, minor: false, bpm: 76.0, progression: "I:4 IV:4", bars: 8, style: StylePattern::ReggaeSkank, genre: "reggae", tags: &["inicial"] },
    Starter { n: 7, name: "Funk em Mi menor", tonic: 4, minor: true, bpm: 104.0, progression: "i7:8 iv7:8", bars: 8, style: StylePattern::FunkStabs, genre: "funk", tags: &["inicial", "groove"] },
    Starter { n: 8, name: "Valsa em Fá", tonic: 5, minor: false, bpm: 150.0, progression: "I:3 IV:3 V7:3 I:3", bars: 8, style: StylePattern::Waltz, genre: "ballad", tags: &["inicial", "3/4"] },
];

/// Bases iniciais (receitas geradas), com ids estáveis para a semeadura ser idempotente.
pub fn starter_bases() -> AppResult<Vec<Base>> {
    STARTERS
        .iter()
        .map(|s| {
            let tonic = PitchClass::new(s.tonic).ok_or_else(|| invalid("internal", "tônica"))?;
            let key = BaseKey::new(
                tonic,
                if s.minor {
                    KeyMode::Minor
                } else {
                    KeyMode::Major
                },
            );
            let meter = Meter::new(s.style.meter_numerator(), 4).map_err(library_err)?;
            let mut b = Base::new(
                s.name,
                BaseKind::Generated(GenerationRecipe {
                    progression: s.progression.into(),
                    bars: s.bars,
                    style: s.style,
                    drums: s.style.default_drums(),
                    bass: s.style.default_bass(),
                }),
                s.bpm,
                Some(key),
                meter,
            )
            .map_err(library_err)?;
            b.id = BaseId(uuid::Uuid::from_u128(
                0x0190_0000_0000_7000_8000_5ba5_e000_0000 + s.n,
            ));
            b.genre = Some(GenreCode::new(s.genre).map_err(library_err)?);
            b.tags = s
                .tags
                .iter()
                .map(|t| Tag::new(t).map_err(library_err))
                .collect::<AppResult<_>>()?;
            b.instruments =
                style_instruments(s.style, s.style.default_drums(), s.style.default_bass());
            Ok(b)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::{BaseRoots, Composer, MemoryLibraryStore};
    use ctx_harmony::builtin_catalog;

    fn composer() -> Composer {
        Composer::open(
            Box::new(MemoryLibraryStore::default()),
            BaseRoots {
                user: PathBuf::from("/tmp/nao-usado"),
                bundled: PathBuf::from("/tmp/nao-usado"),
            },
        )
        .unwrap()
    }

    #[test]
    fn recipe_text_roundtrips_whole_catalog_in_its_base_key() {
        let c = composer();
        let mut ok = 0;
        for p in builtin_catalog() {
            for tonic in [0u8, 3, 7, 10] {
                let key = Key::new(PitchClass::new(tonic).unwrap(), p.mode);
                let chords = c
                    .resolve_progression(&ProgressionRef::Builtin { slug: slug(p) }, key)
                    .unwrap()
                    .chords;
                // receitas longas podem exceder 256 caracteres — erro tipado, nunca perda
                match recipe_text(&chords, base_key_of(key), p.beats_per_bar) {
                    Ok(t) => {
                        assert_eq!(
                            recipe_chords(&t, base_key_of(key), p.beats_per_bar).unwrap(),
                            chords
                        );
                        ok += 1;
                    }
                    Err(e) => assert_eq!(e.code, "composition.recipe_too_long", "{}", p.name),
                }
            }
        }
        assert!(ok > builtin_catalog().len() * 3);
    }

    fn slug(p: &Progression) -> String {
        match &p.key {
            ctx_harmony::ProgressionKey::Builtin(s) => s.clone(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn starter_bases_are_valid_and_generate() {
        let c = composer();
        let list = starter_bases().unwrap();
        assert!(list.len() >= 6);
        for b in &list {
            b.validate().unwrap();
            let g = c.generated_base_events(b, 0).unwrap();
            assert_eq!(g.total_ticks, b.meter.bar_ticks() * 8, "{}", b.name);
            assert!(!generated_events(&g).is_empty());
        }
        // ids estáveis
        assert_eq!(starter_bases().unwrap()[0].id, list[0].id);
    }

    #[test]
    fn manifest_parses_and_rejects_non_cc0() {
        let empty =
            BundledManifest::parse(include_str!("../../../../assets/bases/manifest.json")).unwrap();
        assert!(empty.bases().unwrap().is_empty());
        let json = r#"{"version":2,"bases":[{"id":"01900000-0000-7000-8000-000000000001",
            "name":"Groove CC0","file":"pop/groove.ogg","bpm":96,"license":"CC0-1.0",
            "key":{"tonic":9,"mode":"minor"},"tags":["Pop"],"instruments":["drums"]}]}"#;
        let m = BundledManifest::parse(json).unwrap();
        let b = m.bases().unwrap();
        assert_eq!(b[0].kind.tag(), ctx_library::BaseKindTag::BundledCc0);
        assert_eq!(b[0].meter, Meter::FOUR_FOUR);
        let bad = json.replace("CC0-1.0", "CC-BY-4.0");
        assert_eq!(
            BundledManifest::parse(&bad)
                .unwrap()
                .bases()
                .unwrap_err()
                .code,
            "composition.invalid_manifest"
        );
        let traversal = json.replace("pop/groove.ogg", "../../etc/passwd");
        assert!(BundledManifest::parse(&traversal).unwrap().bases().is_err());
        assert!(BundledManifest::parse("{").is_err());
    }
}
