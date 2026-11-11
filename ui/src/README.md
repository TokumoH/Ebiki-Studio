# src

| Campo | Valor |
|---|---|
| Id | DIR-ui-src |
| Título | src |
| Tipo | código da interface (índice das camadas) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/README.md`) |
| Arquivos | 11 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Código-fonte da interface React/TypeScript, dividido em camadas com regras de dependência testadas.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Camadas `shared`, `ipc`, `core`, `contexts`, `panels`, `app` (+ `test`), cada uma com README; `main.tsx` inicializa o transporte e a tela de carregamento.

## Por que esta organização

Direção única de dependência entre camadas, verificada por teste.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Pastas por funcionalidade sem camadas | Regras de dependência ficariam implícitas |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [shared/](shared/) | Componentes e estilos reutilizáveis sem conhecer IPC nem domínio: botões, diálogos, fader, knob, medidor, barra de menus, camada de legendas, tokens de tema. (README próprio) |
| [ipc/](ipc/) | Único ponto da UI que conhece o mecanismo de comunicação: transporte Tauri ou simulado, esquemas (zod) que validam cada resposta do núcleo, API tipada, mocks para testes e o atualizador. (README próprio) |
| [core/](core/) | Stores (zustand) e funções puras usadas por várias áreas: sessão, telemetria, tema, áudio, loop, compassos, glossário das legendas, avisos. (README próprio) |
| [contexts/](contexts/) | Uma pasta por contexto de domínio (projeto/arranjo, mixagem, gravação, transporte, harmonia, biblioteca, notação, letra, afinação…), com painéis e regras de apresentação. Um contexto nunca importa outro (teste de arquitetura). (README próprio) |
| [panels/](panels/) | Catálogo de painéis, registro e o layout encaixável (dockview): onde cada contexto vira um painel do workspace. (README próprio) |
| [app/](app/) | Tela inicial, workspace, menu principal, atalhos, central de ajuda, tutorial, legendas, atualizações, diálogos de nível do app e tela de carregamento. (README próprio) |
| [test/](test/) | Preparação do ambiente de testes unitários (jsdom, polyfills). (README próprio) |
| [README.md](README.md) | Documento |
| [architecture.test.ts](architecture.test.ts) | U4: regras de dependência da UI (arquitetura de componentes). |
| [main.tsx](main.tsx) | espera o primeiro quadro desenhado pelo React antes de revelar a interface |
| [vite-env.d.ts](vite-env.d.ts) | / <reference types="vite/client" /> |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
