# VER-2026-11-10-04 — Loop configurável (opções, projeto inteiro, régua)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-10-04 |
| Título | Loop configurável: 4 compassos, projeto inteiro, compassos X–Y, desenhar e ajustar na régua |
| Gravidade | nenhuma (funcionalidade/navegação) |
| Tipo | funcionalidade |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH (pedido "navegação mais clara… loop"; mockup 10 aprovado; interpretação "loop no projeto inteiro" confirmada) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/core/bars.ts`, `ui/src/core/loop.ts` (novo), `ui/src/contexts/transport/LoopPopover.tsx` (novo), `TransportBar.tsx`, `transport.css`, `ui/src/contexts/project/Timeline.tsx`, `timeline.css`, `ui/src/app/Workspace.tsx`, `ui/src/app/menuModel.ts`, `ui/src/core/glossary.ts`, testes e docs |

## Problema

O loop só existia como "4 compassos" (botão começava sempre nos compassos 1–4; L a partir do cursor),
sem como mudar o trecho nem indicação de onde configurar. O usuário não achou como estender o loop.

## Era proposital?

Não. O loop fixo veio da Fase 2 (gravação), como atalho mínimo; não havia ADR limitando o trecho. O
botão e a tecla L usavam regras diferentes (1–4 × cursor) — inconsistência sem motivo registrado.

## Solução

- Botão **⟲ Loop · 1–8** (mostra o trecho) + **▾** (e *Transporte › Opções de loop…*) com: 4 compassos a
  partir do cursor, **projeto inteiro** (até o fim do compasso do último clipe) e compassos X–Y.
- **Régua:** arrastar na faixa de cima (10 px) desenha o loop; bordas arrastáveis ajustam; clique simples
  só posiciona o cursor.
- Botão, L, menu e régua usam as mesmas funções (`core/loop.ts`); religar volta ao último trecho.
- Aviso no painel: na gravação o loop é ignorado — usar punch para regravar trecho.

## Por que esta solução

Conta de compassos pura em `core/bars.ts`, testada contra a numeração da régua; estado compartilhado em
`core` porque transporte e arranjo são contextos diferentes (regra de arquitetura: contexto não importa
contexto — o teste de arquitetura acusou a primeira versão, que ficava em `contexts/transport`).

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Só arrastar na régua | Pouco descobrível (o usuário não achou nem o loop atual); o ▾ deixa explícito |
| Loop com take por volta na gravação | Muda a engine e o modelo de takes; fora do pedido (confirmado pelo usuário) |
| Reaproveitar a conversão do Mapa de tempo | Usa compassos fracionários e diverge da régua; registrada tarefa separada para unificar |
| Estado do loop em `contexts/transport` | Quebra a regra de contextos independentes (arranjo também usa) |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [core/bars.ts](../../ui/src/core/bars.ts#L50) | 50–92 | `barToTick`, `tickToBar`, `loopRangeBars`, `projectLoopRange`, `loopLabel` |
| [core/loop.ts](../../ui/src/core/loop.ts) | 1–50 | `useLoopUi`, `setLoop`, `toggleLoop` (lembra o último trecho) |
| [LoopPopover.tsx](../../ui/src/contexts/transport/LoopPopover.tsx) | todo | Opções de loop |
| [TransportBar.tsx](../../ui/src/contexts/transport/TransportBar.tsx) | botão Loop | Rótulo com o trecho + ▾ |
| [Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx) | régua | Desenhar/ajustar loop, bordas |
| [Workspace.tsx](../../ui/src/app/Workspace.tsx), [menuModel.ts](../../ui/src/app/menuModel.ts) | ações/menu | L e menu usam `core/loop`; item "Opções de loop…" |
| [loopRanges.test.ts](../../ui/src/contexts/project/loopRanges.test.ts), [loop.spec.ts](../../ui/e2e/loop.spec.ts) | todo | Testes |

## Lógica

Antes: botão → `[0, 4 compassos]`; L → 4 compassos do cursor; sem edição.
Depois: botão/L → desliga, ou religa `último ?? 4 compassos do cursor`; ▾ → escolha → `setLoop(faixa)`;
régua → `setLoop([min, max])` se ≥ 1 tempo, senão posiciona o cursor.

## Exemplo

Compassos 2 até 5 → `[3840, 19200]` tiques, botão "Loop · 2–5"; desligar/religar mantém 2–5 (com a
regra antiga voltaria a 1–4 — confirmado por sabotagem). Projeto com clipes até 5 compassos e 10 tiques
→ loop 1–6.

## Verificação

- **Verificação 1 (do zero, após mover para `core`):** `tsc`; Vitest 241/241; Playwright 51/51.
- **Verificação 2 (outra abordagem):** build de produção; arquitetura + `core` + `contexts/project`
  44/44; E2E de loop, ferramentas (atalho L) e camadas repetidos 2× (18/18).
- **Sabotagens:** sem lembrar o trecho → "Loop · 1–4" em vez de 2–5 (falha); bordas desmontando no
  arraste → o ajuste não era aplicado (falha observada antes da correção).
- Achados durante a verificação: (1) violação de arquitetura (contexto importando contexto) → movido
  para `core`; (2) teste clicava no compasso 8 fora da janela de 1280 px → corrigido para o 7.

## Riscos

- O loop continua sem efeito durante a gravação (decisão confirmada).
- Conversão de compassos do Mapa de tempo ainda diverge da régua em compassos cortados (tarefa sugerida).
