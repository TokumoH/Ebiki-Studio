//! Repositório da sessão em `project.db`.

use crate::effects::{load_slots, save_slots, Owner};
use crate::{open_project_db, PersistenceError};
use app_core::ports::{PortError, SessionRepository};
use app_core::Session;
use ctx_mixing::{AuxSend, Bus, ChannelStrip, Mixer, Output};
use ctx_project::{AudioSource, Clip, Project, RelPath, Track, TrackKind};
use ctx_recording::{
    InputAssignment, MetronomeConfig, MonitorMode, RecordingSetup, Take, TrackRecording,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use shared_kernel::tempo::{MeterEvent, TempoEvent};
use shared_kernel::{AudioFormat, BitDepth, Db, Pan, SampleRate, TempoMap, Tick};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteSessionRepository;

fn corrupt(what: &str) -> PersistenceError {
    PersistenceError::Corrupt(what.to_owned())
}

fn parse_id<T: std::str::FromStr>(s: &str) -> Result<T, PersistenceError> {
    s.parse().map_err(|_| corrupt("identificador inválido"))
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

impl SqliteSessionRepository {
    pub fn save_to(&self, path: &Path, s: &Session) -> Result<(), PersistenceError> {
        let mut conn = open_project_db(path)?;
        let tx = conn.transaction()?;
        for t in [
            "take",
            "track_recording",
            "recording_settings",
            "eq_band",
            "effect_param",
            "effect_slot",
            "aux_send",
            "master_strip",
            "track_strip",
            "bus",
            "clip",
            "track",
            "audio_source",
            "meter_event",
            "tempo_event",
            "project",
        ] {
            // nomes de tabela são constantes do código, nunca entrada do usuário
            tx.execute(&format!("DELETE FROM {t}"), [])?;
        }
        write_project(&tx, &s.project)?;
        write_mixer(&tx, &s.mixer)?;
        write_recording(&tx, &s.recording)?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_from(&self, path: &Path) -> Result<Session, PersistenceError> {
        if !path.exists() {
            return Err(corrupt("arquivo do projeto não encontrado"));
        }
        let conn = open_project_db(path)?;
        let project = read_project(&conn)?;
        let mixer = read_mixer(&conn)?;
        let recording = read_recording(&conn)?;
        Ok(Session {
            project,
            mixer,
            recording,
        })
    }
}

impl SessionRepository for SqliteSessionRepository {
    fn save(&self, db_path: &Path, session: &Session) -> Result<(), PortError> {
        Ok(self.save_to(db_path, session)?)
    }
    fn load(&self, db_path: &Path) -> Result<Session, PortError> {
        Ok(self.load_from(db_path)?)
    }
}

fn write_project(tx: &Transaction, p: &Project) -> Result<(), PersistenceError> {
    tx.execute(
        "INSERT INTO project (singleton, id, name, sample_rate, bit_depth, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
        params![p.id.to_string(), p.name(), p.format.sample_rate.hz(), p.format.bit_depth.bits(), now_iso()],
    )?;
    for e in p.tempo().tempos() {
        tx.execute(
            "INSERT INTO tempo_event (tick, bpm) VALUES (?1, ?2)",
            params![e.tick.0, e.bpm],
        )?;
    }
    for e in p.tempo().meters() {
        tx.execute(
            "INSERT INTO meter_event (tick, numerator, denominator) VALUES (?1, ?2, ?3)",
            params![e.tick.0, e.numerator, e.denominator],
        )?;
    }
    for s in p.sources() {
        tx.execute(
            "INSERT INTO audio_source (id, content_hash, rel_path, original_name, channels, length_frames) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![s.id.to_string(), s.content_hash, s.rel_path.as_str(), s.original_name, s.channels, s.length_frames as i64],
        )?;
    }
    for (pos, t) in p.tracks().iter().enumerate() {
        tx.execute(
            "INSERT INTO track (id, name, kind, position) VALUES (?1, ?2, 'audio', ?3)",
            params![t.id.to_string(), t.name, pos as i64],
        )?;
        for (cpos, c) in t.clips.iter().enumerate() {
            tx.execute(
                "INSERT INTO clip (id, track_id, source_id, position, start_tick, offset_frames, length_frames, gain_db) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    c.id.to_string(),
                    t.id.to_string(),
                    c.source_id.to_string(),
                    cpos as i64,
                    c.start.0,
                    c.offset_frames as i64,
                    c.length_frames as i64,
                    c.gain.0 as f64
                ],
            )?;
        }
    }
    Ok(())
}

fn read_project(conn: &Connection) -> Result<Project, PersistenceError> {
    let (id, name, sr, bits): (String, String, u32, u16) = conn
        .query_row(
            "SELECT id, name, sample_rate, bit_depth FROM project WHERE singleton = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| corrupt("projeto vazio"))?;
    let format = AudioFormat {
        sample_rate: SampleRate::new(sr).map_err(|e| PersistenceError::Corrupt(e.to_string()))?,
        bit_depth: BitDepth::new(bits).map_err(|e| PersistenceError::Corrupt(e.to_string()))?,
    };
    let tempos: Vec<TempoEvent> = conn
        .prepare("SELECT tick, bpm FROM tempo_event ORDER BY tick")?
        .query_map([], |r| {
            Ok(TempoEvent {
                tick: Tick(r.get(0)?),
                bpm: r.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let meters: Vec<MeterEvent> = conn
        .prepare("SELECT tick, numerator, denominator FROM meter_event ORDER BY tick")?
        .query_map([], |r| {
            Ok(MeterEvent {
                tick: Tick(r.get(0)?),
                numerator: r.get(1)?,
                denominator: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let tempo = TempoMap::from_events(tempos, meters)
        .map_err(|e| PersistenceError::Corrupt(e.to_string()))?;
    let mut sources = Vec::new();
    let mut st = conn.prepare("SELECT id, content_hash, rel_path, original_name, channels, length_frames FROM audio_source ORDER BY rowid")?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, u16>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })?;
    for row in rows {
        let (id, hash, rel, orig, ch, len) = row?;
        sources.push(AudioSource {
            id: parse_id(&id)?,
            content_hash: hash,
            rel_path: RelPath::new(rel).map_err(|e| PersistenceError::Corrupt(e.to_string()))?,
            original_name: orig,
            channels: ch,
            length_frames: u64::try_from(len).map_err(|_| corrupt("duração negativa"))?,
        });
    }
    let mut tracks = Vec::new();
    let mut ts = conn.prepare("SELECT id, name FROM track ORDER BY position")?;
    let mut cs = conn.prepare(
        "SELECT id, source_id, start_tick, offset_frames, length_frames, gain_db FROM clip WHERE track_id = ?1 ORDER BY position",
    )?;
    let trows: Vec<(String, String)> = ts
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (tid, tname) in trows {
        let clips: Vec<Clip> = cs
            .query_map(params![tid], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, f64>(5)?,
                ))
            })?
            .map(|row| {
                let (id, src, start, off, len, gain) = row?;
                Ok(Clip {
                    id: parse_id(&id)?,
                    source_id: parse_id(&src)?,
                    start: Tick(start),
                    offset_frames: u64::try_from(off).map_err(|_| corrupt("offset negativo"))?,
                    length_frames: u64::try_from(len).map_err(|_| corrupt("duração negativa"))?,
                    gain: Db(gain as f32),
                })
            })
            .collect::<Result<_, PersistenceError>>()?;
        tracks.push(Track {
            id: parse_id(&tid)?,
            name: tname,
            kind: TrackKind::Audio,
            clips,
        });
    }
    Project::restore(parse_id(&id)?, name, format, tempo, tracks, sources)
        .map_err(|e| PersistenceError::Corrupt(e.to_string()))
}

fn write_mixer(tx: &Transaction, m: &Mixer) -> Result<(), PersistenceError> {
    for (pos, b) in m.buses().iter().enumerate() {
        tx.execute(
            "INSERT INTO bus (id, name, position, volume_db, pan, mute, solo) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![b.id.to_string(), b.name, pos as i64, b.strip.volume.0 as f64, b.strip.pan.0 as f64, b.strip.mute, b.strip.solo],
        )?;
        let bid = b.id.to_string();
        save_slots(tx, Owner::Bus(&bid), &b.strip.inserts)?;
    }
    for (tid, s) in m.strips() {
        let tid = tid.to_string();
        let out = match s.output {
            Output::Master => None,
            Output::Bus(b) => Some(b.to_string()),
        };
        tx.execute(
            "INSERT INTO track_strip (track_id, volume_db, pan, mute, solo, output_bus_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![tid, s.volume.0 as f64, s.pan.0 as f64, s.mute, s.solo, out],
        )?;
        for (pos, snd) in s.sends.iter().enumerate() {
            tx.execute(
                "INSERT INTO aux_send (id, track_id, bus_id, position, level_db, pre_fader) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![snd.id.to_string(), tid, snd.bus.to_string(), pos as i64, snd.level.0 as f64, snd.pre_fader],
            )?;
        }
        save_slots(tx, Owner::Track(&tid), &s.inserts)?;
    }
    let ms = m.master();
    tx.execute(
        "INSERT INTO master_strip (singleton, volume_db, pan, mute) VALUES (1, ?1, ?2, ?3)",
        params![ms.volume.0 as f64, ms.pan.0 as f64, ms.mute],
    )?;
    save_slots(tx, Owner::Master, &ms.inserts)?;
    Ok(())
}

fn read_mixer(conn: &Connection) -> Result<Mixer, PersistenceError> {
    let mut buses = Vec::new();
    let mut bs =
        conn.prepare("SELECT id, name, volume_db, pan, mute, solo FROM bus ORDER BY position")?;
    let brows: Vec<(String, String, f64, f64, bool, bool)> = bs
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
    for (id, name, vol, pan, mute, solo) in brows {
        let inserts = load_slots(conn, Owner::Bus(&id))?;
        buses.push(Bus {
            id: parse_id(&id)?,
            name,
            strip: ChannelStrip {
                volume: Db(vol as f32),
                pan: Pan(pan as f32),
                mute,
                solo,
                inserts,
                ..Default::default()
            },
        });
    }
    let mut strips = BTreeMap::new();
    let mut ss = conn
        .prepare("SELECT track_id, volume_db, pan, mute, solo, output_bus_id FROM track_strip")?;
    let srows: Vec<(String, f64, f64, bool, bool, Option<String>)> = ss
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
    let mut sends_stmt = conn.prepare("SELECT id, bus_id, level_db, pre_fader FROM aux_send WHERE track_id = ?1 ORDER BY position")?;
    for (tid, vol, pan, mute, solo, out) in srows {
        let sends: Vec<AuxSend> = sends_stmt
            .query_map(params![tid], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, bool>(3)?,
                ))
            })?
            .map(|row| {
                let (id, bus, lvl, pre) = row?;
                Ok(AuxSend {
                    id: parse_id(&id)?,
                    bus: parse_id(&bus)?,
                    level: Db(lvl as f32),
                    pre_fader: pre,
                })
            })
            .collect::<Result<_, PersistenceError>>()?;
        let output = match out {
            Some(b) => Output::Bus(parse_id(&b)?),
            None => Output::Master,
        };
        let inserts = load_slots(conn, Owner::Track(&tid))?;
        strips.insert(
            parse_id(&tid)?,
            ChannelStrip {
                volume: Db(vol as f32),
                pan: Pan(pan as f32),
                mute,
                solo,
                inserts,
                sends,
                output,
            },
        );
    }
    let master = conn
        .query_row(
            "SELECT volume_db, pan, mute FROM master_strip WHERE singleton = 1",
            [],
            |r| {
                Ok((
                    r.get::<_, f64>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()?
        .map(|(v, p, m)| ChannelStrip {
            volume: Db(v as f32),
            pan: Pan(p as f32),
            mute: m,
            ..Default::default()
        })
        .unwrap_or_default();
    let master = ChannelStrip {
        inserts: load_slots(conn, Owner::Master)?,
        ..master
    };
    Mixer::restore(strips, buses, master).map_err(|e| PersistenceError::Corrupt(e.to_string()))
}

fn monitor_str(m: MonitorMode) -> &'static str {
    match m {
        MonitorMode::Off => "off",
        MonitorMode::WhenArmed => "whenArmed",
        MonitorMode::Always => "always",
    }
}

fn write_recording(tx: &Transaction, r: &RecordingSetup) -> Result<(), PersistenceError> {
    let m = r.metronome;
    let (ps, pe) = r
        .punch()
        .map_or((None, None), |(a, b)| (Some(a.0), Some(b.0)));
    let (ls, le) = r
        .loop_range()
        .map_or((None, None), |(a, b)| (Some(a.0), Some(b.0)));
    tx.execute(
        "INSERT INTO recording_settings (singleton, metro_enabled, metro_volume, metro_subdivision, metro_accent, count_in_bars, punch_start, punch_end, loop_start, loop_end, latency_frames) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![m.enabled, m.volume as f64, m.subdivision, m.accent_first_beat, m.count_in_bars, ps, pe, ls, le, r.latency_compensation_frames],
    )?;
    for (tid, t) in r.tracks() {
        let tid = tid.to_string();
        tx.execute(
            "INSERT INTO track_recording (track_id, armed, input_left, input_right, monitor, active_take_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![tid, t.armed, t.input.map(|i| i.left), t.input.and_then(|i| i.right), monitor_str(t.monitor), t.active_take.map(|x| x.to_string())],
        )?;
        for (pos, k) in t.takes.iter().enumerate() {
            tx.execute(
                "INSERT INTO take (id, track_id, source_id, position, start_tick, length_frames, number) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![k.id.to_string(), tid, k.source_id.to_string(), pos as i64, k.start.0, k.length_frames as i64, k.number],
            )?;
        }
    }
    Ok(())
}

fn read_recording(conn: &Connection) -> Result<RecordingSetup, PersistenceError> {
    type Row = (
        bool,
        f64,
        u8,
        bool,
        u8,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        u32,
    );
    let row: Option<Row> = conn
        .query_row(
            "SELECT metro_enabled, metro_volume, metro_subdivision, metro_accent, count_in_bars, punch_start, punch_end, loop_start, loop_end, latency_frames FROM recording_settings WHERE singleton = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
        )
        .optional()?;
    let Some((en, vol, sub, acc, ci, ps, pe, ls, le, lat)) = row else {
        return Ok(RecordingSetup::default());
    };
    let metronome = MetronomeConfig {
        enabled: en,
        volume: vol as f32,
        subdivision: sub,
        accent_first_beat: acc,
        count_in_bars: ci,
    };
    let pair = |a: Option<i64>, b: Option<i64>| a.zip(b).map(|(a, b)| (Tick(a), Tick(b)));
    let mut tracks = BTreeMap::new();
    let mut ts = conn.prepare("SELECT track_id, armed, input_left, input_right, monitor, active_take_id FROM track_recording")?;
    let mut ks = conn.prepare("SELECT id, source_id, start_tick, length_frames, number FROM take WHERE track_id = ?1 ORDER BY position")?;
    type TrackRow = (
        String,
        bool,
        Option<u16>,
        Option<u16>,
        String,
        Option<String>,
    );
    let trows: Vec<TrackRow> = ts
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
    for (tid, armed, l, rr, mon, active) in trows {
        let takes: Vec<Take> = ks
            .query_map(params![tid], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, u32>(4)?,
                ))
            })?
            .map(|row| {
                let (id, src, start, len, number) = row?;
                Ok(Take {
                    id: parse_id(&id)?,
                    source_id: parse_id(&src)?,
                    start: Tick(start),
                    length_frames: len.max(0) as u64,
                    number,
                })
            })
            .collect::<Result<_, PersistenceError>>()?;
        let monitor = match mon.as_str() {
            "whenArmed" => MonitorMode::WhenArmed,
            "always" => MonitorMode::Always,
            _ => MonitorMode::Off,
        };
        tracks.insert(
            parse_id(&tid)?,
            TrackRecording {
                armed,
                input: l.map(|left| InputAssignment { left, right: rr }),
                monitor,
                takes,
                active_take: active.map(|a| parse_id(&a)).transpose()?,
            },
        );
    }
    RecordingSetup::restore(metronome, pair(ps, pe), pair(ls, le), lat, tracks)
        .map_err(|e| PersistenceError::Corrupt(e.to_string()))
}
