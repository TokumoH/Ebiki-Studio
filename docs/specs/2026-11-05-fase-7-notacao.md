# Fase 7 — Notação: partitura e tablatura (Especificação)

- **Data:** 2026-11-05 · **Volta da espiral:** 7 · **Status:** Aprovada (modo autônomo)
- **Requisitos:** RF-7.1 a RF-7.4; parte de RF-9.1/RF-9.2 (MIDI, MusicXML, Guitar Pro)
- **ADRs:** 0008 (comandos), 0010 (alphaTab), 0020 (Guitar Pro), 0021 (modelo e I/O de notação)
- **Crates:** `crates/ctx-notation` (domínio), `crates/infra-notation-io` (Pure Fabrication)

## 1. Objetivos

1. Modelo de domínio **completo** de partitura e tablatura: partes, pautas (padrão e/ou tablatura com
   afinação e capotraste), compassos (armadura, fórmula, repetições), até 4 vozes, notas/acordes/pausas,
   figuras de semibreve a quartifusa (1/128) com pontos e quiálteras, ligaduras de valor, articulações,
   dinâmicas, crescendo/diminuendo, ligaduras de expressão, apojaturas, letra e técnicas de cordas.
2. **Editor completo** com operações desfazíveis (`Command<Score>`), todas com teste de ida e volta.
3. **Auxílios**: validar compasso, completar com pausas, transposição com grafia correta, destacar notas
   fora da escala, gerar partitura a partir de acordes, mapear tablatura ⇄ altura.
4. **Reprodução**: partitura → eventos MIDI temporizados (para soundfont GM e exportação).
5. **Formatos**: MIDI (importar/exportar), MusicXML `.musicxml`/`.mxl` (importar/exportar),
   Guitar Pro 7+ (importar; GP3–GP6 via alphaTab na UI — ADR-0020).

Fora desta volta: renderização/edição visual (UI com alphaTab), layout de página e PDF, cifras sobre a
pauta, quiálteras aninhadas, repetições aninhadas/casas de 1ª e 2ª vez, D.C./D.S./Coda.

## 2. Riscos (volta 7)

| ID | Risco | Prob. | Impacto | Mitigação | Teste que comprova |
|---|---|---|---|---|---|
| R1 | Erro acumulado de duração em quiálteras/pontos | Alta | Alto | Durações como fração exata (`Frac`, 128 bits na redução); ticks só na borda | `duration::decompose_sums_to_total`, `playback::tuplets_are_exact`, `frac::ticks_roundtrip` |
| R2 | Grafia enarmônica errada ao transpor | Alta | Médio | Armadura de destino com menos acidentes; diatônicas pela armadura, cromáticas pela direção | `pitch::spelling_preserves_midi`, `pitch::diatonic_notes_use_key_accidentals`, `transpose::transpose_roundtrip_preserves_midi` |
| R3 | Desfazer não restaura exatamente | Média | Alto | Comandos aplicam numa cópia do compasso, validam e trocam (atômico); inverso guarda snapshot mínimo + contador de ids | `commands::*_roundtrip`, `commands::works_with_history` |
| R4 | Partes com estruturas de compasso divergentes | Média | Alto | Invariante `check_invariants`; comandos estruturais aplicam em todas as partes; importadores normalizam | `model::push_part_assigns_ids_and_rejects_mismatch`, `musicxml::imported_partial_files_stay_consistent` |
| R5 | Perda de informação em MusicXML (ida e volta) | Média | Alto | Mapeamento explícito (seção 6) e teste de ida e volta de partitura rica | `musicxml::musicxml_roundtrip_preserves_rich_score`, `roundtrip_keeps_playback_identical` |
| R6 | MIDI com sobreposições/notas fora da grade gera partitura ilegível | Alta | Médio | Quantização configurável, acordes por (início, fim), vozes até 4, ligaduras nas barras | `arrange::overlapping_notes_go_to_new_voices_and_excess_is_dropped`, `midi::imports_format0_with_quantization_and_other_resolution` |
| R7 | Zip bomb / arquivo enorme | Baixa | Alto | Limites de tamanho; leitura descompactada com `take`; nº de entradas | `archive::rejects_bomb_entry`, `error::codes_are_stable` |
| R8 | XML malicioso (profundidade, entidades) | Baixa | Alto | DOM com limite de profundidade/nós; entidades externas não resolvidas | `xml::rejects_malformed_and_deep_documents`, `musicxml::malformed_inputs_are_errors_not_panics` |
| R9 | Pânico com entrada corrompida | Média | Alto | Toda leitura retorna `Result`; aritmética saturada; fuzz | `fuzz::random_bytes_never_panic`, `fuzz::corrupted_valid_files_never_panic`, `guitar_pro::mutated_gpif_never_panics`, `midi::garbage_and_truncated_midi_never_panics` |
| R10 | Formato Guitar Pro proprietário | Alta | Médio | Só GPIF (XML) no núcleo; GP3–GP6 → GP7 pelo alphaTab (ADR-0020) | `guitar_pro::imports_gp7_core_subset` |
| R11 | Tablatura incoerente com a altura | Média | Médio | `tab::pitch_of`/`best_position`; posição validada contra afinação/braço; ao mudar a altura a posição acompanha | `tab::every_position_maps_back`, `commands::tab_and_techniques`, `transpose::tab_follows_transposition` |

## 3. Modelo de domínio

```mermaid
classDiagram
    class Score {
        +String title
        +String composer
        +TempoMap tempo
        +Vec~Part~ parts
        +u64 next_event_id
        +meters() Vec~Meter~
        +measure_starts() Vec~Tick~
        +tempo_map() TempoMap
        +check_invariants()
    }
    class Part {
        +String name
        +u8 program
        +bool percussion
        +Vec~Staff~ staves
        +Vec~Measure~ measures
        +Vec~Spanner~ spanners
    }
    class Staff { +Clef clef; +bool standard; +Option~TabConfig~ tab }
    class TabConfig { +Vec~u8~ tuning; +u8 capo; +u8 frets }
    class Measure {
        +KeySignature key
        +Option~Meter~ meter
        +bool repeat_start
        +Option~u8~ repeat_end
        +Vec~Voice~ voices
    }
    class Voice { +u8 staff; +Vec~Event~ events }
    class Event {
        +EventId id
        +Duration duration
        +EventKind kind
        +Vec~Articulation~ articulations
        +Option~Dynamic~ dynamic
        +Option~GraceKind~ grace
        +Option~Lyric~ lyric
    }
    class EventKind { <<enum>> Rest; Chord(Vec~Note~) }
    class Note { +Pitch pitch; +bool tie; +Option~TabPosition~ tab; +Vec~Technique~ techniques }
    class Duration { +NoteValue value; +u8 dots; +Option~Tuplet~ tuplet; +fraction() Frac }
    class Pitch { +Step step; +i8 alter; +i8 octave; +midi() }
    class KeySignature { +i8 fifths; +Mode mode; +spell(); +transposed() }
    class Spanner { +SpannerKind kind; +EventId start; +EventId end }
    class Technique { <<enum>> HammerOn; PullOff; Slide; Bend(cents); Vibrato; PalmMute; Harmonic; LetRing }
    Score "1" *-- "n" Part
    Part "1" *-- "1..2" Staff
    Staff o-- "0..1" TabConfig
    Part "1" *-- "n" Measure
    Part "1" *-- "n" Spanner
    Measure "1" *-- "1..4" Voice
    Voice "1" *-- "n" Event
    Event *-- EventKind
    EventKind o-- "n" Note
    Event *-- Duration
    Note *-- Pitch
    Note o-- "n" Technique
    Measure *-- KeySignature
    Spanner ..> Event : por EventId
```

### Invariantes
- Todas as partes têm o mesmo nº de compassos (≥ 1) e os mesmos atributos de compasso por índice.
- Fórmula de compasso: numerador 1–32, denominador ∈ {1,2,4,8,16,32}; `None` herda (o 1º compasso herda a
  fórmula inicial do `TempoMap`). Armadura −7…+7. Repetição final toca 2–16 vezes.
- ≤ 4 vozes por compasso; cada voz aponta para uma pauta da parte (≤ 2 pautas); ≤ 1.024 eventos por voz;
  ≤ 16 notas por acorde; altura em 0–127; acidente −2…+2; até 3 pontos; quiálteras 2–32 : 1–32.
- Tablatura: corda 1 = mais aguda; `altura = afinação[corda] + capotraste + traste`; traste + capo ≤ trastes.
- Ids de evento únicos, não nulos e menores que `next_event_id`; spanners apontam para eventos da parte.
- Apojaturas não ocupam tempo no compasso; pausas não podem ser apojaturas.

## 4. Comandos de edição (`ctx_notation::commands`)

| Comando | Variante | Inverso |
|---|---|---|
| `EditVoice` | `Insert {index, event}` (cria voz nova se índice = nº de vozes) | snapshot do compasso + spanners + contador de ids |
| | `Delete {index}` (remove spanners do evento) | idem |
| | `Replace {index, event}` (mantém id) | idem |
| | `SetDuration`, `SetPitch` (tablatura acompanha), `AddNote`, `RemoveNote` | idem |
| | `AddArticulation`, `RemoveArticulation`, `SetDynamic` (adicionar/remover) | idem |
| | `SetTie`, `SetTab` (altura acompanha), `AddTechnique`, `RemoveTechnique` | idem |
| | `SetLyric`, `SetGrace`, `FillRests` | idem |
| `EditScore` | `AddMeasure {index}` / `RemoveMeasure {index}` (fórmula migra ao seguinte) | remove/reinsere; restaura contador/spanners |
| | `SetKey {measure, key}` (até a próxima mudança) | lista de armaduras anteriores |
| | `SetMeter {measure, meter}` / `SetRepeat {measure, start, end}` | valor anterior |
| | `AddPart {index, part}` / `RemovePart {index}` | remove/reinsere |
| | `AddSpanner` / `RemoveSpanner` (ligadura, crescendo, diminuendo) | pop/insere |
| | `Transpose {scope: Score/Part/Events, semitones}` | partes anteriores |
| | `SetHeader {title, composer}`, `SetTempo {tick, bpm}` | valores anteriores |

Todos validam antes de alterar (atômicos) e retornam `CommandError` com código `notation.*` e mensagem
em PT-BR. Teste padrão: aplicar → desfazer = antes; refazer = depois; invariantes válidas.

## 5. Auxílios e serviços de domínio

| Função | Módulo | Descrição |
|---|---|---|
| `validate_measures(score)` | `analysis` | voz a voz: `Underfilled{missing}` / `Overfilled{excess}` (frações exatas) |
| `rests_to_fill`, `fill_rests` | `analysis` | completa a voz com pausas decompostas |
| `out_of_scale_notes(score, &[PitchClass])` | `analysis` | notas fora de um conjunto de classes (escala vinda da camada de aplicação, sem depender de Harmony) |
| `out_of_key_notes(score)` | `analysis` | fora da escala diatônica da armadura |
| `transpose(score, scope, semitones)` | `transpose` | grafia pela armadura de destino; armaduras transpostas no escopo `Score`; tablatura reposicionada |
| `chords_to_part` / `chords_to_score` | `arrange` | `ChordSlot {start_tick, duration_ticks, notes}` → piano em pauta dupla (divisão em Dó4) |
| `arrange_part`, `build_template` | `arrange` | notas temporizadas → compassos/vozes (quantização, acordes, vozes, ligaduras, pausas, dinâmicas por velocity, tablatura) |
| `pitch_of`, `positions_for`, `best_position`, `assign_chord` | `tab` | tablatura ⇄ altura; melhor posição perto da mão; acorde em cordas distintas com menor abertura |
| `render`, `performance_order`, `channels` | `playback` | eventos `{part, start_tick, duration_ticks, channel, program, key, velocity}` |

**Reprodução:** ligaduras fundem notas (inclusive entre compassos e repetições); quiálteras exatas;
dinâmica → velocity (ppp 16 … fff 127, padrão mf 80); crescendo/diminuendo interpolam até a próxima
dinâmica (ou ±24); acento +12, marcato +20; staccato toca metade; acciaccatura antes do tempo
(1/64), appoggiatura divide a duração; repetição simples expandida; percussão no canal 10.

## 6. Mapeamentos de I/O (`infra_notation_io`)

### 6.1 MIDI (SMF)

| SMF | Score |
|---|---|
| Cabeçalho formato 1, 960 PPQ (exportação) / qualquer PPQ métrico (importação) | ticks a 960 PPQ |
| Trilha 0: `TrackName`, `Tempo`, `TimeSignature`, `KeySignature` | `title`, `tempo`, `Measure.meter`, `Measure.key` |
| Uma trilha por parte: `TrackName`, `ProgramChange`, `NoteOn/NoteOff` | `Part.name`, `Part.program`, notas de `playback::render` |
| Canal 10 | `Part.percussion` |
| Importação: uma parte por (trilha, canal); formato 0 dividido por canal | `arrange_part` com grade (`MidiImportOptions.grid_ticks`, padrão semicolcheia) |
| Velocity | marca de dinâmica quando muda de faixa |
| Programa 24–31 / 32–39 / 0–7 | violão/guitarra com tab / baixo com tab / piano em pauta dupla |

Exportação expande repetições por padrão (`MidiExportOptions.expand_repeats`). SMPTE: não suportado.

### 6.2 MusicXML 4.0 (partwise)

| MusicXML | Score |
|---|---|
| `work/work-title`, `creator[@type=composer]` | `title`, `composer` |
| `score-part`, `part-name`, `midi-program` (1-based), `midi-channel` = 10 | `Part.name`, `program`, `percussion` |
| `divisions` (exporta 960) | durações por `<type>`, `<dot/>`, `<time-modification>`; sem `<type>`: deduzido de `<duration>` |
| `key/fifths`, `key/mode` | `Measure.key` |
| `time/beats` (aceita `3+2`), `time/beat-type` | `Measure.meter` / fórmula inicial |
| `staves`, `clef` (`G`, `G`+`clef-octave-change −1`, `F`, `C`, `percussion`, `TAB`) | `Part.staves[].clef`, `standard` |
| `staff-details/staff-tuning` (linha 1 = corda grave), `capo` | `TabConfig.tuning` (corda 1 = aguda), `capo` |
| `note/pitch` (`step`, `alter`, `octave`), `unpitched`, `rest`, `chord` | `Note.pitch`, `EventKind` |
| `voice`, `staff`, `backup`, `forward` | `Measure.voices[]` (até 4, por ordem de aparição), `Voice.staff` |
| `tie` + `tied` | `Note.tie` |
| `articulations` (`staccato`, `accent`, `tenuto`, `strong-accent`), `fermata` | `Event.articulations` |
| `direction/dynamics` (ppp…fff) | `Event.dynamic` (no próximo evento) |
| `direction/wedge` (`crescendo`/`diminuendo`/`stop`, `number`) | `Spanner` Crescendo/Diminuendo |
| `slur` (`start`/`stop`, `number`) | `Spanner` Slur |
| `technical/string`, `fret` | `Note.tab` |
| `hammer-on`, `pull-off`, `bend/bend-alter` (semitons), `harmonic` (`natural`/`artificial`), `other-technical` ("palm mute", "let ring") | `Technique::*` |
| `slide`/`glissando` start, `ornaments/wavy-line` start ou `vibrato` | `Slide`, `Vibrato` |
| `grace` (`slash="yes"` = acciaccatura) | `Event.grace` |
| `lyric` (`syllabic`, `text`) — 1ª estrofe | `Event.lyric` |
| `barline/repeat` forward / backward (`times`) | `repeat_start` / `repeat_end` |
| `sound/@tempo`, `metronome` | `TempoMap` (mudanças no início do compasso) |
| `.mxl`: `META-INF/container.xml` → `rootfile/@full-path` | idem |

### 6.3 Guitar Pro 7+ (GPIF) — ver ADR-0020

| GPIF | Score |
|---|---|
| `Score/Title`, `Score/Music` (ou `Artist`) | `title`, `composer` |
| `Track/Name`, `Sounds/Sound/MIDI/Program`, `MidiConnection/PrimaryChannel` = 9, `InstrumentSet/Type=drumKit` | `Part` |
| `Staves/Staff/Properties`: `Tuning/Pitches` (grave→aguda), `CapoFret/Fret` | `Staff.tab` |
| `MasterBar`: `Key/AccidentalCount`, `Key/Mode`, `Time` "n/d", `Repeat@start/@end/@count`, `Bars` (por faixa e pauta) | `Measure` |
| `Bar/Voices` (4 slots, −1 = vazio) → `Voice/Beats` | `Measure.voices` |
| `Beat`: `Rhythm@ref`, `Notes`, `Dynamic`, `GraceNotes` (OnBeat/BeforeBeat) | `Event` |
| `Rhythm`: `NoteValue`, `AugmentationDot@count`, `PrimaryTuplet@num/@den` | `Duration` |
| `Note`: `String` (0 = grave), `Fret`, `Midi`, `ConcertPitch`; `Tie@origin`; `Accent` (bits 1/4/8/16) | `Note`, `Event.articulations` |
| `HopoOrigin`, `Slide/Flags`, `Bended`+`BendDestinationValue` (100 = 1 tom), `Vibrato`, `PalmMuted`, `HarmonicType/HType`, `LetRing` | `Technique::*` (hammer vira pull-off se a próxima nota for mais grave) |
| `MasterTrack/Automations/Automation[Type=Tempo]` `Bar`, `Value` | `TempoMap` |

### 6.4 Segurança de entrada

Arquivo ≤ 32 MiB · entrada de zip ≤ 64 MiB descompactada (lida com `take`) · ≤ 2.000 entradas ·
XML profundidade ≤ 128 e ≤ 2 mi elementos · entidades externas não resolvidas · ≤ 500 mil notas MIDI ·
limites do domínio (64 partes, 10.000 compassos, 4 vozes, 1.024 eventos/voz, 16 notas/acorde) ·
aritmética de frações em 128 bits com saturação · avisos (PT-BR) no `Imported.warnings`.

## 7. Componentes

```mermaid
flowchart LR
    UI[UI React + alphaTab] -- IPC tipado --> APP[app-core]
    APP --> NOT[ctx-notation<br/>Score + comandos + auxílios + playback]
    APP --> IO[infra-notation-io<br/>MIDI · MusicXML · GPIF]
    IO --> NOT
    NOT --> SK[shared-kernel<br/>Tick · PPQ · TempoMap · PitchClass · Command]
    UI -. GP3–GP6 → Gp7Exporter .-> APP
    APP -. eventos de playback .-> ENG[audio-engine / synth GM]
```

## 8. Critérios de aceite

- [x] Modelo cobre padrão + tablatura, 4 vozes, figuras 1–1/128, pontos, quiálteras, ligaduras,
      articulações, dinâmicas, crescendo/diminuendo, ligaduras de expressão, apojaturas, letra e técnicas.
- [x] Todos os comandos da seção 4 com teste de ida e volta (aplicar/desfazer/refazer) e integração com `History`.
- [x] Validação de compasso, preenchimento de pausas, notas fora da escala (conjunto de classes), transposição
      com grafia correta (propriedade: +n −n preserva alturas e grafia segue a armadura).
- [x] Acordes → partitura de piano válida (sem compassos incompletos).
- [x] Tablatura ⇄ altura com capotraste (propriedade: toda posição volta à mesma altura).
- [x] Reprodução honra ligaduras, quiálteras, dinâmicas/rampas e repetições.
- [x] MusicXML: ida e volta da partitura rica sem perda (modelo e reprodução idênticos); `.mxl`; amostra escrita à mão.
- [x] MIDI: ida e volta preserva reprodução, fórmulas, andamentos e armadura; importação formato 0 com
      quantização e outra resolução.
- [x] Guitar Pro 7: importação do subconjunto central (amostra GPIF), `.gp` e GPIF bruto.
- [x] Fuzz: bytes aleatórios e arquivos válidos corrompidos nunca causam pânico; zip bomb rejeitado.
- [x] `cargo fmt`, `cargo clippy -D warnings` e testes verdes nos dois crates.

## 9. Limitações conhecidas (próximas voltas)

- Repetições aninhadas, casas de 1ª/2ª vez, D.C./D.S./Coda não modelados; fermata não altera o tempo na reprodução.
- Mudanças de andamento no meio do compasso são exportadas em MusicXML no início do compasso.
- MusicXML: só a 1ª estrofe de letra; tie de "stop" é inferido (só `start` é guardado); quiálteras aninhadas,
  ornamentos além de vibrato, cifras (`harmony`), instrumentos transpositores e layout não são lidos.
- MIDI: velocities individuais viram marcas de dinâmica (perda de nuance); notas além de 4 vozes sobrepostas
  são descartadas com aviso; SMPTE não suportado.
- GPIF: bends com múltiplos pontos viram um único valor; slides sem distinção de tipo; letras, acordes,
  automação de volume e efeitos de batida (whammy, tapping, rasgueado) não importados.
- `SetKey` não regrafa as notas existentes (mantém as alturas escritas).

## 10. Integração (volta 2)

- **Data:** 2026-11-05 · **Status:** Implementada (backend; UI fora desta volta)
- **ADRs:** [0027](../adr/0027-partitura-documento-versionado-com-cabecalho-normalizado.md) (persistência
  da partitura), [0029](../adr/0029-processamento-de-altura-e-tempo-gera-fonte-derivada.md) (render para faixa)
- **Código:** `ctx_notation::book` (`ScoreBook`, `ScoreId`, `AddScore`, `RemoveScore`, `OnScore`),
  `app_core::notation_service`, `infra_persistence::music_tables`, `app-desktop/src/commands_music.rs`

### 10.1 Objetivos

1. Partituras como agregado da sessão: `Session.scores: ScoreBook` (lista ordenada com `ScoreId`
   estável); toda edição passa pelo `History` do `Studio` (`on_score(id, EditVoice|EditScore)`).
2. Casos de uso: criar em branco (modelo de instrumentos), criar de acordes realizados
   (`chords_to_score`), editar voz/estrutura, remover, importar (MusicXML/MXL/MIDI/GP7), exportar
   (MusicXML/MXL/MIDI), validar (compassos + fora da escala), ouvir (WAV em memória) e **renderizar
   para faixa** (fonte + faixa + clipe num único desfazer).
3. Persistência no `project.db` v2 (cabeçalho normalizado + corpo versionado — ADR-0027).
4. PDF: gerado na UI pelo alphaTab a partir de `score_get` (nenhum comando de backend).

### 10.2 Riscos (volta 2)

| ID | Risco | Mitigação | Teste que comprova |
|---|---|---|---|
| I1 | Desfazer de edição de partitura afeta outra partitura ou fica inconsistente | `OnScore` localiza por `ScoreId` (não por índice); comandos de `ctx-notation` já são atômicos | `book::tests::add_remove_and_edit_by_id_roundtrip_through_history`, `music::score_edits_undo_and_redo_through_the_studio_history` |
| I2 | Perda na ida e volta pelo app (exportar → importar) | Reaproveita os mapeamentos testados do `infra-notation-io`; reprodução comparada evento a evento | `music::import_export_musicxml_roundtrip_via_studio` (MusicXML, MXL e MIDI) |
| I3 | Arquivo do usuário enorme/malicioso trava o app | Tamanho checado **antes** de ler (32 MiB); limites do `infra-notation-io` (zip bomb, XML profundo) | `music::import_export_musicxml_roundtrip_via_studio` (formato desconhecido → `notation_io.unsupported`), fuzz do `infra-notation-io` |
| I4 | Render para faixa deixa estado parcial (fonte sem clipe, faixa sem canal) | Um único `Composite` (fonte, faixa, canal, clipe); áudio gravado antes de executar | `music::score_audition_and_render_to_track_is_one_undo` |
| I5 | Partitura persistida diverge do cabeçalho / documento corrompido | Cabeçalho só nas colunas (sem redundância); `doc_version`; `check_invariants` na carga | `music_roundtrip::score_header_lives_in_columns_not_in_the_document`, `corrupted_score_document_is_rejected`, `lyrics_tuning_and_scores_roundtrip` |
| I6 | Render muito longo esgota memória | Limite de 30 min de áudio por partitura; partitura sem notas → erro | `music::score_audition_and_render_to_track_is_one_undo` (`app.empty_score`) |
| I7 | Edições inválidas vindas da UI (JSON) | `VoiceEdit`/`ScoreEdit` desserializados com tag `kind`; validação do domínio antes de aplicar | `commands_music::tests::requests_deserialize_from_camel_case_json`, `book::tests::edits_serialize_with_kind_tag` |

### 10.3 Sequência — renderizar partitura para faixa

```mermaid
sequenceDiagram
    participant UI
    participant C as commands_music
    participant S as Studio (notation_service)
    participant N as ctx-notation
    participant Y as MidiRenderer (synth GM)
    participant M as MediaPort
    participant H as History
    UI->>C: score_render_to_track {scoreId, trackId?, atTick}
    C->>S: score_render_to_track(id, track, at)
    S->>N: playback::render(score) → PlaybackEvent[]
    S->>S: → MidiNoteEvent[] (limite 30 min)
    S->>Y: render(eventos, score.tempo_map(), sr, cauda 1,5 s)
    Y-->>S: SourceBuffer (estéreo)
    S->>M: encode(audio/rendered/<uuid>.wav, float 32)
    S->>M: hash_file → fonte nova (ou reaproveitada)
    S->>H: execute(Composite[AddSource, CreateTrack?, AddStrip?, AddClip])
    H-->>S: ok (um item "Renderizar partitura")
    S-->>UI: SessionView (faixa/clipe novos)
```

### 10.4 Contrato IPC (JSON camelCase)

Erros: `{code, message}` (`notation.*`, `notation_io.*`, `app.*`, `api.invalid_argument`).
`SessionView` ganhou `scores: ScoreSummary[]` (`{id, title, composer, parts: string[], measures}`).

| Comando | Argumentos | Retorno |
|---|---|---|
| `score_list` | — | `ScoreSummary[]` |
| `score_create` | `req: {title, composer?, bpm, meterNumerator, meterDenominator, keyFifths?, keyMode?: "major"\|"minor", measures (1–1000), instruments: [{kind: "piano"\|"lead"\|"guitar"\|"bass"\|"drums", name?}]}` | `{scoreId, view: SessionView}` |
| `score_create_from_chords` | `req: {title, meterNumerator, meterDenominator, keyFifths?, keyMode?, slots: [{startTick, durationTicks, notes: number[]}]}` (BPM = do projeto) | `{scoreId, view}` |
| `score_get` | `scoreId` | `Score` completo (modelo da seção 3, camelCase) |
| `score_delete` | `scoreId` | `SessionView` |
| `score_edit_voice` | `req: {scoreId, loc: {part, measure, voice}, edit: VoiceEdit, gesture?}` — `VoiceEdit` = `{kind: "insert"\|"delete"\|"replace"\|"setDuration"\|"setPitch"\|"addNote"\|"removeNote"\|"addArticulation"\|"removeArticulation"\|"setDynamic"\|"setTie"\|"setTab"\|"addTechnique"\|"removeTechnique"\|"setLyric"\|"setGrace"\|"fillRests", ...campos da seção 4 em camelCase}` | `SessionView` |
| `score_edit` | `req: {scoreId, edit: ScoreEdit}` — `{kind: "addMeasure"\|"removeMeasure"\|"setKey"\|"setMeter"\|"setRepeat"\|"addPart"\|"removePart"\|"addSpanner"\|"removeSpanner"\|"transpose"\|"setHeader"\|"setTempo", ...}`; `transpose.scope` = `{scope: "score"}` \| `{scope: "part", target: n}` \| `{scope: "events", target: [ids]}` | `SessionView` |
| `score_import` | `path` (absoluto; `.musicxml/.xml/.mxl/.mid/.gp/.gpif`, detecção pelo conteúdo) | `{scoreId, warnings: string[], view}` |
| `score_export` | `req: {scoreId, path, format: "musicXml"\|"mxl"\|"midi"}` | `null` |
| `score_audition` | `scoreId` | bytes crus de WAV float 32 (`ArrayBuffer`) |
| `score_render_to_track` | `req: {scoreId, trackId?, atTick?}` | `SessionView` |
| `score_validate` | `req: {scoreId, scale?: number[] (classes 0–11)}` | `{measureIssues: [{loc, issue: {kind: "underfilled", missing} \| {kind: "overfilled", excess}}], outOfScale: [{event: {voice, index}, noteIndex, pitch}]}` |

### 10.5 Persistência (DER v2 — partituras)

```mermaid
erDiagram
    score ||--o{ score_part : "partes (ordem)"
    score {
        TEXT id PK
        INTEGER position UK
        TEXT title
        TEXT composer
        INTEGER doc_version
        TEXT body "JSON sem cabeçalho (ADR-0027)"
    }
    score_part {
        TEXT score_id FK
        INTEGER position
        TEXT name
        INTEGER program
        INTEGER percussion
    }
```

Projeção `v_score_summary` (título, compositor, nº e nomes das partes) para listagens sem
desserializar o corpo.

### 10.6 Critérios de aceite (volta 2)

- [x] Criar/editar/remover partituras com desfazer/refazer exato pelo `Studio`.
- [x] Importar MusicXML/MXL/MIDI e exportar MusicXML/MXL/MIDI pelo app com reprodução idêntica.
- [x] Acordes → partitura; validação de compassos e notas fora da escala (armadura ou escala dada).
- [x] Audição (WAV em memória) e render para faixa num único desfazer.
- [x] Persistência v2 com identidade salvar → carregar; migração v1 → v2.
- [x] `cargo fmt`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` verdes.

### 10.7 Limitações (volta 2)

- O áudio renderizado segue o andamento **da partitura** (não o do projeto); com mapas diferentes o
  clipe não acompanha a grade do projeto.
- `score_audition` renderiza a partitura inteira (sem trecho/loop); arquivos de render que deixam de
  ser usados após desfazer ficam em `audio/rendered/` até uma limpeza futura (ADR-0029).
- Importação GP3–GP6 continua dependendo da conversão na UI (ADR-0020).
