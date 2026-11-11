# shared

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-shared |
| Título | shared |
| Tipo | design system (componentes genéricos) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/shared/README.md`) |
| Arquivos | 9 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Componentes e estilos reutilizáveis sem conhecer IPC nem domínio: botões, diálogos, fader, knob, medidor, barra de menus, camada de legendas, tokens de tema.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um arquivo por componente maior; `components.tsx` com os pequenos; `tokens.css` (cores, tamanhos, camadas) e `ui.css`.

## Por que esta organização

Camadas com direção única de dependência (app → panels → contexts → core/ipc/shared) mantêm componentes independentes; o teste `architecture.test.ts` impede violações.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Organizar por tipo de arquivo (components/, hooks/…) | Espalharia cada contexto por várias pastas e esconderia as dependências |
| Tudo em `src/` | Sem fronteiras, o acoplamento cresce sem ser notado |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [MenuBar.test.tsx](MenuBar.test.tsx) | Testes: MenuBar; F10 foca o primeiro menu; setas trocam de menu |
| [MenuBar.tsx](MenuBar.tsx) | Barra de menus fixa (estilo Photoshop / VS Code), dirigida por dados e acessível por teclado: |
| [TooltipLayer.tsx](TooltipLayer.tsx) | Camada única de legendas (design system): escuta o documento, espera o ponteiro parar ~0,5 s sobre |
| [components.test.tsx](components.test.tsx) | Testes: curva do fader; é monotônica e inversível |
| [components.tsx](components.tsx) | Design System: componentes visuais sem regra de negócio, reutilizáveis em qualquer contexto. |
| [tokens.css](tokens.css) | Design tokens — tema escuro (padrão de estúdio) e claro. Cores só via tokens. |
| [ui.css](ui.css) | Componentes do design system |
| [assets/](assets/) | Subdiretório (coberto por este README) |
| [assets/ebiki-mark.png](assets/ebiki-mark.png) | Imagem |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/shared`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
