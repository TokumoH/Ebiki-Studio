# VER-2026-11-10-03 — Zoom pela roda, altura das faixas e camadas acima do dockview

| Campo | Valor |
|---|---|
| Id | VER-2026-11-10-03 |
| Título | Zoom pela roda (tempo e altura das faixas) + correção de camadas sobrepostas |
| Gravidade | média (a correção de camadas: cliques bloqueados em diálogos/menus) · nenhuma (zoom: funcionalidade) |
| Tipo | funcionalidade + correção |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH (pedido e aprovação do mockup 10; interpretação "tamanho da onda = altura das faixas" confirmada) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/contexts/project/timelineView.ts`, `Timeline.tsx`, `timeline.css`, `timelineView.test.ts`, `ui/src/shared/MenuBar.tsx`, `ui/src/shared/tokens.css`, `ui/src/shared/ui.css`, `ui/src/app/app.css`, `ui/src/core/glossary.ts`, `ui/src/app/helpTopics.ts`, `ui/e2e/timeline-zoom.spec.ts`, `ui/e2e/layers.spec.ts`, docs |

## Problema

1. Pedido: ampliar/reduzir a onda com a roda do mouse, como em outros estúdios. Só havia Ctrl+=/−
   (zoom no tempo) e altura fixa de 72 px por faixa.
2. Achado na verificação: o divisor de painéis do dockview (z-index 99–9999) ficava **acima** de
   diálogos (50), menus (60), avisos (60) e tutorial (80). Um botão alinhado ao divisor não recebia o
   clique (a central de ajuda falhou quando o texto cresceu uma linha e o botão caiu sobre o divisor).

## Era proposital?

Não. A altura fixa vinha da Fase 1 (`TRACK_H = 72`, sem ADR); os z-index do app foram escolhidos
sem considerar os do dockview (adotado no ADR de painéis encaixáveis).

## Solução

- **Ctrl/⌘+roda** (e pinça): zoom no tempo **centrado no mouse** (o tique sob o ponteiro fica parado);
  **Alt+roda**: altura de todas as faixas; **Shift+roda**: rolagem lateral. Ouvinte nativo não passivo
  (bloqueia o zoom da página do webview).
- **Altura por faixa**: alça na borda de baixo do cabeçalho (arrastar ou ↑/↓); a onda acompanha.
- Arrastar clipe entre faixas usa as alturas reais (`trackIndexAt`).
- Soltar Alt depois de Alt+roda não abre o menu principal.
- **Camadas:** escala `--z-menu … --z-tooltip` (10000–10040) acima do dockview.

## Por que esta solução

Funções puras (`wheelFactor`, `anchoredScrollLeft`, `trackIndexAt`, `clampHeight`) concentram a conta
e são testadas; a timeline só liga eventos a elas. Alturas ficam no estado de visualização (não no
projeto), como o zoom.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| `onWheel` do React | É passivo: não impede o zoom da página com Ctrl+roda |
| Zoom centrado na borda esquerda | O ponto de interesse "foge" da tela; estúdios centram no mouse |
| Altura só global | Pedido de mockup inclui ajustar uma faixa (ex.: voz maior que o resto) |
| Salvar alturas no projeto | Preferência de visualização; mudaria o banco do projeto sem necessidade |
| Subir z-index só do diálogo | Menus, avisos e tutorial tinham o mesmo problema; escala única evita novas inversões |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [timelineView.ts](../../ui/src/contexts/project/timelineView.ts#L15) | 1–90 | Funções puras e estado de alturas/zoom |
| [Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx#L171) | 19–55, 171–200, 224, lanes | Roda, âncora do zoom, `TrackResizer`, alturas por faixa, arraste |
| [timeline.css](../../ui/src/contexts/project/timeline.css) | fim | Alça de altura |
| [MenuBar.tsx](../../ui/src/shared/MenuBar.tsx#L70) | 64–76 | Roda zera o "Alt sozinho" |
| [tokens.css](../../ui/src/shared/tokens.css), [ui.css](../../ui/src/shared/ui.css), [app.css](../../ui/src/app/app.css) | z-index | Escala de camadas |
| [timelineView.test.ts](../../ui/src/contexts/project/timelineView.test.ts), [timeline-zoom.spec.ts](../../ui/e2e/timeline-zoom.spec.ts), [layers.spec.ts](../../ui/e2e/layers.spec.ts) | todo | Testes |

## Lógica

Zoom: antes, Ctrl+roda dava zoom na página inteira (webview). Depois: `fator = e^(−Δy·0,0015)`,
`pxPorBatida = clamp(px·fator)`, e `scrollLeft = tique_sob_mouse × pxPorTique_novo − distância_do_mouse`.
Arraste: antes `faixa = origem + round(Δy/72)`; depois `faixa = trackIndexAt(alturas, y_no_conteúdo)`.

## Exemplo

Mouse sobre o compasso 3 e Ctrl+roda para cima: o compasso fica mais largo e continua sob o mouse
(desvio < 6 px no E2E). Com a faixa 1 em ~192 px, arrastar um clipe 130 px para baixo **fica** na faixa
1; a conta antiga (130/72 → 2) o jogava para a faixa 2.

## Verificação

- **Verificação 1 (do zero, após a última mudança):** `tsc` limpo; Vitest 234/234; Playwright 48/48.
- **Verificação 2 (outra abordagem):** build de produção; E2E focados (zoom, camadas, identidade,
  cobertura de legendas) repetidos 2× sem instabilidade (22/22).
- **Sabotagens:** sem o reset da roda no MenuBar → falha "foco no menu" (esperado false, veio true);
  conta antiga de 72 px no arraste → falha a contagem de clipes; sem a escala de camadas → 3 divisores
  cobrindo o diálogo/menus.
- Correção documental: [VER-2026-11-10-02](VER-2026-11-10-02-legendas.md) dizia 45 E2E; eram 44
  versionados (a execução incluía uma sonda temporária). Anotado lá.

## Riscos

- O menu de contexto do clipe (`.clip-menu`, posicionado dentro do painel) pode ficar sob o divisor se
  aberto exatamente na borda inferior do Arranjo; não coberto pelo teste de camadas.
- Alturas e zoom voltam ao padrão ao reabrir o app (não são salvas).
