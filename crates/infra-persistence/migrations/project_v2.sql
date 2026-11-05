-- project.db — versão 2 (Fases 6, 7 e 8): letra sincronizada, afinação e partituras.
-- Um banco por projeto: tabelas singleton com CHECK (singleton = 1), filhas sem project_id (FNBC).
PRAGMA foreign_keys = ON;

-- Letra (Fase 6) ------------------------------------------------------------
-- Identidade da trilha de letra (uma por projeto).
CREATE TABLE lyric_track (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    id        TEXT NOT NULL
);

-- Linha → palavra → sílaba. O texto de palavra/linha é derivado das sílabas (não é guardado);
-- os tempos ficam só na sílaba (ADR-0017). Posições densas a partir de 0.
CREATE TABLE lyric_line (
    position INTEGER PRIMARY KEY CHECK (position >= 0)
);

CREATE TABLE lyric_word (
    line     INTEGER NOT NULL REFERENCES lyric_line(position) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    PRIMARY KEY (line, position)
);

CREATE TABLE lyric_syllable (
    line       INTEGER NOT NULL,
    word       INTEGER NOT NULL,
    position   INTEGER NOT NULL CHECK (position >= 0),
    text       TEXT NOT NULL CHECK (length(text) >= 1),
    start_tick INTEGER,
    end_tick   INTEGER,
    PRIMARY KEY (line, word, position),
    FOREIGN KEY (line, word) REFERENCES lyric_word(line, position) ON DELETE CASCADE,
    CHECK ((start_tick IS NULL) = (end_tick IS NULL)),
    CHECK (start_tick IS NULL OR (start_tick >= 0 AND start_tick < end_tick))
);
-- Leitura eficiente da timeline (karaokê/exportação): sílabas com tempo, por início.
CREATE INDEX lyric_syllable_by_start ON lyric_syllable (start_tick) WHERE start_tick IS NOT NULL;

-- Projeção de leitura: palavras com texto e intervalo derivados (sem redundância armazenada).
CREATE VIEW v_lyric_words AS
SELECT line, word,
       group_concat(text, '') AS text,
       min(start_tick)        AS start_tick,
       max(end_tick)          AS end_tick
FROM (SELECT * FROM lyric_syllable ORDER BY line, word, position)
GROUP BY line, word;

-- Afinação (Fase 8) ---------------------------------------------------------
CREATE TABLE tuner_settings (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    a4_hz     REAL NOT NULL CHECK (a4_hz BETWEEN 415 AND 466)
);

-- Um mapa de correção por fonte de áudio (correção por escala inclusa: 1:1 com o mapa).
CREATE TABLE pitch_correction_map (
    id              TEXT PRIMARY KEY,
    source_id       TEXT NOT NULL UNIQUE REFERENCES audio_source(id) ON DELETE CASCADE,
    scale_enabled   INTEGER NOT NULL CHECK (scale_enabled IN (0, 1)),
    scale_key       INTEGER NOT NULL CHECK (scale_key BETWEEN 0 AND 11),
    retune_speed_ms REAL NOT NULL CHECK (retune_speed_ms BETWEEN 0 AND 1000),
    humanize        REAL NOT NULL CHECK (humanize BETWEEN 0 AND 1)
);

-- Classes de altura permitidas da escala (conjunto: 1 linha por classe, 1FN).
CREATE TABLE correction_scale_pitch (
    map_id      TEXT NOT NULL REFERENCES pitch_correction_map(id) ON DELETE CASCADE,
    pitch_class INTEGER NOT NULL CHECK (pitch_class BETWEEN 0 AND 11),
    PRIMARY KEY (map_id, pitch_class)
);

CREATE TABLE detected_note (
    id          TEXT PRIMARY KEY,
    map_id      TEXT NOT NULL REFERENCES pitch_correction_map(id) ON DELETE CASCADE,
    start_frame INTEGER NOT NULL CHECK (start_frame >= 0),
    end_frame   INTEGER NOT NULL,
    midi        REAL NOT NULL CHECK (midi BETWEEN 0 AND 127),
    CHECK (start_frame < end_frame)
);
CREATE INDEX detected_note_by_map_start ON detected_note (map_id, start_frame);

-- Edição manual de uma nota (0..1 por nota; edição neutra não é guardada).
CREATE TABLE note_edit (
    note_id           TEXT PRIMARY KEY REFERENCES detected_note(id) ON DELETE CASCADE,
    target_midi       INTEGER CHECK (target_midi BETWEEN 0 AND 127),
    cents_offset      REAL NOT NULL CHECK (cents_offset BETWEEN -1200 AND 1200),
    flatten           REAL NOT NULL CHECK (flatten BETWEEN 0 AND 1),
    formant_semitones REAL NOT NULL CHECK (formant_semitones BETWEEN -12 AND 12)
);

-- Partituras (Fase 7) — ADR-0027 -------------------------------------------
-- Cabeçalho normalizado (consultável, listagem sem desserializar o corpo) + corpo do agregado como
-- documento JSON versionado. Os dados do cabeçalho NÃO se repetem no documento (sem redundância).
CREATE TABLE score (
    id          TEXT PRIMARY KEY,
    position    INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    title       TEXT NOT NULL,
    composer    TEXT NOT NULL,
    doc_version INTEGER NOT NULL CHECK (doc_version >= 1),
    body        TEXT NOT NULL CHECK (json_valid(body))
);

CREATE TABLE score_part (
    score_id   TEXT NOT NULL REFERENCES score(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL CHECK (position >= 0),
    name       TEXT NOT NULL CHECK (length(name) >= 1),
    program    INTEGER NOT NULL CHECK (program BETWEEN 0 AND 127),
    percussion INTEGER NOT NULL CHECK (percussion IN (0, 1)),
    PRIMARY KEY (score_id, position)
);

-- Projeção para listagem (título, compositor, nº e nomes das partes).
CREATE VIEW v_score_summary AS
SELECT s.id, s.position, s.title, s.composer,
       (SELECT count(*) FROM score_part p WHERE p.score_id = s.id) AS part_count,
       (SELECT group_concat(name, ' / ')
          FROM (SELECT name FROM score_part p WHERE p.score_id = s.id ORDER BY p.position)) AS part_names
FROM score s
ORDER BY s.position;
