-- project.db — versão 1 (Fases 0–2). Um banco por projeto: sem project_id nas tabelas filhas (FNBC).
PRAGMA foreign_keys = ON;

CREATE TABLE project (
    singleton   INTEGER PRIMARY KEY CHECK (singleton = 1),
    id          TEXT NOT NULL,
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 256),
    sample_rate INTEGER NOT NULL CHECK (sample_rate IN (44100, 48000, 88200, 96000)),
    bit_depth   INTEGER NOT NULL CHECK (bit_depth IN (16, 24, 32)),
    updated_at  TEXT NOT NULL
);

CREATE TABLE tempo_event (
    tick INTEGER PRIMARY KEY CHECK (tick >= 0),
    bpm  REAL NOT NULL CHECK (bpm BETWEEN 20 AND 999)
);

CREATE TABLE meter_event (
    tick        INTEGER PRIMARY KEY CHECK (tick >= 0),
    numerator   INTEGER NOT NULL CHECK (numerator BETWEEN 1 AND 32),
    denominator INTEGER NOT NULL CHECK (denominator IN (1, 2, 4, 8, 16, 32))
);

CREATE TABLE audio_source (
    id            TEXT PRIMARY KEY,
    content_hash  TEXT NOT NULL UNIQUE CHECK (length(content_hash) = 64),
    rel_path      TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL,
    channels      INTEGER NOT NULL CHECK (channels > 0),
    length_frames INTEGER NOT NULL CHECK (length_frames > 0)
);

CREATE TABLE track (
    id       TEXT PRIMARY KEY,
    name     TEXT NOT NULL,
    kind     TEXT NOT NULL CHECK (kind IN ('audio')),
    position INTEGER NOT NULL UNIQUE
);

CREATE TABLE clip (
    id            TEXT PRIMARY KEY,
    track_id      TEXT NOT NULL REFERENCES track(id) ON DELETE CASCADE,
    source_id     TEXT NOT NULL REFERENCES audio_source(id) ON DELETE RESTRICT,
    position      INTEGER NOT NULL,
    start_tick    INTEGER NOT NULL CHECK (start_tick >= 0),
    offset_frames INTEGER NOT NULL CHECK (offset_frames >= 0),
    length_frames INTEGER NOT NULL CHECK (length_frames > 0),
    gain_db       REAL NOT NULL,
    UNIQUE (track_id, position)
);
CREATE INDEX clip_by_track_start ON clip (track_id, start_tick);
CREATE INDEX clip_by_source ON clip (source_id);

-- Mixer -------------------------------------------------------------------
CREATE TABLE bus (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    position  INTEGER NOT NULL UNIQUE,
    volume_db REAL NOT NULL,
    pan       REAL NOT NULL CHECK (pan BETWEEN -1 AND 1),
    mute      INTEGER NOT NULL CHECK (mute IN (0, 1)),
    solo      INTEGER NOT NULL CHECK (solo IN (0, 1))
);

CREATE TABLE track_strip (
    track_id      TEXT PRIMARY KEY REFERENCES track(id) ON DELETE CASCADE,
    volume_db     REAL NOT NULL,
    pan           REAL NOT NULL CHECK (pan BETWEEN -1 AND 1),
    mute          INTEGER NOT NULL CHECK (mute IN (0, 1)),
    solo          INTEGER NOT NULL CHECK (solo IN (0, 1)),
    output_bus_id TEXT REFERENCES bus(id) ON DELETE SET NULL
);

CREATE TABLE master_strip (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    volume_db REAL NOT NULL,
    pan       REAL NOT NULL CHECK (pan BETWEEN -1 AND 1),
    mute      INTEGER NOT NULL CHECK (mute IN (0, 1))
);

CREATE TABLE aux_send (
    id        TEXT PRIMARY KEY,
    track_id  TEXT NOT NULL REFERENCES track_strip(track_id) ON DELETE CASCADE,
    bus_id    TEXT NOT NULL REFERENCES bus(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    level_db  REAL NOT NULL,
    pre_fader INTEGER NOT NULL CHECK (pre_fader IN (0, 1)),
    UNIQUE (track_id, bus_id),
    UNIQUE (track_id, position)
);

-- Efeito inserido: exatamente um dono (faixa, bus ou master).
CREATE TABLE effect_slot (
    id          TEXT PRIMARY KEY,
    track_id    TEXT REFERENCES track_strip(track_id) ON DELETE CASCADE,
    bus_id      TEXT REFERENCES bus(id) ON DELETE CASCADE,
    on_master   INTEGER NOT NULL DEFAULT 0 CHECK (on_master IN (0, 1)),
    position    INTEGER NOT NULL,
    effect_type TEXT NOT NULL,
    bypass      INTEGER NOT NULL CHECK (bypass IN (0, 1)),
    CHECK ((track_id IS NOT NULL) + (bus_id IS NOT NULL) + on_master = 1)
);
CREATE INDEX effect_by_track ON effect_slot (track_id, position);
CREATE INDEX effect_by_bus ON effect_slot (bus_id, position);

-- Parâmetros escalares (1 linha por parâmetro: 1FN, sem JSON).
CREATE TABLE effect_param (
    effect_id TEXT NOT NULL REFERENCES effect_slot(id) ON DELETE CASCADE,
    name      TEXT NOT NULL,
    kind      TEXT NOT NULL CHECK (kind IN ('num', 'bool')),
    value     REAL NOT NULL,
    PRIMARY KEY (effect_id, name)
);

CREATE TABLE eq_band (
    effect_id TEXT NOT NULL REFERENCES effect_slot(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    kind      TEXT NOT NULL,
    enabled   INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    freq_hz   REAL NOT NULL,
    gain_db   REAL NOT NULL,
    q         REAL NOT NULL,
    PRIMARY KEY (effect_id, position)
);

-- Gravação ----------------------------------------------------------------
CREATE TABLE recording_settings (
    singleton         INTEGER PRIMARY KEY CHECK (singleton = 1),
    metro_enabled     INTEGER NOT NULL,
    metro_volume      REAL NOT NULL,
    metro_subdivision INTEGER NOT NULL,
    metro_accent      INTEGER NOT NULL,
    count_in_bars     INTEGER NOT NULL,
    punch_start       INTEGER,
    punch_end         INTEGER,
    loop_start        INTEGER,
    loop_end          INTEGER,
    latency_frames    INTEGER NOT NULL,
    CHECK ((punch_start IS NULL) = (punch_end IS NULL)),
    CHECK ((loop_start IS NULL) = (loop_end IS NULL))
);

CREATE TABLE track_recording (
    track_id       TEXT PRIMARY KEY REFERENCES track(id) ON DELETE CASCADE,
    armed          INTEGER NOT NULL,
    input_left     INTEGER,
    input_right    INTEGER,
    monitor        TEXT NOT NULL CHECK (monitor IN ('off', 'whenArmed', 'always')),
    active_take_id TEXT
);

CREATE TABLE take (
    id            TEXT PRIMARY KEY,
    track_id      TEXT NOT NULL REFERENCES track_recording(track_id) ON DELETE CASCADE,
    source_id     TEXT NOT NULL REFERENCES audio_source(id) ON DELETE RESTRICT,
    position      INTEGER NOT NULL,
    start_tick    INTEGER NOT NULL,
    length_frames INTEGER NOT NULL CHECK (length_frames > 0),
    number        INTEGER NOT NULL,
    UNIQUE (track_id, number)
);

-- Leitura eficiente: clipes ordenados por faixa (usado pela UI e diagnóstico).
CREATE VIEW v_track_clips AS
SELECT t.position AS track_position, t.name AS track_name, c.*
FROM clip c JOIN track t ON t.id = c.track_id
ORDER BY t.position, c.start_tick;
