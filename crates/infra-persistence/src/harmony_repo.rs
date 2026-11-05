//! Repositórios do `library.db` para Harmonia (Fase 5) e Bases (Fase 4) — Pure Fabrication.
//!
//! Esquema: `ctx_harmony::LIBRARY_SCHEMA_V1` (migração 2 do `library.db`). Regras:
//!
//! - **Semeadura** a partir do catálogo Rust (fonte única): vocabulário sempre por *upsert* de
//!   código (idempotente); progressões embutidas regravadas só quando
//!   `seed_meta['harmony_catalog_version']` < [`ctx_harmony::CATALOG_VERSION`]; bases CC0 do
//!   manifesto e bases "iniciais" geradas, cada uma com sua versão em `seed_meta`.
//!   **Dados do usuário nunca são tocados pela semeadura.**
//! - Salvar = uma transação que regrava o conjunto do usuário (progressões do usuário e bases não
//!   CC0), preservando a ordem (ordem de inserção = `id`).
//! - Todas as consultas são parametrizadas (`?N`).

use crate::library::LibraryDb;
use crate::PersistenceError;
use app_core::composition::{Library, LibraryStore};
use app_core::ports::PortError;
use ctx_harmony::{
    builtin_catalog, ChordQuality, Degree, Genre, HarmonyWorkspace, Mode, Progression,
    ProgressionId, ProgressionKey, ProgressionStep, QualitySpec, RomanChord, ScaleFamily, Tension,
    CATALOG_VERSION,
};
use ctx_library::{
    AudioBase, AudioPath, Base, BaseCatalog, BaseId, BaseKey, BaseKind, BaseKindTag, BassPattern,
    DrumPattern, GenerationRecipe, GenreCode, Instrument, KeyMode, Meter, StylePattern, Tag,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use shared_kernel::music::PitchClass;
use std::collections::BTreeSet;

/// Chaves de `seed_meta`.
pub const META_CATALOG_VERSION: &str = "harmony_catalog_version";
pub const META_BUNDLED_BASES_VERSION: &str = "bundled_bases_version";
pub const META_STARTER_BASES_VERSION: &str = "starter_bases_version";

fn corrupt(msg: impl Into<String>) -> PersistenceError {
    PersistenceError::Corrupt(msg.into())
}

fn meta(conn: &Connection, key: &str) -> Result<u32, PersistenceError> {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM seed_meta WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()?;
    Ok(v.and_then(|s| s.parse().ok()).unwrap_or(0))
}

fn set_meta(tx: &Transaction<'_>, key: &str, value: u32) -> Result<(), PersistenceError> {
    tx.execute(
        "INSERT INTO seed_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )?;
    Ok(())
}

// ------------------------------------------------------------------ vocabulário

fn seed_vocabulary(tx: &Transaction<'_>) -> Result<(), PersistenceError> {
    for f in ScaleFamily::ALL {
        tx.execute(
            "INSERT INTO scale_family (code) VALUES (?1) ON CONFLICT(code) DO NOTHING",
            params![f.code()],
        )?;
        for (i, semis) in f.intervals().iter().enumerate() {
            tx.execute(
                "INSERT INTO scale_family_interval (family_id, position, semitones)
                 VALUES ((SELECT id FROM scale_family WHERE code = ?1), ?2, ?3)
                 ON CONFLICT(family_id, position) DO UPDATE SET semitones = excluded.semitones",
                params![f.code(), i as i64 + 1, *semis as i64],
            )?;
        }
    }
    for m in Mode::ALL {
        tx.execute(
            "INSERT INTO mode (code, name_pt, family_id, rotation)
             VALUES (?1, ?2, (SELECT id FROM scale_family WHERE code = ?3), ?4)
             ON CONFLICT(code) DO UPDATE SET name_pt = excluded.name_pt",
            params![
                m.code(),
                m.name_pt(),
                m.family().code(),
                m.rotation() as i64
            ],
        )?;
    }
    for q in ChordQuality::ALL {
        tx.execute(
            "INSERT INTO chord_quality (code) VALUES (?1) ON CONFLICT(code) DO NOTHING",
            params![q.code()],
        )?;
        for s in q.intervals() {
            tx.execute(
                "INSERT INTO chord_quality_interval (quality_id, semitones)
                 VALUES ((SELECT id FROM chord_quality WHERE code = ?1), ?2)
                 ON CONFLICT DO NOTHING",
                params![q.code(), *s as i64],
            )?;
        }
    }
    for t in Tension::ALL {
        tx.execute(
            "INSERT INTO tension (code, semitones) VALUES (?1, ?2) ON CONFLICT(code) DO NOTHING",
            params![t.symbol(), t.semitones() as i64],
        )?;
    }
    for g in Genre::ALL {
        tx.execute(
            "INSERT INTO genre (code, name_pt) VALUES (?1, ?2)
             ON CONFLICT(code) DO UPDATE SET name_pt = excluded.name_pt",
            params![g.code(), g.name_pt()],
        )?;
    }
    for i in Instrument::ALL {
        tx.execute(
            "INSERT INTO instrument (code, name_pt) VALUES (?1, ?2)
             ON CONFLICT(code) DO UPDATE SET name_pt = excluded.name_pt",
            params![i.code(), i.name_pt()],
        )?;
    }
    for s in StylePattern::ALL {
        tx.execute(
            "INSERT INTO style_pattern (code, name_pt, meter_numerator) VALUES (?1, ?2, ?3)
             ON CONFLICT(code) DO UPDATE SET name_pt = excluded.name_pt,
                                             meter_numerator = excluded.meter_numerator",
            params![s.code(), s.name_pt(), s.meter_numerator() as i64],
        )?;
    }
    for d in DrumPattern::ALL {
        tx.execute(
            "INSERT INTO drum_pattern (code, name_pt) VALUES (?1, ?2)
             ON CONFLICT(code) DO UPDATE SET name_pt = excluded.name_pt",
            params![d.code(), d.name_pt()],
        )?;
    }
    for b in BassPattern::ALL {
        tx.execute(
            "INSERT INTO bass_pattern (code, name_pt) VALUES (?1, ?2)
             ON CONFLICT(code) DO UPDATE SET name_pt = excluded.name_pt",
            params![b.code(), b.name_pt()],
        )?;
    }
    Ok(())
}

// ------------------------------------------------------------------ progressões

fn insert_progression(tx: &Transaction<'_>, p: &Progression) -> Result<i64, PersistenceError> {
    tx.execute(
        "INSERT INTO progression (name, mode_id, is_universal, beats_per_bar)
         VALUES (?1, (SELECT id FROM mode WHERE code = ?2), ?3, ?4)",
        params![
            p.name,
            p.mode.code(),
            p.universal as i64,
            p.beats_per_bar as i64
        ],
    )?;
    let id = tx.last_insert_rowid();
    match &p.key {
        ProgressionKey::Builtin(slug) => {
            tx.execute(
                "INSERT INTO builtin_progression (progression_id, slug, catalog_version)
                 VALUES (?1, ?2, ?3)",
                params![id, slug, CATALOG_VERSION as i64],
            )?;
        }
        ProgressionKey::User(uid) => {
            tx.execute(
                "INSERT INTO user_progression (progression_id, uuid) VALUES (?1, ?2)",
                params![id, uid.to_string()],
            )?;
        }
    }
    for g in &p.genres {
        tx.execute(
            "INSERT INTO progression_genre (progression_id, genre_id)
             SELECT ?1, id FROM genre WHERE code = ?2",
            params![id, g.code()],
        )?;
    }
    for (pos, s) in p.steps.iter().enumerate() {
        let c = &s.chord;
        let (quality, seventh) = match c.quality {
            QualitySpec::Diatonic => (None, 0i64),
            QualitySpec::DiatonicSeventh => (None, 1),
            QualitySpec::Explicit(q) => (Some(q.code()), 0),
        };
        tx.execute(
            "INSERT INTO progression_step
               (progression_id, position, degree, accidental, quality_id, diatonic_seventh,
                bass_degree, bass_accidental, target_degree, target_accidental, beats)
             VALUES (?1, ?2, ?3, ?4, (SELECT id FROM chord_quality WHERE code = ?5), ?6,
                     ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                pos as i64,
                c.degree.step() as i64,
                c.degree.accidental() as i64,
                quality,
                seventh,
                c.bass.map(|d| d.step() as i64),
                c.bass.map(|d| d.accidental() as i64),
                c.of.map(|d| d.step() as i64),
                c.of.map(|d| d.accidental() as i64),
                s.beats as i64,
            ],
        )?;
        for t in &c.tensions {
            tx.execute(
                "INSERT INTO progression_step_tension (progression_id, position, tension_id)
                 SELECT ?1, ?2, id FROM tension WHERE code = ?3",
                params![id, pos as i64, t.symbol()],
            )?;
        }
    }
    Ok(id)
}

fn degree(step: Option<i64>, acc: Option<i64>) -> Result<Option<Degree>, PersistenceError> {
    match (step, acc) {
        (Some(s), Some(a)) => Degree::new(s as u8, a as i8)
            .map(Some)
            .map_err(|e| corrupt(e.to_string())),
        _ => Ok(None),
    }
}

fn load_steps(
    conn: &Connection,
    progression_id: i64,
) -> Result<Vec<ProgressionStep>, PersistenceError> {
    let mut st = conn.prepare_cached(
        "SELECT degree, accidental, quality_code, diatonic_seventh, bass_degree, bass_accidental,
                target_degree, target_accidental, beats, tension_codes
         FROM v_progression_step WHERE progression_id = ?1 ORDER BY position",
    )?;
    let rows = st.query_map(params![progression_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, Option<i64>>(5)?,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, Option<i64>>(7)?,
            r.get::<_, i64>(8)?,
            r.get::<_, Option<String>>(9)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (deg, acc, q, seventh, bd, ba, td, ta, beats, tensions) = row?;
        let quality = match q {
            Some(code) => QualitySpec::Explicit(
                ChordQuality::from_code(&code)
                    .ok_or_else(|| corrupt(format!("qualidade {code}")))?,
            ),
            None if seventh == 1 => QualitySpec::DiatonicSeventh,
            None => QualitySpec::Diatonic,
        };
        let mut chord = RomanChord::new(
            Degree::new(deg as u8, acc as i8).map_err(|e| corrupt(e.to_string()))?,
            quality,
        );
        chord.bass = degree(bd, ba)?;
        chord.of = degree(td, ta)?;
        for code in tensions.iter().flat_map(|t| t.split(',')) {
            let t = Tension::ALL
                .into_iter()
                .find(|t| t.symbol() == code)
                .ok_or_else(|| corrupt(format!("tensão {code}")))?;
            chord.tensions.insert(t);
        }
        out.push(ProgressionStep {
            chord,
            beats: u16::try_from(beats).map_err(|_| corrupt("duração"))?,
        });
    }
    Ok(out)
}

fn load_progressions(
    conn: &Connection,
    origin: &str,
) -> Result<Vec<Progression>, PersistenceError> {
    let mut st = conn.prepare(
        "SELECT id, name, mode_code, slug, uuid, is_universal, beats_per_bar, genre_codes
         FROM v_progression_summary WHERE origin = ?1 ORDER BY id",
    )?;
    let rows = st.query_map(params![origin], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, Option<String>>(7)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, name, mode, slug, uuid, universal, bpb, genres) = row?;
        let key = match (slug, uuid) {
            (Some(s), _) => ProgressionKey::Builtin(s),
            (None, Some(u)) => ProgressionKey::User(
                u.parse::<ProgressionId>()
                    .map_err(|e| corrupt(e.to_string()))?,
            ),
            (None, None) => return Err(corrupt("progressão sem origem")),
        };
        let mut genre_set = BTreeSet::new();
        for code in genres.iter().flat_map(|g| g.split(',')) {
            genre_set.insert(Genre::from_code(code).map_err(|e| corrupt(e.to_string()))?);
        }
        let p = Progression {
            key,
            name,
            mode: Mode::from_code(&mode).map_err(|e| corrupt(e.to_string()))?,
            genres: genre_set,
            universal: universal == 1,
            beats_per_bar: u8::try_from(bpb).map_err(|_| corrupt("compasso"))?,
            steps: load_steps(conn, id)?,
        };
        p.validate().map_err(|e| corrupt(e.to_string()))?;
        out.push(p);
    }
    Ok(out)
}

// ------------------------------------------------------------------ bases

fn ensure_genre(tx: &Transaction<'_>, g: &GenreCode) -> Result<(), PersistenceError> {
    tx.execute(
        "INSERT INTO genre (code, name_pt) VALUES (?1, ?1) ON CONFLICT(code) DO NOTHING",
        params![g.as_str()],
    )?;
    Ok(())
}

fn insert_base(tx: &Transaction<'_>, b: &Base) -> Result<(), PersistenceError> {
    if let Some(g) = &b.genre {
        ensure_genre(tx, g)?;
    }
    tx.execute(
        "INSERT INTO base (uuid, name, kind, bpm, tonic, key_mode, meter_numerator,
                           meter_denominator, genre_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, (SELECT id FROM genre WHERE code = ?9))",
        params![
            b.id.to_string(),
            b.name,
            b.kind.tag().code(),
            b.bpm,
            b.key.map(|k| k.tonic.value() as i64),
            b.key.map(|k| k.mode.code()),
            b.meter.numerator() as i64,
            b.meter.denominator() as i64,
            b.genre.as_ref().map(|g| g.as_str()),
        ],
    )?;
    let id = tx.last_insert_rowid();
    match &b.kind {
        BaseKind::BundledCc0(a) | BaseKind::UserAudio(a) => {
            tx.execute(
                "INSERT INTO base_audio (base_id, rel_path, progression_text) VALUES (?1, ?2, ?3)",
                params![id, a.path.as_str(), a.progression],
            )?;
        }
        BaseKind::Generated(r) => {
            tx.execute(
                "INSERT INTO base_recipe (base_id, progression_text, bars, style_id,
                                          drum_pattern_id, bass_pattern_id)
                 VALUES (?1, ?2, ?3,
                         (SELECT id FROM style_pattern WHERE code = ?4),
                         (SELECT id FROM drum_pattern WHERE code = ?5),
                         (SELECT id FROM bass_pattern WHERE code = ?6))",
                params![
                    id,
                    r.progression,
                    r.bars as i64,
                    r.style.code(),
                    r.drums.code(),
                    r.bass.code()
                ],
            )?;
        }
    }
    for i in &b.instruments {
        tx.execute(
            "INSERT INTO base_instrument (base_id, instrument_id)
             SELECT ?1, id FROM instrument WHERE code = ?2",
            params![id, i.code()],
        )?;
    }
    for t in &b.tags {
        tx.execute(
            "INSERT INTO tag (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![t.as_str()],
        )?;
        tx.execute(
            "INSERT INTO base_tag (base_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
            params![id, t.as_str()],
        )?;
    }
    Ok(())
}

fn find_code<T: Copy>(all: &[T], code: &str, f: impl Fn(T) -> &'static str) -> Option<T> {
    all.iter().copied().find(|x| f(*x) == code)
}

fn load_bases(conn: &Connection) -> Result<Vec<Base>, PersistenceError> {
    let mut st = conn.prepare(
        "SELECT b.uuid, b.name, b.kind, b.bpm, b.tonic, b.key_mode, b.meter_numerator,
                b.meter_denominator, g.code, a.rel_path, a.progression_text,
                r.progression_text, r.bars, sp.code, dp.code, bp.code,
                (SELECT group_concat(i.code, ',') FROM base_instrument bi
                   JOIN instrument i ON i.id = bi.instrument_id WHERE bi.base_id = b.id),
                (SELECT group_concat(t.name, ',') FROM base_tag bt
                   JOIN tag t ON t.id = bt.tag_id WHERE bt.base_id = b.id)
         FROM base b
         LEFT JOIN genre g          ON g.id = b.genre_id
         LEFT JOIN base_audio a     ON a.base_id = b.id
         LEFT JOIN base_recipe r    ON r.base_id = b.id
         LEFT JOIN style_pattern sp ON sp.id = r.style_id
         LEFT JOIN drum_pattern dp  ON dp.id = r.drum_pattern_id
         LEFT JOIN bass_pattern bp  ON bp.id = r.bass_pattern_id
         ORDER BY b.id",
    )?;
    type Row = (
        String,
        String,
        String,
        f64,
        Option<i64>,
        Option<String>,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let rows = st.query_map([], |r| -> rusqlite::Result<Row> {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
            r.get(6)?,
            r.get(7)?,
            r.get(8)?,
            r.get(9)?,
            r.get(10)?,
            r.get(11)?,
            r.get(12)?,
            r.get(13)?,
            r.get(14)?,
            r.get(15)?,
            r.get(16)?,
            r.get(17)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (
            uuid,
            name,
            kind,
            bpm,
            tonic,
            key_mode,
            num,
            den,
            genre,
            rel_path,
            audio_prog,
            recipe_prog,
            bars,
            style,
            drums,
            bass,
            instruments,
            tags,
        ) = row?;
        let c = |e: ctx_library::LibraryError| corrupt(e.to_string());
        let audio = || -> Result<AudioBase, PersistenceError> {
            Ok(AudioBase {
                path: AudioPath::new(rel_path.clone().ok_or_else(|| corrupt("base sem áudio"))?)
                    .map_err(c)?,
                progression: audio_prog.clone(),
            })
        };
        let kind = match kind.as_str() {
            "bundled_cc0" => BaseKind::BundledCc0(audio()?),
            "user_audio" => BaseKind::UserAudio(audio()?),
            "generated" => {
                let miss = || corrupt("receita incompleta");
                BaseKind::Generated(GenerationRecipe {
                    progression: recipe_prog.clone().ok_or_else(miss)?,
                    bars: u16::try_from(bars.ok_or_else(miss)?).map_err(|_| miss())?,
                    style: find_code(&StylePattern::ALL, &style.ok_or_else(miss)?, |s| s.code())
                        .ok_or_else(miss)?,
                    drums: find_code(&DrumPattern::ALL, &drums.ok_or_else(miss)?, |s| s.code())
                        .ok_or_else(miss)?,
                    bass: find_code(&BassPattern::ALL, &bass.ok_or_else(miss)?, |s| s.code())
                        .ok_or_else(miss)?,
                })
            }
            other => return Err(corrupt(format!("tipo de base {other}"))),
        };
        let key = match (tonic, key_mode.as_deref()) {
            (Some(t), Some(m)) => Some(BaseKey::new(
                PitchClass::new(t as u8).ok_or_else(|| corrupt("tônica"))?,
                if m == "minor" {
                    KeyMode::Minor
                } else {
                    KeyMode::Major
                },
            )),
            _ => None,
        };
        let mut b = Base {
            id: uuid.parse::<BaseId>().map_err(|e| corrupt(e.to_string()))?,
            name,
            kind,
            bpm,
            key,
            meter: Meter::new(num as u8, den as u8).map_err(c)?,
            genre: genre
                .as_deref()
                .map(GenreCode::new)
                .transpose()
                .map_err(c)?,
            instruments: BTreeSet::new(),
            tags: BTreeSet::new(),
        };
        for code in instruments.iter().flat_map(|s| s.split(',')) {
            b.instruments
                .insert(Instrument::from_code(code).map_err(c)?);
        }
        for t in tags.iter().flat_map(|s| s.split(',')) {
            b.tags.insert(Tag::new(t).map_err(c)?);
        }
        out.push(b);
    }
    Ok(out)
}

// ------------------------------------------------------------------ API pública

impl LibraryDb {
    /// Semeia vocabulário (sempre, idempotente) e progressões embutidas (se a versão aumentou).
    pub fn seed_catalog(&self) -> Result<(), PersistenceError> {
        self.seed_builtin_progressions(builtin_catalog(), CATALOG_VERSION)
    }

    /// Versão parametrizável (testes de atualização do catálogo). Só toca progressões embutidas.
    pub fn seed_builtin_progressions(
        &self,
        catalog: &[Progression],
        version: u32,
    ) -> Result<(), PersistenceError> {
        let conn = self.connection();
        let tx = conn.unchecked_transaction()?;
        seed_vocabulary(&tx)?;
        if meta(&tx, META_CATALOG_VERSION)? < version {
            tx.execute(
                "DELETE FROM progression
                 WHERE id IN (SELECT progression_id FROM builtin_progression)",
                [],
            )?;
            for p in catalog.iter().filter(|p| !p.key.is_user()) {
                insert_progression(&tx, p)?;
            }
            set_meta(&tx, META_CATALOG_VERSION, version)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Versão do catálogo embutido já aplicada.
    pub fn catalog_version(&self) -> Result<u32, PersistenceError> {
        meta(self.connection(), META_CATALOG_VERSION)
    }

    /// Progressões embutidas como estão no banco (leitura/diagnóstico).
    pub fn builtin_progressions(&self) -> Result<Vec<Progression>, PersistenceError> {
        load_progressions(self.connection(), "builtin")
    }

    pub fn load_user_progressions(&self) -> Result<HarmonyWorkspace, PersistenceError> {
        let list = load_progressions(self.connection(), "user")?;
        HarmonyWorkspace::from_progressions(list).map_err(|e| corrupt(e.to_string()))
    }

    pub fn save_user_progressions(&self, ws: &HarmonyWorkspace) -> Result<(), PersistenceError> {
        let tx = self.connection().unchecked_transaction()?;
        write_user_progressions(&tx, ws)?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_bases(&self) -> Result<BaseCatalog, PersistenceError> {
        BaseCatalog::from_bases(load_bases(self.connection())?).map_err(|e| corrupt(e.to_string()))
    }

    /// Regrava as bases do usuário/geradas (as CC0 incluídas são da semeadura).
    pub fn save_bases(&self, catalog: &BaseCatalog) -> Result<(), PersistenceError> {
        let tx = self.connection().unchecked_transaction()?;
        write_bases(&tx, catalog)?;
        tx.commit()?;
        Ok(())
    }

    /// Bases CC0 do manifesto empacotado: regravadas quando `version` aumenta.
    pub fn seed_bundled_bases(&self, bases: &[Base], version: u32) -> Result<(), PersistenceError> {
        let tx = self.connection().unchecked_transaction()?;
        if meta(&tx, META_BUNDLED_BASES_VERSION)? < version {
            tx.execute("DELETE FROM base WHERE kind = 'bundled_cc0'", [])?;
            for b in bases {
                if b.kind.tag() != BaseKindTag::BundledCc0 {
                    return Err(corrupt("manifesto com base que não é CC0 incluída"));
                }
                b.validate().map_err(|e| corrupt(e.to_string()))?;
                insert_base(&tx, b)?;
            }
            set_meta(&tx, META_BUNDLED_BASES_VERSION, version)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Bases iniciais (receitas geradas): inseridas uma vez por versão; depois são do usuário
    /// (editáveis/removíveis) e nunca são reinseridas por cima.
    pub fn seed_starter_bases(&self, bases: &[Base], version: u32) -> Result<(), PersistenceError> {
        let tx = self.connection().unchecked_transaction()?;
        if meta(&tx, META_STARTER_BASES_VERSION)? < version {
            for b in bases {
                b.validate().map_err(|e| corrupt(e.to_string()))?;
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM base WHERE uuid = ?1)",
                    params![b.id.to_string()],
                    |r| r.get(0),
                )?;
                if !exists {
                    insert_base(&tx, b)?;
                }
            }
            set_meta(&tx, META_STARTER_BASES_VERSION, version)?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn write_user_progressions(
    tx: &Transaction<'_>,
    ws: &HarmonyWorkspace,
) -> Result<(), PersistenceError> {
    tx.execute(
        "DELETE FROM progression WHERE id IN (SELECT progression_id FROM user_progression)",
        [],
    )?;
    for p in ws.user_progressions() {
        insert_progression(tx, p)?;
    }
    Ok(())
}

fn write_bases(tx: &Transaction<'_>, catalog: &BaseCatalog) -> Result<(), PersistenceError> {
    tx.execute("DELETE FROM base WHERE kind <> 'bundled_cc0'", [])?;
    for b in catalog
        .bases()
        .iter()
        .filter(|b| b.kind.tag() != BaseKindTag::BundledCc0)
    {
        insert_base(tx, b)?;
    }
    Ok(())
}

/// O `library.db` é o repositório da biblioteca de composição do `app-core`.
impl LibraryStore for LibraryDb {
    fn load(&self) -> Result<Library, PortError> {
        Ok(Library {
            harmony: self.load_user_progressions()?,
            bases: self.load_bases()?,
        })
    }

    fn save(&self, library: &Library) -> Result<(), PortError> {
        let run = || -> Result<(), PersistenceError> {
            let tx = self.connection().unchecked_transaction()?;
            write_user_progressions(&tx, &library.harmony)?;
            write_bases(&tx, &library.bases)?;
            tx.commit()?;
            Ok(())
        };
        Ok(run()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctx_library::{AudioBase, BaseKind};

    fn db() -> (tempfile::TempDir, LibraryDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = LibraryDb::open(&dir.path().join("library.db")).unwrap();
        (dir, db)
    }

    fn count(db: &LibraryDb, sql: &str) -> i64 {
        db.connection().query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn user(name: &str, steps: &str) -> Progression {
        let mut g = BTreeSet::new();
        g.insert(Genre::Samba);
        Progression::new_user(name, Mode::Dorian, g, 4, steps).unwrap()
    }

    fn generated(name: &str) -> Base {
        let c = PitchClass::new(2).unwrap();
        let mut b = Base::new(
            name,
            BaseKind::Generated(GenerationRecipe {
                progression: "ii7:4 V7:4 I:8".into(),
                bars: 8,
                style: StylePattern::BossaComping,
                drums: DrumPattern::Bossa,
                bass: BassPattern::BossaRootFifth,
            }),
            128.0,
            Some(BaseKey::new(c, KeyMode::Major)),
            Meter::FOUR_FOUR,
        )
        .unwrap();
        b.genre = Some(GenreCode::new("bossa_nova").unwrap());
        b.instruments.insert(Instrument::NylonGuitar);
        b.instruments.insert(Instrument::Drums);
        b.tags.insert(Tag::new("suave").unwrap());
        b.tags.insert(Tag::new("Noite Fria").unwrap());
        b
    }

    fn audio(name: &str, path: &str, kind_cc0: bool) -> Base {
        let a = AudioBase {
            path: AudioPath::new(path).unwrap(),
            progression: Some("C G Am F".into()),
        };
        let kind = if kind_cc0 {
            BaseKind::BundledCc0(a)
        } else {
            BaseKind::UserAudio(a)
        };
        let mut b = Base::new(name, kind, 90.5, None, Meter::THREE_FOUR).unwrap();
        b.genre = Some(GenreCode::new("valsa_caipira").unwrap());
        b
    }

    #[test]
    fn open_seeds_vocabulary_and_catalog_and_is_idempotent() {
        let (dir, db) = db();
        assert_eq!(db.catalog_version().unwrap(), CATALOG_VERSION);
        let n_prog = count(&db, "SELECT count(*) FROM builtin_progression");
        assert_eq!(n_prog as usize, builtin_catalog().len());
        assert_eq!(count(&db, "SELECT count(*) FROM mode"), 21);
        assert_eq!(count(&db, "SELECT count(*) FROM chord_quality"), 18);
        // a view de intervalos bate com o domínio (sem dados deriváveis armazenados)
        for m in Mode::ALL {
            let mut st = db
                .connection()
                .prepare(
                    "SELECT semitones FROM v_mode_interval WHERE mode_code = ?1 ORDER BY degree",
                )
                .unwrap();
            let iv: Vec<u8> = st
                .query_map(params![m.code()], |r| r.get::<_, i64>(0))
                .unwrap()
                .map(|x| x.unwrap() as u8)
                .collect();
            assert_eq!(iv, m.intervals().to_vec(), "{}", m.code());
        }
        drop(db);
        let db = LibraryDb::open(&dir.path().join("library.db")).unwrap();
        db.seed_catalog().unwrap();
        assert_eq!(
            count(&db, "SELECT count(*) FROM builtin_progression") as usize,
            builtin_catalog().len()
        );
        assert_eq!(count(&db, "SELECT count(*) FROM mode"), 21);
        // ida e volta do catálogo inteiro pelo banco
        assert_eq!(
            db.builtin_progressions().unwrap(),
            builtin_catalog().to_vec()
        );
    }

    #[test]
    fn reseed_on_version_bump_keeps_user_progressions() {
        let (_dir, db) = db();
        let mine = user("Minha; DROP TABLE progression; --", "i7 IV7 bVII:8");
        let ws = HarmonyWorkspace::from_progressions(vec![mine.clone()]).unwrap();
        db.save_user_progressions(&ws).unwrap();
        // catálogo "v+1": uma progressão renomeada, as outras removidas
        let mut changed = builtin_catalog()[0].clone();
        changed.name = "Renomeada".into();
        db.seed_builtin_progressions(std::slice::from_ref(&changed), CATALOG_VERSION + 1)
            .unwrap();
        assert_eq!(db.builtin_progressions().unwrap(), vec![changed.clone()]);
        assert_eq!(db.catalog_version().unwrap(), CATALOG_VERSION + 1);
        assert_eq!(db.load_user_progressions().unwrap(), ws);
        // mesma versão: nada muda
        db.seed_builtin_progressions(builtin_catalog(), CATALOG_VERSION + 1)
            .unwrap();
        assert_eq!(db.builtin_progressions().unwrap(), vec![changed]);
    }

    #[test]
    fn user_progressions_roundtrip_in_order_with_injection_text() {
        let (_dir, db) = db();
        let a = user(
            "Robert'); DROP TABLE base;--",
            "ii7 V7/ii ii7 V7 Imaj7(9):8",
        );
        let b = user("B \"aspas\"", "i V/7 bIIImaj7 IV7(#11)");
        let ws = HarmonyWorkspace::from_progressions(vec![a, b]).unwrap();
        db.save_user_progressions(&ws).unwrap();
        assert_eq!(db.load_user_progressions().unwrap(), ws);
        // regravar menos itens não deixa sobras
        let ws2 =
            HarmonyWorkspace::from_progressions(vec![ws.user_progressions()[1].clone()]).unwrap();
        db.save_user_progressions(&ws2).unwrap();
        assert_eq!(db.load_user_progressions().unwrap(), ws2);
        assert_eq!(
            count(&db, "SELECT count(*) FROM builtin_progression") as usize,
            builtin_catalog().len()
        );
    }

    #[test]
    fn bases_roundtrip_and_bundled_are_kept_apart() {
        let (_dir, db) = db();
        let cc0 = audio("Valsa CC0", "cc0/valsa.ogg", true);
        db.seed_bundled_bases(std::slice::from_ref(&cc0), 1)
            .unwrap();
        db.seed_bundled_bases(std::slice::from_ref(&cc0), 1)
            .unwrap();
        let mut evil = audio("x'); DELETE FROM base; --", "user/a b.wav", false);
        evil.tags.insert(Tag::new("o'reilly").unwrap());
        let g = generated("Bossa em Ré");
        let mut cat = db.load_bases().unwrap();
        assert_eq!(cat.bases(), std::slice::from_ref(&cc0));
        cat.insert(g.clone(), usize::MAX).unwrap();
        cat.insert(evil.clone(), usize::MAX).unwrap();
        db.save_bases(&cat).unwrap();
        let loaded = db.load_bases().unwrap();
        assert_eq!(loaded, cat);
        assert_eq!(loaded.get(evil.id).unwrap().name, evil.name);
        // salvar sem as do usuário não apaga a CC0
        db.save_bases(&BaseCatalog::new()).unwrap();
        assert_eq!(db.load_bases().unwrap().bases(), std::slice::from_ref(&cc0));
        // nova versão do manifesto troca as CC0 e mantém as do usuário
        db.save_bases(&cat).unwrap();
        db.seed_bundled_bases(&[], 2).unwrap();
        let after = db.load_bases().unwrap();
        assert!(after.get(cc0.id).is_none());
        assert!(after.get(g.id).is_some() && after.get(evil.id).is_some());
        // índice de BPM é usado pela busca por faixa
        let plan: String = db
            .connection()
            .query_row(
                "EXPLAIN QUERY PLAN SELECT id FROM base WHERE bpm BETWEEN ?1 AND ?2",
                params![80.0, 120.0],
                |r| r.get(3),
            )
            .unwrap();
        assert!(plan.contains("ix_base_bpm"), "{plan}");
    }

    #[test]
    fn starter_bases_seed_once_and_never_overwrite_user_choice() {
        let (_dir, db) = db();
        let s = generated("Inicial");
        db.seed_starter_bases(std::slice::from_ref(&s), 1).unwrap();
        db.seed_starter_bases(std::slice::from_ref(&s), 1).unwrap();
        assert_eq!(db.load_bases().unwrap().bases().len(), 1);
        // usuário remove; mesma versão não recoloca
        db.save_bases(&BaseCatalog::new()).unwrap();
        db.seed_starter_bases(std::slice::from_ref(&s), 1).unwrap();
        assert!(db.load_bases().unwrap().bases().is_empty());
        // manifesto com base que não é CC0 é rejeitado sem gravar nada
        assert!(db.seed_bundled_bases(std::slice::from_ref(&s), 5).is_err());
        assert!(db.load_bases().unwrap().bases().is_empty());
    }

    #[test]
    fn library_store_roundtrip() {
        let (_dir, db) = db();
        let lib = Library {
            harmony: HarmonyWorkspace::from_progressions(vec![user("Minha", "i iv V7")]).unwrap(),
            bases: BaseCatalog::from_bases(vec![generated("G")]).unwrap(),
        };
        LibraryStore::save(&db, &lib).unwrap();
        assert_eq!(LibraryStore::load(&db).unwrap(), lib);
    }
}
