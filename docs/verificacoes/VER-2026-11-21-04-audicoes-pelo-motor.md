# VER-2026-11-21-04 — Audições de Harmonia, Bases e Partitura pelo motor de áudio (S-3 / AU-09)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-21-04 |
| Título | "Ouvir" da Harmonia, das Bases e da Partitura toca pelo motor de áudio, no fone escolhido no app |
| Gravidade | média (confunde o diagnóstico de "não ouço nada": o app toca num dispositivo e as audições em outro) |
| Tipo | correção |
| Data | 2026-11-21 |
| Usuário responsável | TokumoH (aprovou S-3 em 2026-11-21) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `crates/audio-engine/src/{preview.rs,core.rs,handle.rs,lib.rs}`, `crates/audio-engine/tests/{engine.rs,no_alloc.rs}`, `crates/app-core/src/{preview.rs,lib.rs}`, `crates/app-desktop/src/{commands_composition.rs,commands_music.rs,main.rs}`, `ui/src/core/{audition.ts,audition.test.ts}`, `ui/src/contexts/notation/ScorePanel.tsx`, `ui/src/ipc/{api.ts,schema.ts,mock.ts,mockComposition.ts,mockComposition.test.ts,mockMusic.ts,mockMusic.test.ts}`, `ui/e2e/score-entry.spec.ts`, `ui/src/app/helpTopics.ts`, docs (manual, spec da UI, relatório da varredura 6, READMEs, CHANGELOG) |

## Problema

- As audições recebiam um WAV do núcleo e tocavam pela **WebAudio** da janela. A WebAudio usa o dispositivo
  **padrão do sistema operacional**, não o "Fone / alto-falantes (saída)" escolhido em *Configurações de áudio*.
- Efeito: com o fone USB escolhido no app e as caixas como padrão do SO, o arranjo saía no fone e a audição nas
  caixas (ou o contrário). O usuário que "não ouve nada" não sabia qual saída conferir. A Partitura ainda criava
  um `AudioContext` próprio a cada clique (`ScorePanel.tsx`).

## Era proposital?

Não. A audição WAV → WebAudio da Fase 4/5 (`docs/specs/2026-11-05-ui-workspace.md`) foi um atalho de interface:
a engine ainda não tinha uma via para tocar áudio fora do grafo. O relatório da varredura 6
(`docs/relatorios/2026-11-17-varredura-6-audio.md`, AU-09) registrou o problema e a solução S-3.

## Solução

1. **Engine (`preview.rs`)**: `PreviewClip` (estéreo intercalado, já na taxa da engine) e `Preview`, que soma o
   trecho à saída do dispositivo **depois do master**, como o "Testar som". Não passa pela mixagem e não é
   gravado nem exportado. O trecho chega pré-alocado por `Message::PlayPreview`. Ao acabar, ser trocado ou
   parado (`Message::StopPreview`), ele volta pela fila de lixo (`Garbage::Preview`): **nada é alocado nem
   liberado no callback**.
2. **Aplicação (`app-core/src/preview.rs`)**: `Studio::play_preview_wav` decodifica o WAV que as audições já
   produziam (PCM 16 e float 32), ajusta à taxa da engine por interpolação linear e devolve a duração em
   segundos. `Studio::stop_preview` para.
3. **Comandos**: `harmony_audition`, `bases_generated_audition`, `bases_audition` e `score_audition` geram o WAV
   como antes, mas tocam no motor e devolvem `{ seconds }`. O novo comando `audition_stop` para.
4. **UI**:
   - `core/audition.ts` chama o comando e marca "tocando" pela duração mais 150 ms de margem. "Parar" chama
     `audition_stop`.
   - Se o usuário parar durante o preparo, a audição é parada de novo quando o motor começa.
   - A Partitura passa a usar o mesmo serviço (id `score:<id>`); o `AudioContext` próprio foi removido.

## Por que esta solução

- **Mesma saída para tudo**: o motor já é o único dono do dispositivo escolhido, com reabertura automática quando
  ele cai (VER-2026-11-17-11).
- **Reuso**: a geração dos WAVs não mudou; o WAV só deixa de cruzar o IPC (antes eram até alguns MB por clique).
- **Tempo real seguro**: o mesmo padrão do tom de teste e das trocas de grafo (mensagem + fila de lixo), coberto
  pelo teste `no_alloc`.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| `AudioContext.setSinkId` com o id do dispositivo | Os ids do cpal não são os ids da WebAudio; o WebView do Linux (WebKitGTK) não implementa `setSinkId` |
| Tocar a audição como clipe numa faixa temporária do grafo | Mexe no projeto (desfazer, mixagem, exportação) e no transporte; a audição deve ser independente |
| Decodificar e reamostrar dentro do callback | Alocação e custo variável no tempo real (viola R1) |
| Manter a WebAudio e só avisar "a audição usa o padrão do sistema" | Não resolve o problema e mantém dois caminhos de saída |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [crates/audio-engine/src/preview.rs](../../crates/audio-engine/src/preview.rs) | todo (novo) | `PreviewClip`, `Preview` (`play`/`stop`/`add_to`), 3 testes |
| [crates/audio-engine/src/core.rs](../../crates/audio-engine/src/core.rs#L44) | 44–45, 273–281, 374–375 | mensagens `PlayPreview`/`StopPreview`, `Garbage::Preview`, soma depois do tom de teste |
| [crates/audio-engine/src/handle.rs](../../crates/audio-engine/src/handle.rs#L159) | 159–167 | `play_preview`, `stop_preview` |
| [crates/audio-engine/tests/engine.rs](../../crates/audio-engine/tests/engine.rs) | fim | audição toca, termina e volta pelo lixo |
| [crates/audio-engine/tests/no_alloc.rs](../../crates/audio-engine/tests/no_alloc.rs#L72) | 72–76, 104–110 | audição tocando, trocada e parada sem alocar |
| [crates/app-core/src/preview.rs](../../crates/app-core/src/preview.rs) | todo (novo) | `decode_wav`, `resample_linear`, `play_preview_wav`, `stop_preview`, 2 testes |
| [crates/app-desktop/src/commands_composition.rs](../../crates/app-desktop/src/commands_composition.rs#L122) | 122–136, 213, 344, 380 | `AuditionPlay`, `play`, `audition_stop`; audições devolvem `{ seconds }` |
| [crates/app-desktop/src/commands_music.rs](../../crates/app-desktop/src/commands_music.rs#L176) | 176 | `score_audition` toca no motor |
| [crates/app-desktop/src/main.rs](../../crates/app-desktop/src/main.rs) | registro de comandos | `audition_stop` |
| [ui/src/core/audition.ts](../../ui/src/core/audition.ts) | todo | serviço pela engine (duração, Parar, parada durante o preparo) |
| [ui/src/core/audition.test.ts](../../ui/src/core/audition.test.ts) | todo (novo) | 6 testes do serviço |
| [ui/src/contexts/notation/ScorePanel.tsx](../../ui/src/contexts/notation/ScorePanel.tsx) | `stopAudio`/`audition` | usa o serviço; sem `AudioContext` próprio |
| [ui/src/ipc/api.ts](../../ui/src/ipc/api.ts) | audições, `auditionStop` | JSON `{ seconds }`; `callBinary` removido (sem uso) |
| [ui/src/ipc/schema.ts](../../ui/src/ipc/schema.ts) | `AuditionPlay` | esquema zod |
| [ui/src/ipc/mockComposition.ts](../../ui/src/ipc/mockComposition.ts), [mockMusic.ts](../../ui/src/ipc/mockMusic.ts), [mock.ts](../../ui/src/ipc/mock.ts) | audições | simulado devolve a duração; geradores de WAV de teste removidos (sem uso) |
| [ui/e2e/score-entry.spec.ts](../../ui/e2e/score-entry.spec.ts) | 1º teste | Ouvir → Parar → Ouvir na Partitura |

## Lógica

**Antes:** clique → núcleo gera o WAV → bytes pelo IPC → `decodeAudioData` → `AudioBufferSourceNode` →
**saída padrão do SO**.

**Depois:** clique → núcleo gera o WAV → `play_preview_wav` decodifica e ajusta a taxa (fora do tempo real) →
`PlayPreview` na fila → o callback soma o trecho depois do master → **saída escolhida no app**. A UI recebe só
`{ seconds }`. Ao acabar ou ao trocar, o trecho volta pelo lixo e é liberado na thread de controle.

## Exemplo

Projeto a 48 kHz, fone USB escolhido no app, caixas como padrão do Windows. A progressão "I V vi IV" tem
4 acordes × 4 tempos a 100 BPM: 9,6 s mais a cauda da audição. O núcleo gera o WAV na taxa do projeto
(`DEFAULT_AUDITION_RATE` = 48 kHz sem projeto aberto). Se a engine roda na mesma taxa, `resample_linear` devolve
os canais sem cópia. Se roda em outra (por exemplo, sem projeto aberto e a engine a 44,1 kHz), o trecho é
reamostrado: 9,6 s viram cerca de 423 360 quadros a 44,1 kHz. A UI recebe `{ seconds }` com essa duração, e o
botão volta a "Ouvir" 150 ms depois dela. O som sai no **fone USB**; antes saía nas caixas.

## Verificação

**Verificação 1**
- Rust: `cargo fmt --check` e `cargo clippy --workspace --all-targets -D warnings`. O clippy acusou um
  `Ok(…?)` desnecessário em `audition_stop`; foi corrigido e as verificações recomeçaram do zero.
- Testes `audio-engine` (60), `app-core` + `app-desktop` (94): todos passam.
- `no_alloc` em release, contrato IPC (fixtures sem mudança), `check_readmes.py` e `test_release_latest_json.py`:
  sem erros.
- UI: `tsc -b`, vitest (54 arquivos, 352 testes), `vite build` e Playwright completo (81 testes): sem erros.

**Verificação 2 (outra abordagem: sabotagens)**
- Tirar a nova parada do motor após "Parar" durante o preparo: `audition.test.ts` falha ("para o motor de novo").
- Zerar o canal direito em `Preview::add_to`: o teste da engine falha.
- Inverter a razão em `resample_linear`: o teste de duração/nível falha.
- Remover o guarda do temporizador (`playing === id`): nenhum teste falha, porque `clearTimer` já cancela o
  temporizador anterior em toda troca. O guarda é defesa extra e foi mantido.
- Arquivos restaurados (diff conferido). Depois: clippy do workspace e testes Rust de novo, sem erros.

## Riscos

- **Não verificado em hardware real** (o contêiner não tem placa de som): o caminho até o fone foi conferido pelo
  núcleo simulado e pelo dispositivo nulo.
- A audição sai **depois do master**: o fader master e os efeitos do master não a afetam, assim como o tom de
  teste. Isso é intencional, porque a audição não faz parte da mixagem.
- No modo silencioso (sem saída), a audição "toca" em silêncio. A barra de status já avisa o modo silencioso.
- A reamostragem linear pode gerar leve aliasing ao baixar a taxa. Para audição curta é aceitável; o S-1 (conversor
  de taxa do dispositivo) é independente disso.
- Uma audição durante a gravação também sai no fone, mas não entra na gravação.
