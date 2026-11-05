//! Mapeamento dos agregados das Fases 6–8 (letra, afinação, partituras) para o `project.db` v2.
//!
//! - Letra e afinação: totalmente normalizadas (linha/palavra/sílaba; mapa/escala/nota/edição).
//! - Partituras: cabeçalho normalizado (`score`, `score_part`) + corpo do agregado em documento JSON
//!   versionado sem os campos do cabeçalho (ADR-0027).

use crate::PersistenceError;
use ctx_notation::{Score, ScoreBook, ScoreEntry};
use ctx_sync::{LyricLine, LyricTrack, LyricWord, Span, Syllable};
use ctx_tuning::{
    DetectedNote, NoteEdit, PitchCorrectionMap, ScaleCorrection, TunerSettings, TuningBook,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{Map, Value};
use shared_kernel::id::AudioSourceId;
use shared_kernel::music::{MidiNote, PitchClass};
use shared_kernel::Tick;
use std::collections::{BTreeMap, HashSet};

/// Versão atual do documento do corpo da partitura.
pub const SCORE_DOC_VERSION: i64 = 1;

/// Tabelas desta camada, em ordem segura de exclusão (filhas primeiro).
pub const TABLES: &[&str] = &[
    "lyric_syllable",
    "lyric_word",
    "lyric_line",
    "lyric_track",
    "note_edit",
    "detected_note",
    "correction_scale_pitch",
    "pitch_correction_map",
    "tuner_settings",
    "score_part",
    "score",
];

fn corrupt(what: impl Into<String>) -> PersistenceError {
    PersistenceError::Corrupt(what.into())
}

fn parse_id<T: std::str::FromStr>(s: &str) -> Result<T, PersistenceError> {
    s.parse().map_err(|_| corrupt("identificador inválido"))
}

fn idx(v: i64) -> Result<usize, PersistenceError> {
    usize::try_from(v).map_err(|_| corrupt("posição negativa"))
}

// ------------------------------------------------------------------ letra

pub fn write_lyrics(tx: &Transaction, t: &LyricTrack) -> Result<(), PersistenceError> {
    tx.execute(
        "INSERT INTO lyric_track (singleton, id) VALUES (1, ?1)",
        params![t.id.to_string()],
    )?;
    let mut line_st = tx.prepare_cached("INSERT INTO lyric_line (position) VALUES (?1)")?;
    let mut word_st =
        tx.prepare_cached("INSERT INTO lyric_word (line, position) VALUES (?1, ?2)")?;
    let mut syl_st = tx.prepare_cached(
        "INSERT INTO lyric_syllable (line, word, position, text, start_tick, end_tick) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (li, line) in t.lines().iter().enumerate() {
        line_st.execute(params![li as i64])?;
        for (wi, word) in line.words.iter().enumerate() {
            word_st.execute(params![li as i64, wi as i64])?;
            for (si, s) in word.syllables.iter().enumerate() {
                syl_st.execute(params![
                    li as i64,
                    wi as i64,
                    si as i64,
                    s.text,
                    s.span.map(|sp| sp.start.0),
                    s.span.map(|sp| sp.end.0)
                ])?;
            }
        }
    }
    Ok(())
}

pub fn read_lyrics(conn: &Connection) -> Result<Option<LyricTrack>, PersistenceError> {
    let Some(id): Option<String> = conn
        .query_row("SELECT id FROM lyric_track WHERE singleton = 1", [], |r| {
            r.get(0)
        })
        .optional()?
    else {
        return Ok(None);
    };
    type Row = (i64, i64, i64, String, Option<i64>, Option<i64>);
    let rows: Vec<Row> = conn
        .prepare(
            "SELECT line, word, position, text, start_tick, end_tick FROM lyric_syllable ORDER BY line, word, position",
        )?
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut lines: Vec<LyricLine> = Vec::new();
    for (l, w, s, text, start, end) in rows {
        let (l, w, s) = (idx(l)?, idx(w)?, idx(s)?);
        // posições densas: cada nova linha/palavra/sílaba é exatamente a próxima
        if l == lines.len() {
            lines.push(LyricLine { words: Vec::new() });
        } else if l + 1 != lines.len() {
            return Err(corrupt("linhas da letra fora de sequência"));
        }
        let line = lines.last_mut().expect("linha recém-criada");
        if w == line.words.len() {
            line.words.push(LyricWord {
                syllables: Vec::new(),
            });
        } else if w + 1 != line.words.len() {
            return Err(corrupt("palavras da letra fora de sequência"));
        }
        let word = line.words.last_mut().expect("palavra recém-criada");
        if s != word.syllables.len() {
            return Err(corrupt("sílabas da letra fora de sequência"));
        }
        let span = match (start, end) {
            (Some(a), Some(b)) => {
                Some(Span::new(Tick(a), Tick(b)).map_err(|e| corrupt(e.to_string()))?)
            }
            (None, None) => None,
            _ => return Err(corrupt("tempo de sílaba incompleto")),
        };
        word.syllables.push(Syllable { text, span });
    }
    let mut track = LyricTrack::from_lines(lines).map_err(|e| corrupt(e.to_string()))?;
    track.id = parse_id(&id)?;
    Ok(Some(track))
}

// --------------------------------------------------------------- afinação

pub fn write_tuning(
    tx: &Transaction,
    t: &TuningBook,
    known_sources: &HashSet<AudioSourceId>,
) -> Result<(), PersistenceError> {
    tx.execute(
        "INSERT INTO tuner_settings (singleton, a4_hz) VALUES (1, ?1)",
        params![t.tuner.a4_hz()],
    )?;
    let mut note_st = tx.prepare_cached(
        "INSERT INTO detected_note (id, map_id, start_frame, end_frame, midi) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut edit_st = tx.prepare_cached(
        "INSERT INTO note_edit (note_id, target_midi, cents_offset, flatten, formant_semitones) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (source, m) in t.maps() {
        // mapa de fonte que não existe mais no projeto (inalcançável): não é persistido
        if !known_sources.contains(source) {
            continue;
        }
        let map_id = m.id.to_string();
        let sc = m.scale();
        tx.execute(
            "INSERT INTO pitch_correction_map (id, source_id, scale_enabled, scale_key, retune_speed_ms, humanize) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![map_id, source.to_string(), sc.enabled, sc.key.value(), sc.retune_speed_ms, sc.humanize],
        )?;
        for pc in &sc.allowed {
            tx.execute(
                "INSERT INTO correction_scale_pitch (map_id, pitch_class) VALUES (?1, ?2)",
                params![map_id, pc.value()],
            )?;
        }
        for n in m.notes() {
            note_st.execute(params![
                n.id.to_string(),
                map_id,
                n.start_frame as i64,
                n.end_frame as i64,
                n.midi
            ])?;
        }
        for (note, e) in m.edits() {
            edit_st.execute(params![
                note.to_string(),
                e.target.map(|t| t.0),
                e.cents_offset,
                e.flatten,
                e.formant_semitones
            ])?;
        }
    }
    Ok(())
}

pub fn read_tuning(conn: &Connection) -> Result<TuningBook, PersistenceError> {
    let a4: Option<f64> = conn
        .query_row(
            "SELECT a4_hz FROM tuner_settings WHERE singleton = 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let tuner = match a4 {
        Some(hz) => TunerSettings::new(hz).map_err(|e| corrupt(e.to_string()))?,
        None => TunerSettings::default(),
    };
    type MapRow = (String, String, bool, u8, f64, f64);
    let map_rows: Vec<MapRow> = conn
        .prepare(
            "SELECT id, source_id, scale_enabled, scale_key, retune_speed_ms, humanize FROM pitch_correction_map ORDER BY source_id",
        )?
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut pcs_st = conn.prepare_cached(
        "SELECT pitch_class FROM correction_scale_pitch WHERE map_id = ?1 ORDER BY pitch_class",
    )?;
    let mut notes_st = conn.prepare_cached(
        "SELECT id, start_frame, end_frame, midi FROM detected_note WHERE map_id = ?1 ORDER BY start_frame",
    )?;
    let mut edits_st = conn.prepare_cached(
        "SELECT e.note_id, e.target_midi, e.cents_offset, e.flatten, e.formant_semitones FROM note_edit e JOIN detected_note n ON n.id = e.note_id WHERE n.map_id = ?1",
    )?;
    let mut maps = Vec::with_capacity(map_rows.len());
    for (id, source, enabled, key, retune, humanize) in map_rows {
        let pc = |v: u8| PitchClass::new(v).ok_or_else(|| corrupt("classe de altura inválida"));
        let allowed: Vec<PitchClass> = pcs_st
            .query_map(params![id], |r| r.get::<_, u8>(0))?
            .map(|v| pc(v?))
            .collect::<Result<_, PersistenceError>>()?;
        let scale = ScaleCorrection {
            enabled,
            key: pc(key)?,
            allowed,
            retune_speed_ms: retune,
            humanize,
        };
        let notes: Vec<DetectedNote> = notes_st
            .query_map(params![id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, f64>(3)?,
                ))
            })?
            .map(|row| {
                let (nid, a, b, midi) = row?;
                Ok(DetectedNote {
                    id: parse_id(&nid)?,
                    start_frame: u64::try_from(a).map_err(|_| corrupt("quadro negativo"))?,
                    end_frame: u64::try_from(b).map_err(|_| corrupt("quadro negativo"))?,
                    midi,
                })
            })
            .collect::<Result<_, PersistenceError>>()?;
        let mut edits = BTreeMap::new();
        for row in edits_st.query_map(params![id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<u8>>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, f64>(3)?,
                r.get::<_, f64>(4)?,
            ))
        })? {
            let (nid, target, cents, flatten, formant) = row?;
            edits.insert(
                parse_id(&nid)?,
                NoteEdit {
                    target: target.map(MidiNote),
                    cents_offset: cents,
                    flatten,
                    formant_semitones: formant,
                },
            );
        }
        let map = PitchCorrectionMap::from_parts(
            parse_id(&id)?,
            parse_id::<AudioSourceId>(&source)?,
            notes,
            edits,
            scale,
        )
        .map_err(|e| corrupt(format!("mapa de afinação: {e}")))?;
        maps.push(map);
    }
    Ok(TuningBook::from_parts(maps, tuner))
}

// ------------------------------------------------------------- partituras

/// Campos do cabeçalho (normalizados em colunas) retirados do documento.
const SCORE_HEADER: &[&str] = &["title", "composer"];
const PART_HEADER: &[&str] = &["name", "program", "percussion"];

/// Corpo do documento: o agregado serializado sem os campos do cabeçalho.
pub fn score_body(score: &Score) -> Result<String, PersistenceError> {
    let mut v = serde_json::to_value(score).map_err(|e| corrupt(e.to_string()))?;
    let obj = v
        .as_object_mut()
        .ok_or_else(|| corrupt("partitura não é um objeto"))?;
    for k in SCORE_HEADER {
        obj.remove(*k);
    }
    if let Some(Value::Array(parts)) = obj.get_mut("parts") {
        for p in parts {
            if let Some(po) = p.as_object_mut() {
                for k in PART_HEADER {
                    po.remove(*k);
                }
            }
        }
    }
    serde_json::to_string(&v).map_err(|e| corrupt(e.to_string()))
}

/// Partes (cabeçalho) lidas das colunas.
pub struct PartHeader {
    pub name: String,
    pub program: u8,
    pub percussion: bool,
}

/// Reconstrói a partitura a partir das colunas + documento (validando invariantes).
pub fn score_from_parts(
    title: String,
    composer: String,
    parts: Vec<PartHeader>,
    doc_version: i64,
    body: &str,
) -> Result<Score, PersistenceError> {
    if doc_version != SCORE_DOC_VERSION {
        return Err(corrupt(format!(
            "versão do documento de partitura não suportada: {doc_version}"
        )));
    }
    let mut v: Value =
        serde_json::from_str(body).map_err(|e| corrupt(format!("partitura: {e}")))?;
    let obj = v
        .as_object_mut()
        .ok_or_else(|| corrupt("partitura não é um objeto"))?;
    obj.insert("title".into(), Value::String(title));
    obj.insert("composer".into(), Value::String(composer));
    let body_parts = match obj.get_mut("parts") {
        Some(Value::Array(a)) => a,
        _ => return Err(corrupt("partitura sem partes")),
    };
    if body_parts.len() != parts.len() {
        return Err(corrupt("partes da partitura divergem do cabeçalho"));
    }
    for (p, h) in body_parts.iter_mut().zip(parts) {
        let po: &mut Map<String, Value> = p
            .as_object_mut()
            .ok_or_else(|| corrupt("parte não é um objeto"))?;
        po.insert("name".into(), Value::String(h.name));
        po.insert("program".into(), Value::from(h.program));
        po.insert("percussion".into(), Value::Bool(h.percussion));
    }
    let score: Score = serde_json::from_value(v).map_err(|e| corrupt(format!("partitura: {e}")))?;
    score
        .check_invariants()
        .map_err(|e| corrupt(format!("partitura inválida: {e}")))?;
    Ok(score)
}

pub fn write_scores(tx: &Transaction, book: &ScoreBook) -> Result<(), PersistenceError> {
    let mut part_st = tx.prepare_cached(
        "INSERT INTO score_part (score_id, position, name, program, percussion) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (pos, e) in book.entries().iter().enumerate() {
        let id = e.id.to_string();
        tx.execute(
            "INSERT INTO score (id, position, title, composer, doc_version, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, pos as i64, e.score.title, e.score.composer, SCORE_DOC_VERSION, score_body(&e.score)?],
        )?;
        for (pp, p) in e.score.parts.iter().enumerate() {
            part_st.execute(params![id, pp as i64, p.name, p.program, p.percussion])?;
        }
    }
    Ok(())
}

pub fn read_scores(conn: &Connection) -> Result<ScoreBook, PersistenceError> {
    type Row = (String, String, String, i64, String);
    let rows: Vec<Row> = conn
        .prepare("SELECT id, title, composer, doc_version, body FROM score ORDER BY position")?
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut parts_st = conn.prepare_cached(
        "SELECT name, program, percussion FROM score_part WHERE score_id = ?1 ORDER BY position",
    )?;
    let mut entries = Vec::with_capacity(rows.len());
    for (id, title, composer, version, body) in rows {
        let parts: Vec<PartHeader> = parts_st
            .query_map(params![id], |r| {
                Ok(PartHeader {
                    name: r.get(0)?,
                    program: r.get(1)?,
                    percussion: r.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        entries.push(ScoreEntry {
            id: parse_id(&id)?,
            score: score_from_parts(title, composer, parts, version, &body)?,
        });
    }
    ScoreBook::from_entries(entries).map_err(|e| corrupt(e.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::{migrate, open, SqliteSessionRepository, PROJECT_MIGRATIONS};
    use ctx_mixing::{Effect, EffectSlot, PitchCorrectorParams, StripEdit, StripTarget};

    #[test]
    fn v1_database_is_migrated_to_v2_and_loads_with_empty_music_aggregates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.db");
        {
            // banco criado por uma versão que só conhecia o schema 1
            let mut c = open(&path).unwrap();
            assert_eq!(migrate(&mut c, &PROJECT_MIGRATIONS[..1]).unwrap(), 1);
            c.execute_batch(
                "INSERT INTO project (singleton, id, name, sample_rate, bit_depth, updated_at)
                   VALUES (1, '0190a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b', 'Antigo', 48000, 24, '0');
                 INSERT INTO tempo_event (tick, bpm) VALUES (0, 100.0);
                 INSERT INTO meter_event (tick, numerator, denominator) VALUES (0, 3, 4);
                 INSERT INTO master_strip (singleton, volume_db, pan, mute) VALUES (1, -2.0, 0.0, 0);",
            )
            .unwrap();
        }
        let s = SqliteSessionRepository.load_from(&path).unwrap();
        assert_eq!(s.project.name(), "Antigo");
        assert!((s.project.tempo().bpm_at(shared_kernel::Tick(0)) - 100.0).abs() < 1e-9);
        assert!(s.lyrics.lines().is_empty());
        assert!(s.scores.is_empty());
        assert!(s.tuning.maps().is_empty());
        assert!((s.tuning.tuner.a4_hz() - 440.0).abs() < 1e-9);
        let c = open(&path).unwrap();
        let v: u32 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, 2, "abrir migra para a versão atual");
        // e o projeto migrado salva/carrega normalmente
        SqliteSessionRepository.save_to(&path, &s).unwrap();
        assert_eq!(SqliteSessionRepository.load_from(&path).unwrap(), s);
    }

    #[test]
    fn pitch_corrector_effect_roundtrips_with_integer_params() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.db");
        let p = ctx_project::Project::new("Fx", shared_kernel::AudioFormat::default()).unwrap();
        let mut s = app_core::Session::new(p);
        let slot = EffectSlot::new(Effect::PitchCorrector(PitchCorrectorParams {
            humanize: 0.25,
            retune_ms: 42.5,
            ..PitchCorrectorParams::major(7)
        }));
        s.mixer
            .edit(
                StripTarget::Master,
                &StripEdit::AddEffect { slot, index: None },
            )
            .unwrap();
        SqliteSessionRepository.save_to(&path, &s).unwrap();
        let loaded = SqliteSessionRepository.load_from(&path).unwrap();
        assert_eq!(loaded.mixer, s.mixer);
    }
}
