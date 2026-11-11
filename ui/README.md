# ui

| Campo | Valor |
|---|---|
| Id | DIR-ui |
| Título | ui |
| Tipo | pacote da interface (React + Vite) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `1711a11`; README por módulo em 2026-11-11 (ver `git log -- ui/README.md`) |
| Arquivos | 9 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Interface do Ebiki Studio: React 19 + TypeScript + Vite, testes Vitest e Playwright, modo simulado para rodar sem o núcleo.

## Origem

Criado em `1711a11 2026-11-05 test: padroniza modo de contagem do assert_no_alloc no workspace (engine e dsp)`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

`src/` (código), `e2e/` (testes de ponta a ponta), `public/` (estáticos), configurações na raiz do pacote.

## Por que esta organização

Pacote npm próprio, consumido pelo Tauri como frontend (`beforeBuildCommand`).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| UI dentro do crate app-desktop | Misturaria ferramentas Node e Cargo no mesmo diretório |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [src/](src/) | Código da interface (README próprio) |
| [e2e/](e2e/) | Testes de ponta a ponta (README próprio) |
| [public/](public/) | Arquivos estáticos (README próprio) |
| [index.html](index.html) | Página HTML |
| [package-lock.json](package-lock.json) | Versões travadas das dependências |
| [package.json](package.json) | Dependências e scripts npm |
| [playwright.config.ts](playwright.config.ts) | E2E_PORT permite rodar em paralelo com outro servidor de desenvolvimento (padrão 5173). |
| [tsconfig.json](tsconfig.json) | Configuração TypeScript |
| [vite.config.ts](vite.config.ts) | '--mode mock' força o transporte simulado (UI no navegador sem o núcleo Rust). |

## Lógica

Comandos: `npm run dev:mock` (navegador, núcleo simulado), `npx vitest run`, `npx playwright test --trace off`, `npx vite build`. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
