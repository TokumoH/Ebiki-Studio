# AppMusica — Arquitetura Geral

- **Data:** 2026-11-05
- **Status:** Aprovada (seções 1 a 4)
- **Requisitos:** [`../requisitos/2026-11-05-levantamento-requisitos.md`](../requisitos/2026-11-05-levantamento-requisitos.md)
- **Decisões:** [`../adr/`](../adr/)

---

## 1. Visão de Contexto (C4 — Nível 1)

```mermaid
flowchart LR
    user([Músico / Usuário])
    app[AppMusica<br/>Estúdio desktop]
    dev[(Interface / placa de áudio<br/>ASIO · WASAPI · CoreAudio · JACK)]
    fs[(Sistema de arquivos<br/>projetos .amproj, áudios, soundfonts)]
    user -- grava, mixa, edita, compõe --> app
    app -- entrada / saída de áudio --> dev
    app -- lê / grava projetos e arquivos --> fs
```

## 2. Camadas (arquitetura hexagonal + DDD)

```mermaid
flowchart TB
    subgraph UI["UI — React + TypeScript"]
        comps[Componentes por contexto<br/>Design System · Painéis]
    end
    subgraph IF["Interface — Rust"]
        cmds[Comandos Tauri · DTOs · Mapeadores]
    end
    subgraph APP["Application"]
        uc[Casos de uso · CommandBus · History]
    end
    subgraph DOM["Domain — puro, sem I/O"]
        dom[Entidades · VOs · Agregados<br/>Eventos · PORTAS traits]
    end
    subgraph INF["Infrastructure — Pure Fabrication"]
        inf[AudioEngine · CodecService · SqliteRepository<br/>FileStorage · SoundfontSynth · DspLibrary]
    end
    UI -- IPC commands --> IF
    IF -- events / channels --> UI
    IF --> APP --> DOM
    INF -. implementa portas .-> DOM
    APP --> INF
```

**Regra de dependência:** todas as dependências apontam para o domínio; o domínio não depende de nenhuma outra camada.

## 3. Bounded Contexts

```mermaid
flowchart TB
    SK[[Shared Kernel<br/>Tick · BPM · TempoMap · Pitch · IDs · unidades de áudio]]
    subgraph Produção
        P[Project<br/>projeto, faixas, mapa de tempo]
        R[Recording<br/>captura, takes, metrônomo]
        M[Mixing<br/>mixer, buses, sends, efeitos, medidores]
        E[Editing<br/>clipes, fades, operações]
        L[Library<br/>bases, sons, metadados]
    end
    subgraph Música
        H[Harmony<br/>campos, progressões, sugestões]
        S[Sync<br/>letras, time-stretch]
        N[Notation<br/>partitura, tablatura, playback]
        T[Tuning<br/>afinador, correção de pitch]
    end
    SK --- P & R & M & E & L & H & S & N & T
```

| Contexto | Responsabilidade | Agregados principais (preliminar) |
|---|---|---|
| Project | Projeto, faixas, mapa de tempo, fontes de áudio | `Project`, `Track`, `TempoMap` |
| Recording | Gravação, takes, punch, metrônomo | `RecordingSession`, `TakeLane` |
| Mixing | Mixer, roteamento, efeitos, medição | `MixerGraph`, `EffectChain` |
| Editing | Clipes, fades, operações não destrutivas | `ClipArrangement` |
| Library | Bases instrumentais e metadados | `Base`, `Catalog` |
| Harmony | Campos harmônicos, progressões, sugestões | `HarmonicField`, `Progression` |
| Sync | Letra sincronizada, alinhamento, time-stretch | `LyricTrack`, `WarpMap` |
| Notation | Partitura/tablatura, edição, reprodução | `Score` |
| Tuning | Afinação e correção de pitch | `PitchCorrectionMap` |

**Comunicação entre contextos:** somente por **eventos de domínio** e **IDs**; nunca referência direta a agregados de outro contexto. Import/Export é **infraestrutura** (`CodecService`), não contexto.

### Pontos de extensão (portas)

| Porta (trait) | Implementação inicial | Extensão futura |
|---|---|---|
| `SuggestionProvider` | `RuleBasedHarmonySuggester` | IA de composição |
| `MixAdvisor` | `RuleBasedMixAdvisor` | IA de mixagem |
| `EffectProcessor` | Efeitos nativos | Host VST3 / CLAP |
| `ParameterSource` | Valor constante | Curvas de automação |
| `PitchDetector` | YIN / MPM | Modelo de IA |
| `AudioBackend` | `cpal` | Bindings diretos por SO |

## 4. Organização do Repositório

```
AppMusica/
├── crates/                     # Cargo workspace
│   ├── shared-kernel/
│   ├── ctx-project/            # domain + application de cada contexto
│   ├── ctx-<contexto>/         # criados conforme as fases
│   ├── audio-engine/           # Pure Fabrication: engine em tempo real
│   ├── infra-persistence/      # SQLite, repositórios, migrações
│   ├── infra-codecs/           # import / export
│   └── app-desktop/            # Tauri: comandos, composition root
├── ui/src/{shared,contexts,panels,app,ipc}
├── docs/{requisitos,arquitetura,adr,specs,diagramas,manual}
└── .github/workflows/
```

Um crate por contexto faz o **compilador** garantir a independência entre contextos.

```mermaid
flowchart BT
    SK[shared-kernel]
    CP[ctx-project] --> SK
    CX[ctx-...] --> SK
    AE[audio-engine] --> SK
    IP[infra-persistence] --> CP
    IC[infra-codecs] --> SK
    AD[app-desktop] --> CP & CX & AE & IP & IC
```

---

## 5. Engine de Áudio

### 5.1 Modelo de Threads

```mermaid
flowchart LR
    UI[UI WebView] -- IPC commands --> CT[Control Thread<br/>Application + Domain]
    CT -- events ~30 Hz / channels --> UI
    CT -- SPSC lock-free:<br/>RenderGraph, parâmetros --> AT[AUDIO THREAD<br/>callback tempo real]
    AT -- ring buffer:<br/>medidores, posição, xruns --> CT
    DR[Disk Reader Threads] -- ring buffers --> AT
    AT -- ring buffers --> DW[Disk Writer Thread<br/>WAV float32]
    AN[Analysis Thread<br/>picos, espectrograma, LUFS] -.-> CT
```

**Regra de ouro da Audio Thread:** sem alocação, sem `lock`, sem I/O, sem log, sem `panic`. Verificado em CI (`assert_no_alloc`).

### 5.2 Grafo de Áudio

```mermaid
flowchart LR
    IN[InputNode] --> MON[Monitor]
    C1[ClipPlayer faixa 1] --> FX1[EffectChain] --> F1[Fader/Pan]
    C2[ClipPlayer faixa 2] --> FX2[EffectChain] --> F2[Fader/Pan]
    C3[ClipPlayer faixa 3] --> FX3[EffectChain] --> F3[Fader/Pan]
    F1 --> BV[Bus Vozes] 
    F2 --> BV
    F3 -- send --> BR[Bus Reverb FX]
    F3 --> MS
    BV --> MS[Master] --> LIM[Limiter] --> OUT[Saída]
    BR --> MS
    MON --> OUT
    MET[MetronomeNode] --> OUT
```

- Domínio descreve a mixagem → Control Thread **compila** um `RenderGraph` plano (ordem topológica).
- Troca **atômica** do grafo na Audio Thread; grafo antigo devolvido para desalocação fora do tempo real.
- Parâmetros via atômicos com suavização (sem recompilar o grafo).
- Todo nó implementa `AudioNode::process(&mut self, ctx, buffers)`.

### 5.3 Backends

| SO | API | Biblioteca |
|---|---|---|
| Windows | ASIO (preferencial), WASAPI exclusivo | `cpal` (feature `asio`) |
| macOS | CoreAudio | `cpal` |
| Linux | JACK / PipeWire, ALSA | `cpal` |

- Buffer 64 amostras @ 48 kHz = 1,33 ms por bloco; ida e volta estimada 5–10 ms (ASIO/CoreAudio).
- **Buffer adaptativo:** detecção de xruns → sugere/aumenta (64 → 128 → 256). Perfis "Gravação" (mínimo) e "Mixagem" (maior).
- **Compensação de latência:** medição de ida e volta para alinhar gravações; PDC preparado.

### 5.4 Formatos Internos

- Áudio `f32` não intercalado; posição em amostras `u64`.
- Tempo musical em `Tick` (960 PPQ); conversão via `TempoMap` (BPM e compasso variáveis).
- Import: decodificação `symphonia`, conversão de taxa `rubato`; original preservado.

### 5.5 Máquina de Estados do Transporte

```mermaid
stateDiagram-v2
    [*] --> Stopped
    Stopped --> Playing: play
    Stopped --> CountIn: record (com contagem)
    Stopped --> Recording: record (sem contagem)
    CountIn --> Recording: contagem concluída
    CountIn --> Stopped: stop
    Playing --> Paused: pause
    Paused --> Playing: play
    Paused --> Stopped: stop
    Playing --> Recording: punch-in / record
    Recording --> Playing: punch-out
    Playing --> Stopped: stop
    Recording --> Stopped: stop
    Playing --> Stopped: dispositivo desconectado
    Recording --> Stopped: dispositivo desconectado
```

### 5.6 Testabilidade

- **Modo offline:** engine processa sem dispositivo; testes determinísticos amostra a amostra.
- `assert_no_alloc` no callback em CI.
- Benchmarks `criterion` (custo por bloco com 32 faixas).

---

## 6. Persistência

### 6.1 Bancos

| Banco | Local | Conteúdo |
|---|---|---|
| `project.db` | Pasta do projeto | Faixas, clipes, mapa de tempo, mixer, takes, letras, partituras |
| `library.db` | Dados do usuário no SO | Harmonia, catálogo de bases, presets, preferências, atalhos, recentes |

### 6.2 Pasta do Projeto

```
MinhaMusica.amproj/
├── project.db                  # SQLite WAL
├── audio/imported/<sha256>.<ext>
├── audio/recorded/<uuid>.wav
├── cache/                      # regenerável
└── backups/                    # autosave rotativo
```

### 6.3 Normalização e Leitura

- Mínimo **3FN/FNBC**; **4FN** em multivalorados (tabelas associativas).
- **Nada derivável é armazenado** — progressões em **graus relativos**; acordes do campo derivados de escala + grau.
- Índices em FKs e filtros; **views** para leituras frequentes.
- Desnormalização apenas como **tabela de leitura regenerável**, após medição e aprovação.
- Projeto carregado em memória; banco é persistência.

### 6.4 DER Preliminar — Fase 0 (`project.db`)

```mermaid
erDiagram
    PROJECT ||--o{ TEMPO_EVENT : possui
    PROJECT ||--o{ TIME_SIGNATURE_EVENT : possui
    PROJECT ||--o{ AUDIO_SOURCE : possui
    PROJECT ||--o{ TRACK : possui
    TRACK ||--o{ CLIP : contem
    AUDIO_SOURCE ||--o{ CLIP : referenciada_por
    TRACK ||--o{ SEND : origem
    TRACK ||--o{ SEND : destino
    TRACK |o--o{ TRACK : saida_para

    PROJECT {
        int id PK
        text name
        int sample_rate
        int bit_depth
        text created_at
        text updated_at
        int schema_version
    }
    TEMPO_EVENT {
        int id PK
        int project_id FK
        int tick "UNIQUE(project_id,tick)"
        real bpm
    }
    TIME_SIGNATURE_EVENT {
        int id PK
        int project_id FK
        int tick "UNIQUE(project_id,tick)"
        int numerator
        int denominator
    }
    AUDIO_SOURCE {
        int id PK
        int project_id FK
        text content_hash UK
        text file_path
        text format
        int sample_rate
        int channels
        int length_frames
        text original_name
    }
    TRACK {
        int id PK
        int project_id FK
        text name
        text kind "audio|bus|master"
        int position
        real volume_db
        real pan
        int muted
        int soloed
        int output_track_id FK
    }
    CLIP {
        int id PK
        int track_id FK
        int audio_source_id FK
        int start_tick
        int source_offset_frames
        int length_frames
        real gain_db
    }
    SEND {
        int id PK
        int from_track_id FK
        int to_track_id FK
        real level_db
        int pre_fader
    }
```

### 6.5 Tecnologia

| Item | Escolha |
|---|---|
| Driver | `rusqlite` |
| Migrações | `refinery` (versão por arquivo de projeto) |
| Padrão | Repository por agregado (trait no domínio, impl na infraestrutura) |
| Autosave | Transação + `VACUUM INTO` (backup rotativo) |
| Undo/Redo | Em memória (não persistido) |

---

## 7. UI, IPC e Undo/Redo

### 7.1 Canais IPC

| Canal | Direção | Uso | Formato |
|---|---|---|---|
| Commands | UI → Rust | Intenções do usuário | JSON tipado |
| Events | Rust → UI | Eventos de domínio | JSON tipado |
| Channels | Rust → UI | Posição, medidores, picos | Binário (`ArrayBuffer`) |

Tipos TS gerados do Rust via `tauri-specta`. O núcleo Rust é a **fonte da verdade**; a UI mantém uma projeção.

### 7.2 Fluxo de um Comando com Undo

```mermaid
sequenceDiagram
    participant UI
    participant IPC as Comando Tauri
    participant Bus as CommandBus
    participant Cmd as MoveClipCommand
    participant Dom as Domínio (Project)
    participant Eng as AudioEngine
    participant Hist as History

    UI->>IPC: move_clip(clipId, novoTick)
    IPC->>Bus: dispatch(MoveClipCommand)
    Bus->>Cmd: execute()
    Cmd->>Dom: clip.move_to(novoTick)
    Dom-->>Bus: [ClipMoved]
    Bus->>Hist: push(cmd)
    Bus->>Eng: recompila RenderGraph
    Bus-->>UI: event ClipMoved
    Note over UI,Hist: Ctrl+Z
    UI->>IPC: undo()
    IPC->>Hist: pop()
    Hist->>Cmd: undo()
    Cmd->>Dom: clip.move_to(tickAnterior)
    Bus-->>UI: event ClipMoved
```

- Todo comando mutável passa pelo `CommandBus`.
- Gestos contínuos geram **um** item no histórico (comando composto).
- Ações sem alteração (play, zoom) não entram no histórico.

### 7.3 Componentes da UI

```mermaid
flowchart TB
    APP[app/ — shell: docking, tema, atalhos]
    PAN[panels/ — TimelinePanel, MixerPanel, ScorePanel...]
    subgraph CTX[contexts/ — um por Bounded Context]
        CP[project]
        CM[mixing]
        CN[notation]
        CO[...]
    end
    SH[shared/ — Design System: tokens, Button, Knob, Fader, Meter]
    IPC[ipc/ — bindings gerados]
    APP --> PAN --> CTX --> SH
    CTX --> IPC
```

**Regra:** um contexto da UI nunca importa outro; composição apenas em `panels/`.

| Necessidade | Escolha |
|---|---|
| Estado | Zustand (store por contexto) |
| Docking | `dockview` |
| Timeline / waveform | Canvas 2D (WebGL se necessário) |
| Partitura / tablatura | alphaTab |
| Tema | CSS custom properties; escuro padrão |
| Atalhos | `KeymapService` persistido em `library.db` |

### 7.4 Tratamento de Erros

- Erros de domínio tipados (`thiserror`) → código no DTO → mensagem PT-BR.
- Dispositivo desconectado → transporte para, UI oferece reconexão.
- Crash → recuperação do último autosave.
- Audio Thread nunca entra em `panic`; nó com falha emite silêncio e telemetria.

### 7.5 Testes de UI

- Vitest + Testing Library (componentes isolados).
- Storybook (catálogo do design system e revisão de mockups).
- E2E com `tauri-driver` (Linux/Windows na CI).

---

## 8. Roteiro de Fases

```mermaid
flowchart LR
    F0[0 Fundação] --> F1[1 Captura]
    F0 --> F2[2 Mixagem]
    F0 --> F3[3 Edição]
    F0 --> F4[4 Bases]
    F5[5 Harmonia]
    F0 --> F6[6 Sincronização]
    F3 --> F6
    F0 --> F7[7 Notação]
    F5 --> F7
    F0 --> F8[8 Afinação]
    F1 --> F8
    F0 --> F9[9 Formatos]
    F2 --> F9
```
