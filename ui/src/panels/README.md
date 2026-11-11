# panels

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-panels |
| Título | panels |
| Tipo | composição dos painéis |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/panels/README.md`) |
| Arquivos | 5 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Catálogo de painéis, registro e o layout encaixável (dockview): onde cada contexto vira um painel do workspace.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

`catalog.ts` (lista), `registry.tsx` (componentes), `DockLayout.tsx` e `dockControl.ts` (abrir/fechar/restaurar).

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
| [DockLayout.tsx](DockLayout.tsx) | simétrico (StrictMode monta/desmonta/remonta): conecta ao montar, desconecta ao desmontar |
| [catalog.ts](catalog.ts) | Catálogo dos painéis encaixáveis (metadados sem React): fonte única de ids, títulos e áreas. |
| [dockControl.ts](dockControl.ts) | Fabricação pura: ponte entre a área de painéis (dockview) e quem precisa abrir/focar painéis |
| [registry.test.ts](registry.test.ts) | Testes: registro de painéis; todo painel do catálogo tem componente e vice-versa |
| [registry.tsx](registry.tsx) | Composição: cada painel encaixável junta componentes de um ou mais contextos. |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/panels`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
