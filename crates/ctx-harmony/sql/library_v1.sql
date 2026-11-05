-- =====================================================================================
-- AppMusica — library.db, esquema versão 1 (Fase 5 Harmonia + Fase 4 Bases)
-- Especificação: docs/specs/2026-11-05-fase-5-harmonia-e-4-bases.md
--
-- Regras:
--   * Normalização até 4FN/BCNF: cada fato multivalorado independente em sua própria tabela
--     (intervalos, tensões, gêneros, tags, instrumentos).
--   * Nada derivável é armazenado: intervalos de modo vêm de família + rotação (view
--     v_mode_interval); campos harmônicos e acordes absolutos são calculados no domínio Rust;
--     progressões ficam em graus relativos (ADR-0007) e a qualidade só é gravada quando difere da
--     diatônica (quality_id NULL = diatônica).
--   * Tabelas de vocabulário (família, modo, qualidade, tensão, gênero, instrumento, padrões) NÃO
--     têm dados aqui: a aplicação as semeia a partir dos enums/catálogo Rust (fonte única).
--   * Este arquivo não contém BEGIN/COMMIT: a camada de persistência o executa dentro de uma
--     transação e grava PRAGMA user_version = 1 ao final.
-- =====================================================================================

PRAGMA foreign_keys = ON;

-- -------------------------------------------------------------------------------------
-- Metadados de semeadura (versão do catálogo embutido já aplicada).
-- -------------------------------------------------------------------------------------
CREATE TABLE seed_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

-- -------------------------------------------------------------------------------------
-- Escalas e modos
-- -------------------------------------------------------------------------------------
CREATE TABLE scale_family (
    id   INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE
);

CREATE TABLE scale_family_interval (
    family_id INTEGER NOT NULL REFERENCES scale_family(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL CHECK (position BETWEEN 1 AND 7),
    semitones INTEGER NOT NULL CHECK (semitones BETWEEN 0 AND 11),
    PRIMARY KEY (family_id, position),
    UNIQUE (family_id, semitones)
) WITHOUT ROWID;

CREATE TABLE mode (
    id        INTEGER PRIMARY KEY,
    code      TEXT NOT NULL UNIQUE,
    name_pt   TEXT NOT NULL,
    family_id INTEGER NOT NULL REFERENCES scale_family(id),
    rotation  INTEGER NOT NULL CHECK (rotation BETWEEN 0 AND 6),
    UNIQUE (family_id, rotation)
);

-- Intervalos de cada modo, derivados da família + rotação (mesma regra de Mode::intervals()).
CREATE VIEW v_mode_interval AS
SELECT m.id                                            AS mode_id,
       m.code                                          AS mode_code,
       ((fi.position - 1 - m.rotation + 7) % 7) + 1    AS degree,
       ((fi.semitones - base.semitones + 12) % 12)     AS semitones
FROM mode m
JOIN scale_family_interval fi   ON fi.family_id = m.family_id
JOIN scale_family_interval base ON base.family_id = m.family_id
                               AND base.position = m.rotation + 1;

-- -------------------------------------------------------------------------------------
-- Acordes (vocabulário)
-- -------------------------------------------------------------------------------------
CREATE TABLE chord_quality (
    id   INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE
);

CREATE TABLE chord_quality_interval (
    quality_id INTEGER NOT NULL REFERENCES chord_quality(id) ON DELETE CASCADE,
    semitones  INTEGER NOT NULL CHECK (semitones BETWEEN 0 AND 11),
    PRIMARY KEY (quality_id, semitones)
) WITHOUT ROWID;

CREATE TABLE tension (
    id        INTEGER PRIMARY KEY,
    code      TEXT NOT NULL UNIQUE,
    semitones INTEGER NOT NULL UNIQUE CHECK (semitones BETWEEN 0 AND 11)
);

-- Gênero: vocabulário aberto compartilhado por Harmony (enum Genre) e Library (GenreCode).
CREATE TABLE genre (
    id      INTEGER PRIMARY KEY,
    code    TEXT NOT NULL UNIQUE CHECK (code GLOB '[a-z]*' AND code NOT GLOB '*[^a-z0-9_]*'),
    name_pt TEXT NOT NULL
);

-- -------------------------------------------------------------------------------------
-- Progressões (graus relativos — ADR-0007; notação romana — ADR-0018)
-- -------------------------------------------------------------------------------------
CREATE TABLE progression (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 64),
    mode_id       INTEGER NOT NULL REFERENCES mode(id),
    is_universal  INTEGER NOT NULL DEFAULT 0 CHECK (is_universal IN (0, 1)),
    beats_per_bar INTEGER NOT NULL DEFAULT 4 CHECK (beats_per_bar BETWEEN 1 AND 16),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX ix_progression_mode ON progression(mode_id);

-- Subtipos exclusivos (sem colunas anuláveis): embutida (slug) ou do usuário (UUID v7).
CREATE TABLE builtin_progression (
    progression_id  INTEGER PRIMARY KEY REFERENCES progression(id) ON DELETE CASCADE,
    slug            TEXT NOT NULL UNIQUE CHECK (slug NOT GLOB '*[^a-z0-9-]*'),
    catalog_version INTEGER NOT NULL CHECK (catalog_version >= 1)
);

CREATE TABLE user_progression (
    progression_id INTEGER PRIMARY KEY REFERENCES progression(id) ON DELETE CASCADE,
    uuid           TEXT NOT NULL UNIQUE CHECK (length(uuid) = 36)
);

CREATE TRIGGER trg_builtin_progression_exclusive
BEFORE INSERT ON builtin_progression
WHEN EXISTS (SELECT 1 FROM user_progression WHERE progression_id = NEW.progression_id)
BEGIN
    SELECT RAISE(ABORT, 'progressão já é do usuário');
END;

CREATE TRIGGER trg_user_progression_exclusive
BEFORE INSERT ON user_progression
WHEN EXISTS (SELECT 1 FROM builtin_progression WHERE progression_id = NEW.progression_id)
BEGIN
    SELECT RAISE(ABORT, 'progressão já é embutida');
END;

CREATE TABLE progression_genre (
    progression_id INTEGER NOT NULL REFERENCES progression(id) ON DELETE CASCADE,
    genre_id       INTEGER NOT NULL REFERENCES genre(id),
    PRIMARY KEY (progression_id, genre_id)
) WITHOUT ROWID;
CREATE INDEX ix_progression_genre_genre ON progression_genre(genre_id, progression_id);

-- Um passo: grau (+acidente), qualidade sobrescrita (NULL = diatônica), baixo OU alvo de
-- dominante secundária, e duração em tempos.
CREATE TABLE progression_step (
    progression_id    INTEGER NOT NULL REFERENCES progression(id) ON DELETE CASCADE,
    position          INTEGER NOT NULL CHECK (position BETWEEN 0 AND 63),
    degree            INTEGER NOT NULL CHECK (degree BETWEEN 1 AND 7),
    accidental        INTEGER NOT NULL DEFAULT 0 CHECK (accidental BETWEEN -2 AND 2),
    quality_id        INTEGER REFERENCES chord_quality(id),
    diatonic_seventh  INTEGER NOT NULL DEFAULT 0 CHECK (diatonic_seventh IN (0, 1)),
    bass_degree       INTEGER CHECK (bass_degree BETWEEN 1 AND 7),
    bass_accidental   INTEGER CHECK (bass_accidental BETWEEN -2 AND 2),
    target_degree     INTEGER CHECK (target_degree BETWEEN 1 AND 7),
    target_accidental INTEGER CHECK (target_accidental BETWEEN -2 AND 2),
    beats             INTEGER NOT NULL CHECK (beats BETWEEN 1 AND 64),
    PRIMARY KEY (progression_id, position),
    -- Qualidade explícita exclui o marcador de tétrade diatônica.
    CHECK (quality_id IS NULL OR diatonic_seventh = 0),
    -- Grau alterado sem alvo secundário exige qualidade explícita.
    CHECK (accidental = 0 OR quality_id IS NOT NULL OR target_degree IS NOT NULL),
    -- Baixo e alvo: pares completos e mutuamente exclusivos.
    CHECK ((bass_degree IS NULL) = (bass_accidental IS NULL)),
    CHECK ((target_degree IS NULL) = (target_accidental IS NULL)),
    CHECK (bass_degree IS NULL OR target_degree IS NULL)
) WITHOUT ROWID;

CREATE TABLE progression_step_tension (
    progression_id INTEGER NOT NULL,
    position       INTEGER NOT NULL,
    tension_id     INTEGER NOT NULL REFERENCES tension(id),
    PRIMARY KEY (progression_id, position, tension_id),
    FOREIGN KEY (progression_id, position)
        REFERENCES progression_step(progression_id, position) ON DELETE CASCADE
) WITHOUT ROWID;

-- Leitura: lista de progressões com origem, modo, gêneros e totais (tela de catálogo).
CREATE VIEW v_progression_summary AS
SELECT p.id,
       p.name,
       m.code                                     AS mode_code,
       CASE WHEN b.progression_id IS NOT NULL THEN 'builtin' ELSE 'user' END AS origin,
       b.slug,
       u.uuid,
       p.is_universal,
       p.beats_per_bar,
       (SELECT count(*) FROM progression_step s WHERE s.progression_id = p.id)  AS step_count,
       (SELECT coalesce(sum(s.beats), 0) FROM progression_step s
         WHERE s.progression_id = p.id)                                         AS total_beats,
       (SELECT group_concat(g.code, ',') FROM progression_genre pg
          JOIN genre g ON g.id = pg.genre_id WHERE pg.progression_id = p.id)    AS genre_codes
FROM progression p
JOIN mode m ON m.id = p.mode_id
LEFT JOIN builtin_progression b ON b.progression_id = p.id
LEFT JOIN user_progression u    ON u.progression_id = p.id;

-- Leitura: passos com códigos resolvidos (o domínio reconstrói RomanChord a partir daqui).
CREATE VIEW v_progression_step AS
SELECT s.progression_id,
       s.position,
       s.degree,
       s.accidental,
       q.code AS quality_code,
       s.diatonic_seventh,
       s.bass_degree,
       s.bass_accidental,
       s.target_degree,
       s.target_accidental,
       s.beats,
       (SELECT group_concat(t.code, ',') FROM progression_step_tension st
          JOIN tension t ON t.id = st.tension_id
         WHERE st.progression_id = s.progression_id AND st.position = s.position) AS tension_codes
FROM progression_step s
LEFT JOIN chord_quality q ON q.id = s.quality_id;

-- -------------------------------------------------------------------------------------
-- Bases instrumentais (Fase 4)
-- -------------------------------------------------------------------------------------
CREATE TABLE instrument (
    id      INTEGER PRIMARY KEY,
    code    TEXT NOT NULL UNIQUE,
    name_pt TEXT NOT NULL
);

CREATE TABLE tag (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (length(name) BETWEEN 1 AND 32)
);

CREATE TABLE style_pattern (
    id              INTEGER PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name_pt         TEXT NOT NULL,
    meter_numerator INTEGER NOT NULL CHECK (meter_numerator BETWEEN 1 AND 16)
);

CREATE TABLE drum_pattern (
    id      INTEGER PRIMARY KEY,
    code    TEXT NOT NULL UNIQUE,
    name_pt TEXT NOT NULL
);

CREATE TABLE bass_pattern (
    id      INTEGER PRIMARY KEY,
    code    TEXT NOT NULL UNIQUE,
    name_pt TEXT NOT NULL
);

CREATE TABLE base (
    id                INTEGER PRIMARY KEY,
    uuid              TEXT NOT NULL UNIQUE CHECK (length(uuid) = 36),
    name              TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 64),
    kind              TEXT NOT NULL CHECK (kind IN ('bundled_cc0', 'generated', 'user_audio')),
    bpm               REAL NOT NULL CHECK (bpm BETWEEN 20 AND 999),
    tonic             INTEGER CHECK (tonic BETWEEN 0 AND 11),
    key_mode          TEXT CHECK (key_mode IN ('major', 'minor')),
    meter_numerator   INTEGER NOT NULL DEFAULT 4 CHECK (meter_numerator BETWEEN 1 AND 16),
    meter_denominator INTEGER NOT NULL DEFAULT 4 CHECK (meter_denominator IN (2, 4, 8, 16)),
    genre_id          INTEGER REFERENCES genre(id),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- Tom é tudo ou nada; base gerada sempre tem tom (a receita é em graus).
    CHECK ((tonic IS NULL) = (key_mode IS NULL)),
    CHECK (kind <> 'generated' OR tonic IS NOT NULL)
);
CREATE INDEX ix_base_bpm   ON base(bpm);
CREATE INDEX ix_base_key   ON base(tonic, key_mode);
CREATE INDEX ix_base_genre ON base(genre_id);
CREATE INDEX ix_base_kind  ON base(kind);
CREATE INDEX ix_base_name  ON base(name COLLATE NOCASE);

-- Áudio (CC0 embutido ou do usuário): caminho relativo à raiz da biblioteca e progressão anotada.
CREATE TABLE base_audio (
    base_id          INTEGER PRIMARY KEY REFERENCES base(id) ON DELETE CASCADE,
    rel_path         TEXT NOT NULL UNIQUE
                     CHECK (rel_path NOT LIKE '/%' AND rel_path NOT LIKE '%..%'
                            AND instr(rel_path, '\') = 0 AND instr(rel_path, ':') = 0),
    progression_text TEXT CHECK (progression_text IS NULL OR length(progression_text) <= 256)
);

-- Receita de base gerada pelo app (a progressão vem daqui; não é duplicada em base_audio).
CREATE TABLE base_recipe (
    base_id          INTEGER PRIMARY KEY REFERENCES base(id) ON DELETE CASCADE,
    progression_text TEXT NOT NULL CHECK (length(progression_text) BETWEEN 1 AND 256),
    bars             INTEGER NOT NULL CHECK (bars BETWEEN 1 AND 256),
    style_id         INTEGER NOT NULL REFERENCES style_pattern(id),
    drum_pattern_id  INTEGER NOT NULL REFERENCES drum_pattern(id),
    bass_pattern_id  INTEGER NOT NULL REFERENCES bass_pattern(id)
);
CREATE INDEX ix_base_recipe_style ON base_recipe(style_id);

CREATE TRIGGER trg_base_audio_kind
BEFORE INSERT ON base_audio
WHEN (SELECT kind FROM base WHERE id = NEW.base_id) NOT IN ('bundled_cc0', 'user_audio')
BEGIN
    SELECT RAISE(ABORT, 'base_audio só para bases de áudio');
END;

CREATE TRIGGER trg_base_recipe_kind
BEFORE INSERT ON base_recipe
WHEN (SELECT kind FROM base WHERE id = NEW.base_id) <> 'generated'
BEGIN
    SELECT RAISE(ABORT, 'base_recipe só para bases geradas');
END;

CREATE TRIGGER trg_base_kind_immutable
BEFORE UPDATE OF kind ON base
WHEN NEW.kind <> OLD.kind
BEGIN
    SELECT RAISE(ABORT, 'o tipo da base não pode mudar');
END;

CREATE TABLE base_instrument (
    base_id       INTEGER NOT NULL REFERENCES base(id) ON DELETE CASCADE,
    instrument_id INTEGER NOT NULL REFERENCES instrument(id),
    PRIMARY KEY (base_id, instrument_id)
) WITHOUT ROWID;
CREATE INDEX ix_base_instrument_instrument ON base_instrument(instrument_id, base_id);

CREATE TABLE base_tag (
    base_id INTEGER NOT NULL REFERENCES base(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tag(id),
    PRIMARY KEY (base_id, tag_id)
) WITHOUT ROWID;
CREATE INDEX ix_base_tag_tag ON base_tag(tag_id, base_id);

-- Leitura: lista de bases para a tela de biblioteca (uma linha por base).
CREATE VIEW v_base_summary AS
SELECT b.id,
       b.uuid,
       b.name,
       b.kind,
       b.bpm,
       b.tonic,
       b.key_mode,
       b.meter_numerator,
       b.meter_denominator,
       g.code                                       AS genre_code,
       a.rel_path,
       coalesce(r.progression_text, a.progression_text) AS progression_text,
       sp.code                                      AS style_code,
       r.bars,
       (SELECT group_concat(i.code, ',') FROM base_instrument bi
          JOIN instrument i ON i.id = bi.instrument_id WHERE bi.base_id = b.id) AS instrument_codes,
       (SELECT group_concat(t.name, ',') FROM base_tag bt
          JOIN tag t ON t.id = bt.tag_id WHERE bt.base_id = b.id)              AS tags
FROM base b
LEFT JOIN genre g          ON g.id = b.genre_id
LEFT JOIN base_audio a     ON a.base_id = b.id
LEFT JOIN base_recipe r    ON r.base_id = b.id
LEFT JOIN style_pattern sp ON sp.id = r.style_id;
