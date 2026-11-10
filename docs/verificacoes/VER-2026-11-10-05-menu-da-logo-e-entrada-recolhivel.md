# VER-2026-11-10-05 — Menu da logo e entrada de áudio recolhível

| Campo | Valor |
|---|---|
| Id | VER-2026-11-10-05 |
| Título | Logo vira menu do essencial; bloco Entrada de áudio recolhível |
| Gravidade | nenhuma (funcionalidade/navegação) |
| Tipo | funcionalidade |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH (pedidos "na logo do ebiki fique opções de exportação, configuração…" e "recolher o componente de entrada de áudio"; mockups 9 e 10 aprovados) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/shared/MenuBar.tsx`, `ui/src/shared/ui.css`, `ui/src/app/menuModel.ts`, `ui/src/app/Workspace.tsx`, `ui/src/app/Tutorial.tsx`, `ui/src/contexts/recording/InputSection.tsx`, `recording.css`, `ui/src/core/glossary.ts`, testes e docs |

## Problema

1. A logo no topo era só decorativa; exportar e configurações ficavam dentro de *Arquivo*.
2. O bloco Entrada de áudio ocupava espaço fixo no painel Gravação, empurrando a lista de faixas.

## Era proposital?

Não. A logo entrou como marca na identidade (ADR-0033) sem função; o bloco de entrada (ADR-0035)
foi desenhado aberto por ser novo, sem decisão contra recolher.

## Solução

- **Menu da logo** como primeiro menu da barra (mesmo componente `MenuBar`, acessível por teclado):
  cabeçalho com nome/formato do projeto; novo/abrir/salvar; exportar/importar; configurações de áudio,
  atalhos, tema (oferece o oposto do atual), legendas, atualizações; ajuda, sobre, fechar projeto.
- `MenuBar` ganhou, de forma genérica, `content` (conteúdo visual do botão; `label` segue como nome
  acessível) e a entrada `label` (texto informativo não clicável).
- **Entrada recolhível:** título vira botão (`aria-expanded`, `aria-controls`); recolhido mostra
  dispositivo, medidor resumido (maior nível entre os canais) e "🎧 ouvindo"; escolha lembrada.

## Por que esta solução

Reaproveita o menu existente (atalhos, teclado, legendas e testes já valem) em vez de um componente
novo; o modelo continua puro (a logo, que é JSX, entra no Workspace).

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Popover próprio para a logo | Duplicaria navegação por teclado, foco e camadas do `MenuBar` |
| Mover itens de *Arquivo* para a logo | Quebra o hábito de quem já usa *Arquivo*; a logo é atalho, não substituto |
| Recolher só escondendo por CSS | Mantém controles focáveis invisíveis; o bloco é removido do fluxo quando recolhido |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [MenuBar.tsx](../../ui/src/shared/MenuBar.tsx) | tipos, render | `content`, entrada `label`, `aria-label` do botão com conteúdo visual |
| [menuModel.ts](../../ui/src/app/menuModel.ts) | menu `app` | Itens do menu da logo, cabeçalho do projeto |
| [Workspace.tsx](../../ui/src/app/Workspace.tsx) | `buildMenus` | Logo como conteúdo do primeiro menu; remove a marca solta |
| [Tutorial.tsx](../../ui/src/app/Tutorial.tsx) | passo 5 | Texto cita a logo |
| [InputSection.tsx](../../ui/src/contexts/recording/InputSection.tsx) | cabeçalho, resumo | Recolher/abrir, resumo, memória |
| [menuModel.test.ts](../../ui/src/app/menuModel.test.ts), [identity.spec.ts](../../ui/e2e/identity.spec.ts), [workspace-tools.spec.ts](../../ui/e2e/workspace-tools.spec.ts), [recording-input.spec.ts](../../ui/e2e/recording-input.spec.ts) | — | Testes novos e ajustados (a logo é o 1º menu: Alt/F10 caem nela; → leva a Arquivo) |

## Lógica

Barra antes: [logo decorativa] Arquivo … Ajuda. Depois: [logo ▾ = menu] Arquivo … Ajuda — Alt/F10
focam a logo; → vai a Arquivo. Gravação: ▾ Entrada de áudio (aberto) ⇄ ▸ Entrada de áudio ·
dispositivo · medidor · 🎧 ouvindo (recolhido), com `ebiki.recording.inputCollapsed` = "1".

## Exemplo

Logo › Exportar… abre o diálogo de exportação (E2E); logo › Configurações de áudio… abre o diálogo.
Recolhido, o título "Faixas" sobe ~150 px no painel (captura).

## Verificação

- **Verificação 1:** `tsc`; Vitest 242/242; Playwright 53/53.
- **Verificação 2 (outra abordagem):** build de produção; arquitetura + `app` + `shared` 90/90; E2E de
  identidade, gravação, ferramentas, cobertura de legendas e camadas repetidos 2× (32/32); captura
  comparada ao mockup.
- **Sabotagem:** sem gravar a escolha de recolher → o E2E falha (`Received: null`).
- Testes antigos ajustados de propósito (ordem do menu mudou): contagem de → até *Janela* (7→8) e foco
  inicial do Alt/F10 (Arquivo → logo).

## Riscos

- Quem usava Alt/F10 + → contando posições terá um passo a mais até o menu desejado.
