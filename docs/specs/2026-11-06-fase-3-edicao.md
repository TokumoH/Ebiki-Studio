# Fase 3 — Edição / Pós-produção (especificação)

- **Data:** 2026-11-06 · **Requisitos:** RF-3.1 a RF-3.5, RNF-07 (não destrutivo)
- **Decisão arquitetural:** [ADR-0031](../adr/0031-edicao-nao-destrutiva-de-clipes.md)

## 1. Escopo

| Requisito | Entrega |
|---|---|
| RF-3.1 Cortar, dividir, mover | `TrimClip` (bordas), `SplitClip` (no cursor, atalho **S**), `MoveClip` (já existia) |
| RF-3.2 Fades e crossfades | `SetClipFades` (linear / equal power), `Crossfade` (sobreposição parcial na mesma faixa) |
| RF-3.3 Normalizar, ganho, fase, reverter | `clip_normalize` (pico → −1 dBFS via `SetClipGain`), slider de ganho, `SetClipPolarity`, `SetClipReversed` |
| RF-3.4 Efeitos por clipe/faixa | Por faixa: já entregue na Fase 2 (inserts). Por clipe: fora desta entrega (ver §8) |
| RF-3.5 Waveform + espectrograma selecionável | Alternância por faixa (≋); STFT com cache em disco; binário `AMSG` |

RNF-07: nenhuma operação desta fase escreve em `audio/`. Cortar/dividir mudam só a janela do clipe;
fades, polaridade e reverso são aplicados pela engine na reprodução/exportação.

## 2. Modelo de domínio (contexto Project)

```mermaid
classDiagram
  class Clip {
    ClipId id
    AudioSourceId source_id
    Tick start
    u64 offset_frames
    u64 length_frames
    Db gain
    ClipEdit edit
    source_end() u64
  }
  class ClipEdit {
    <<Value Object>>
    u64 fade_in_frames
    u64 fade_out_frames
    FadeShape fade_in_shape
    FadeShape fade_out_shape
    bool polarity_inverted
    bool reversed
    validate(length)
    clamped_to(length)
  }
  class FadeShape {
    <<Shared Kernel>>
    Linear
    EqualPower
    gain(t) f32
  }
  Clip *-- ClipEdit
  ClipEdit --> FadeShape
  class Project {
    replace_clip(Clip) Clip
    trimmed_clip(id, start, length) Clip
    split_parts(id, at, new_id) (Clip, Clip)
    crossfade_parts(a, b, shape) (Clip, Clip)
  }
  Project o-- Clip
```

Invariantes: `length > 0`; `offset + length ≤ source.length` (sem overflow); `fade_in + fade_out ≤ length`;
`start ≥ 0`. Comandos: `TrimClip`, `SplitClip`, `SetClipFades`, `SetClipPolarity`, `SetClipReversed`,
`Crossfade` — todos com `apply`/`revert` exatos e falha atômica (estado intacto).

### Semântica do reverso
A amostra `k` do clipe (0 ≤ k < length) lê a fonte em `offset + length − 1 − k`. Logo a borda
esquerda da timeline corresponde ao **fim** da região na fonte: cortar à esquerda reduz o fim da
região; dividir um clipe reverso dá à 1ª parte a região final da fonte.

### Curvas
`t ∈ [0,1]`: linear `g = t`; equal power `g = sin(t·π/2)`. Fade de entrada usa `t = k/fade_in`;
saída `t = (length − k)/fade_out`. Num crossfade linear a soma de amplitudes é 1; em equal power, a soma
de potências é 1 (testado).

## 3. Sequência — dividir no cursor

```mermaid
sequenceDiagram
  actor U as Usuário
  participant UI as Timeline / atalho S
  participant IPC as clip_split (Tauri)
  participant ST as Studio (edit_service)
  participant H as History
  participant P as Project (SplitClip)
  participant C as compile()
  participant E as EngineCore
  U->>UI: S (cursor dentro do clipe)
  UI->>UI: canSplitAt(clip, tick)?
  UI->>IPC: invoke("clip_split", {clipId, atTick})
  IPC->>ST: clip_split(id, Tick)
  ST->>H: execute(on_project(SplitClip))
  H->>P: apply → split_parts(id, at, new_id)
  P-->>H: replace_clip(1ª) + insert_clip_after(2ª)
  ST->>C: sync() → novo RenderGraph
  C->>E: swap_graph (fila lock-free)
  ST-->>UI: SessionView (2 clipes)
  U->>UI: Ctrl+Z
  UI->>ST: undo → revert: remove 2ª, restaura original
```

## 4. Estados — edição de um clipe na UI

```mermaid
stateDiagram-v2
  [*] --> Ocioso
  Ocioso --> Selecionado: clique
  Selecionado --> Movendo: arrastar corpo
  Selecionado --> Cortando: arrastar borda (ou ←/→ na borda focada)
  Selecionado --> AjustandoFade: arrastar alça de canto (ou ←/→)
  Selecionado --> MenuAberto: botão direito / tecla Menu / Shift+F10
  Movendo --> Selecionado: soltar (endGesture)
  Cortando --> Selecionado: soltar (endGesture)
  AjustandoFade --> Selecionado: soltar (endGesture)
  MenuAberto --> Selecionado: Esc / ação (Dividir, Normalizar, Fase, Reverter, Crossfade)
  Selecionado --> Selecionado: S divide no cursor
  Selecionado --> Ocioso: clicar fora / Delete
```

Cada arraste usa um `gesture` → **um único item de desfazer**.

## 5. Componentes e serviços

```mermaid
flowchart LR
  subgraph UI["UI (contexts/project)"]
    TL[Timeline] --> CV[ClipView]
    TL --> CM[ClipContextMenu]
    CI[ClipInspector - painel Clipe]
    CA[clipActions] --> EA
    CE[clipEdit - regras puras]
    CV --> SC[spectrogramCache]
  end
  EA[ipc/editApi] -->|invoke / invokeBinary| CMD[commands_edit.rs]
  CMD --> ES[app-core edit_service]
  ES --> PR[ctx-project commands]
  ES -->|MediaPort::spectrogram| MED[infra-adapters CodecsMedia]
  MED --> SPEC[infra-codecs spectrogram]
  ES --> COMP[compile] --> ENG[audio-engine ClipRender]
  PR --> REPO[infra-persistence v3]
```

## 6. Contratos IPC

| Comando | Argumentos (camelCase) | Resposta |
|---|---|---|
| `clip_trim` | `clipId, start (tick), lengthFrames, gesture?` | `SessionView` |
| `clip_split` | `clipId, atTick` | `SessionView` |
| `clip_set_fades` | `clipId, fades {fadeInFrames, fadeOutFrames, fadeInShape, fadeOutShape}, gesture?` | `SessionView` |
| `clip_set_polarity` | `clipId, inverted` | `SessionView` |
| `clip_set_reversed` | `clipId, reversed` | `SessionView` |
| `clip_crossfade` | `aId, bId, shape` | `SessionView` |
| `clip_normalize` | `clipId, targetDbfs?` (−60..0; padrão −1) | `SessionView` |
| `source_spectrogram` | `sourceId, params? {fftSize, hop, bins}` | binário `AMSG` |

`ClipView` ganhou `sourceFrames` e os campos de `ClipEdit`. Erros novos: `project.invalid_fade`,
`project.invalid_split`, `project.clips_do_not_overlap`, `edit.silent_clip`, `media.unsupported`.

### Formato `AMSG` (little-endian)
Cabeçalho de 40 bytes: magic, versão (1), bins (u16), FFT (u32), hop (u32), taxa (u32), colunas (u32),
quadros totais (u64), FNV-1a 64 do payload; depois `colunas × bins` bytes (grave → agudo).
Padrão: FFT 2048, hop 512, 128 faixas log de 20 Hz a Nyquist; −100..0 dBFS → 0..255.

## 7. Persistência (DER — tabela `clip`, schema 3)

```mermaid
erDiagram
  track ||--o{ clip : contem
  audio_source ||--o{ clip : referencia
  clip {
    TEXT id PK
    TEXT track_id FK
    TEXT source_id FK
    INTEGER position
    INTEGER start_tick
    INTEGER offset_frames
    INTEGER length_frames
    REAL gain_db
    INTEGER fade_in_frames "v3, >= 0"
    INTEGER fade_out_frames "v3, >= 0"
    TEXT fade_in_shape "v3, linear|equalPower"
    TEXT fade_out_shape "v3, linear|equalPower"
    INTEGER polarity_inverted "v3, 0|1"
    INTEGER reversed "v3, 0|1"
  }
```

Migração `project_v3.sql` (`ALTER TABLE … ADD COLUMN … DEFAULT`): projetos antigos abrem sem
processamento (padrões = comportamento anterior). Fades incoerentes vindos do banco são recusados
pela restauração do domínio (`persistence.corrupt`).

## 8. Modelo visual (wireframe registrado — modo autônomo)

```
┌ voz ─ M S ⏺ ≋ ─┐┌───────────────────────── clipe selecionado ────────────────────────┐
│                 │▌▪╲                         ∿∿∿ forma de onda ∿∿∿               ╱▪▐ │
│                 │▌  ╲__ curva fade-in                         curva fade-out __╱   ▐ │
└─────────────────┘└▲ borda: cortar início   ▪ alças de fade (cantos)   borda: cortar fim ▲┘
 ≋ = alterna forma de onda / espectrograma da faixa · botão direito = menu do clipe
 Painel "Clipe": [Ganho ────●──] [Normalizar] [Inverter fase] [Reverter]
                 Fade entrada (ms) [ 250 ] (Linear|Equal power) · Fade saída …
                 [Dividir no cursor] [Crossfade]
```

## 9. Fora desta entrega / próximos passos
- **Efeitos por clipe** (RF-3.4 parte "por clipe"): exigiria cadeia de inserts por clipe na engine;
  hoje efeitos são por faixa/bus. Proposta: renderizar como fonte derivada (ADR-0029) ou criar
  `ClipFx` com processadores pré-alocados — a decidir em ADR próprio.
- Espectrograma usa a fonte inteira (cache por conteúdo); zoom vertical/linear não exposto na UI.
- Crossfade automático ao soltar um clipe sobre outro (hoje é ação explícita).
