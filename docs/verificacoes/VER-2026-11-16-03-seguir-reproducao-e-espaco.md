# VER-2026-11-16-03 — Seguir reprodução (S2) e Espaço após clique (UX-01)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-16-03 |
| Título | A vista segue a linha de reprodução; Espaço/Enter não reacionam o último botão clicado |
| Gravidade | alta (pedido do usuário; UX-01 quebrava o atalho mais usado) |
| Tipo | funcionalidade + correção |
| Data | 2026-11-16 |
| Usuário responsável | TokumoH (S2 aprovada com o mockup 11; "corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/core/follow.ts` (novo), `follow.test.ts` (novo), `ui/src/contexts/project/Timeline.tsx`, `ui/src/contexts/transport/TransportBar.tsx`, `ui/src/app/{Workspace.tsx,menuModel.ts,shortcuts.ts,helpTopics.ts}` e testes, `ui/src/core/glossary.ts`, `ui/e2e/follow.spec.ts` (novo), `ui/e2e/recording-input.spec.ts`, `docs/manual/README.md`, READMEs de `ui/src/core` e `ui/e2e` |

## Problema

1. **S2:** ao tocar, a linha de reprodução saía da tela e o usuário precisava rolar a vista à mão.
2. **UX-01:** depois de clicar num botão (Metrônomo, M/S, Exportar…), ele ficava com o foco. Espaço e Enter, que o atalho
   global cedia a qualquer botão focado, **acionavam o botão de novo**: desligavam o metrônomo e reabriam o Exportar.

## Era proposital?

- **S2:** não. Não havia código nem decisão sobre isso (VER-2026-11-16-01).
- **UX-01:** a regra "Enter/Espaço pertencem ao controle focado" (`isActivationKey`) é proposital: protege quem usa o
  teclado. O erro foi aplicá-la também ao foco que veio de um clique. Nos DAWs, Espaço é sempre o transporte.

## Solução

- `core/follow.ts`:
  - `followScrollLeft(px, scrollLeft, width)`, função pura: vira a página quando a linha passa de 90% ou fica antes da
    vista, e deixa a linha a 10% da borda;
  - `isRolling`;
  - estado `enabled` (ligado por padrão, lembrado) e `suspended`.
- `Timeline.tsx` assina a telemetria com `useLive.subscribe`, fora do render, e rola pela vista. A rolagem feita pelo app
  (seguir, zoom ancorado) é marcada para não contar como manual. Rolagem manual durante a reprodução pausa o seguir; um
  novo ▶ ou a linha de volta à vista retomam.
- Botão **⇥ Seguir** no transporte, *Exibir › Seguir reprodução*, tecla **F**, legenda, ajuda e manual.
- UX-01: `notePointerDown`/`noteKeyboardNavigation` lembram se o foco veio do mouse. `isActivationKey` só cede
  Espaço/Enter ao controle quando o foco veio do teclado (Tab, setas).

## Por que esta solução

- Virar a página é o padrão do Reaper e do Audacity e não re-renderiza a timeline a 30 Hz.
- Distinguir foco do mouse e do teclado mantém a acessibilidade (Tab + Espaço ativa) e devolve o Espaço ao transporte.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Rolagem contínua com a linha no centro | Re-render/repaint constante; pode virar opção depois |
| `:focus-visible` no `isActivationKey` | O Chromium passa a considerar foco visível depois de qualquer tecla; não é confiável no próprio keydown |
| Botões sem foco no clique (`preventDefault` no mousedown) | Teria de ser aplicado em todos os botões; quebra o foco de quem alterna mouse e teclado |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/core/follow.ts](../../ui/src/core/follow.ts) | todo | Função pura e estado do seguir |
| [ui/src/contexts/project/Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx) | `onScroll`, efeito do seguir, zoom ancorado | Rolagem automática e detecção de rolagem manual |
| [ui/src/contexts/transport/TransportBar.tsx](../../ui/src/contexts/transport/TransportBar.tsx) | botão Seguir | Alternar |
| [ui/src/app/shortcuts.ts](../../ui/src/app/shortcuts.ts) | `notePointerDown`, `isActivationKey`; atalho F | UX-01 e atalho |
| [ui/src/app/Workspace.tsx](../../ui/src/app/Workspace.tsx) | ouvintes de captura; `toggleFollow` | Origem do foco; comando |
| [ui/src/app/menuModel.ts](../../ui/src/app/menuModel.ts) | Exibir | Item marcável |
| [ui/e2e/follow.spec.ts](../../ui/e2e/follow.spec.ts) | todo | 3 E2E |
| [ui/e2e/recording-input.spec.ts](../../ui/e2e/recording-input.spec.ts) | fim | Chega ao botão por Tab antes do Enter |

## Lógica

- **Antes:** a linha andava e a vista ficava parada. Espaço num botão clicado acionava o botão.
- **Depois:** a cada leitura da telemetria, se o transporte anda e o seguir está ligado e não pausado, calcula a nova
  vista e rola. Espaço num botão focado por clique vai para o atalho global (tocar/pausar).

## Exemplo

Vista de 1200 px. A linha chega a 1080 px (90%), a vista avança para 960 px e a linha aparece a 120 px da borda. No fim
do loop a linha volta para 100 px com a vista em 2000 px, e a vista volta a 0.

## Verificação

- **TDD:**
  - `follow.test.ts`: 8 testes (bordas de 90%/10%, linha antes da vista, linha distante, entradas inválidas, estado);
  - `shortcuts.test.ts`: foco por clique, por Tab e por clique fora;
  - E2E `follow.spec.ts`: vira a página, F desliga, rolagem manual pausa, UX-01.
- **Sabotagem:** sem a rolagem, "vira a página" falha. Sem a checagem de foco do mouse, o teste UX-01 falha.
- **Ajuste durante a verificação:** `recording-input.spec.ts` focava o botão por código depois de um clique. Com o
  UX-01, Enter ali passou a ser do atalho global, que é o comportamento aprovado. O teste passou a chegar ao botão por
  Tab, como um usuário de teclado. Por ser mudança de teste, as verificações recomeçaram do zero.
- **Verificação 1:** Playwright completo, 58/58.
- **Verificação 2 (outra abordagem):** `tsc`; Vitest 45 arquivos / 263 testes; `vite build`; `check_readmes.py` ok.

## Riscos

- O seguir usa a largura da vista do Arranjo; outras vistas com linha de reprodução (Letra) não rolam por tempo.
- Programas que dão foco por código depois de um clique (sem Tab) também passam o Enter ao atalho global; nos
  diálogos o atalho global já é ignorado.
