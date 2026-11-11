# test

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-test |
| Título | test |
| Tipo | configuração de testes |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/test/README.md`) |
| Arquivos | 1 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Preparação do ambiente de testes unitários (jsdom, polyfills).

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Arquivos de setup referenciados pela configuração do Vitest.

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
| [setup.ts](setup.ts) | Preparação dos testes unitários (matchers do jest-dom e polyfills do jsdom). |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/test`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
