# VER-2026-11-11-01 — Varredura 4: popovers visíveis, renders, caches, CI e documentação

| Campo | Valor |
|---|---|
| Id | VER-2026-11-11-01 |
| Título | Correções da varredura 4 (17 objetivos) |
| Gravidade | alta (popovers inutilizáveis por pessoa) · média (renders, disco) · baixa (código morto, CI, docs) |
| Tipo | varredura |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH (pedido "faça uma varredura") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/contexts/transport/LoopPopover.tsx`, `transport.css`, `ui/src/contexts/project/ClipContextMenu.tsx`, `Timeline.tsx`, `timeline.css`, `ui/src/contexts/mixing/MixerPanel.tsx`, `ui/src/app/Workspace.tsx`, `ui/src/contexts/tuning/tuningMath.ts`, `ui/src/ipc/api.ts`, `crates/app-core/src/studio.rs`, `.github/workflows/*.yml`, `Cargo.lock`, `ui/package*.json`, testes novos, 14 READMEs, relatório |

## Problema

1. Opções de loop **cortadas** pela barra de transporte (`overflow: hidden`); menu do clipe **coberto**
   pelos painéis de baixo quando aberto na parte inferior do Arranjo.
2. Workspace, Mixer e Timeline (durante a reprodução) re-renderizavam a cada leitura de telemetria.
3. Caches em disco de áudios removidos nunca eram apagados; caches em memória sobreviviam à troca de projeto.
4. Código morto; jobs da CI sem limite de tempo; dependências com atualizações compatíveis; ~100 diretórios sem README.

## Era proposital?

- `overflow: hidden` na barra é proposital (evitar transbordo em janela estreita) — o erro foi desenhar o
  popover dentro dela. O menu do clipe já constava como risco em VER-2026-11-10-03.
- Assinar o objeto inteiro de medidores veio da Fase 1/2, antes da telemetria a 30 Hz; sem decisão registrada.
- A coleta de mídia (ADR de GC de fontes) cobria só `audio/*`; caches não foram considerados.

## Solução

- Popovers em **portal no `body`** com posição fixa presa à janela (`popoverPosition`, `menuPosition`, que vira
  para cima/esquerda); medição com `opacity: 0` (com `visibility: hidden` o foco não entrava — regressão
  encontrada na verificação e corrigida).
- **Assinaturas finas**: Workspace lê só `xruns`; `LiveMeter` por canal com `useShallow`; `Playhead` próprio.
- Abertura do projeto apaga caches cujo hash nenhuma fonte (projeto ou autosave) referencia; nomes
  desconhecidos não são tocados. Caches em memória limpos ao trocar/fechar o projeto.
- Removidos `NOTE_NAMES` e aliases `Api`/`CompositionApi`/`MusicApi`/`EditApi` (nunca usados).
- `timeout-minutes` por job (CI 60/30/20, CodeQL 45, release 90/10).
- Teste permanente: todo item de menu (nos dois temas) e todo atalho padrão têm ação.
- 24 atualizações compatíveis Rust e 5 npm (opener Rust/JS alinhados em 2.7.3).
- READMEs em `docs/`, `docs/*`, `docs/relatorios/img`, `scripts`, `assets`, `assets/bases`, `ui/e2e`, `ui/public`.

## Por que esta solução

Portal é a forma padrão de escapar de `overflow`/empilhamento sem tirar a proteção da barra; assinaturas
finas mantêm o mesmo visual com custo proporcional ao que muda; a coleta reaproveita o ponto onde a mídia
já é coletada (abertura, histórico vazio, autosaves considerados).

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Tirar `overflow: hidden` da barra | A barra voltaria a transbordar em janelas estreitas |
| `React.memo` nos canais do mixer | O `peak` muda a cada leitura: memo não evitaria o re-render do painel |
| Apagar a pasta `cache/` inteira ao abrir | Recalcular picos/espectrogramas de tudo a cada abertura |
| README completo em cada uma das ~80 pastas de código | Texto repetitivo; proposto modelo por crate/camada para decisão (relatório) |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [LoopPopover.tsx](../../ui/src/contexts/transport/LoopPopover.tsx) | topo, retorno | Portal, `popoverPosition`, `opacity` |
| [ClipContextMenu.tsx](../../ui/src/contexts/project/ClipContextMenu.tsx) | `menuPosition`, retorno | Portal e posição presa à janela |
| [Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx) | `Playhead`, `openMenu`, efeito de caches | Cursor isolado; coordenadas da tela; limpeza de caches |
| [MixerPanel.tsx](../../ui/src/contexts/mixing/MixerPanel.tsx) | `LiveMeter` | Medidor por canal |
| [Workspace.tsx](../../ui/src/app/Workspace.tsx) | `xruns` | Assinatura fina |
| [studio.rs](../../crates/app-core/src/studio.rs) | `collect_unreferenced_media`, `CACHE_DIRS`, `cache_hash` | Coleta de caches órfãos |
| [ci.yml](../../.github/workflows/ci.yml), [codeql.yml](../../.github/workflows/codeql.yml), [release.yml](../../.github/workflows/release.yml) | jobs | Limites de tempo |
| [commands.test.ts](../../ui/src/app/commands.test.ts), [layers.spec.ts](../../ui/e2e/layers.spec.ts), [loop.spec.ts](../../ui/e2e/loop.spec.ts), [popupPosition.test.ts](../../ui/src/contexts/project/popupPosition.test.ts), [LoopPopover.test.ts](../../ui/src/contexts/transport/LoopPopover.test.ts), [tests/studio.rs](../../crates/app-core/tests/studio.rs) | — | Testes novos |

## Lógica

Antes: popover filho da barra → recortado; leitura de telemetria → `setState` no store → componentes que
assinam o objeto inteiro re-renderizam. Depois: popover no `body`; só `LiveMeter`/`Playhead` assinam o que
muda; coleta de caches por hash junto da coleta de mídia.

## Exemplo

Contador temporário de renders (2 s): Workspace parado 61 → 0/s; Mixer parado 61 → 0/s; Timeline tocando
61 → 0/s (o cursor continua andando: posição 002.1.128). Popover de loop a 1024 px: 4 de 5 pontos cobertos
→ 0. Menu do clipe na faixa de baixo: 3 de 3 cobertos → 0.

## Verificação

- **Verificação 1:** fmt, clippy `-D warnings`, Rust 649/649 (workspace); `tsc`; Vitest 247/247; build;
  Playwright 55/55. Antes, `scripts/ci-local.sh rust` completo: 656 testes, simulação 11% do prazo.
- **Verificação 2 (outra abordagem):** simulação release (10% do prazo, 0 blocos acima, 0 alocações);
  contrato IPC 4/4 e fixtures inalteradas; E2E das correções repetidos 3× sem instabilidade (39/39).
- **Sabotagens:** sem portal → popover 4 pontos cobertos / menu 3 cobertos (falham); sem a ação
  `loopOptions` → teste de ligações falha nomeando-a; coleta de caches → teste falhava antes da implementação.
- Medições de render com contador temporário (removido; conferido `grep` = 0).
- **Após rebase sobre `4044b3d`** (Mapa de tempo, feito em outra sessão, só UI): verificações refeitas do
  zero — `tsc`; Vitest 254/254; Playwright 55/55; build; arquitetura + core + project + ligações 55/55;
  E2E de loop/camadas/workspace repetidos 2× (24/24). Rust inalterado pelo commit incorporado (0 arquivos `.rs`).

## Riscos

- READMEs dos diretórios de código pendentes de decisão (relatório).
- Avisos `cargo audit` transitivos (Tauri/GTK) seguem sem correção compatível upstream.
