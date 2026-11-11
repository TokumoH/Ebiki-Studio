# shared-kernel

| Campo | Valor |
|---|---|
| Id | DIR-crates-shared-kernel |
| Título | shared-kernel |
| Tipo | núcleo compartilhado (DDD Shared Kernel) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `684fd38`; README por módulo em 2026-11-11 (ver `git log -- crates/shared-kernel/README.md`) |
| Arquivos | 10 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Tipos compartilhados entre os bounded contexts: IDs, tempo musical, ganho, teoria básica e o contrato de comandos desfazíveis.

## Origem

Criado em `684fd38 2026-11-05 feat(shared-kernel): IDs, tempo musical, mapa de tempo, ganho/pan, teoria básica `. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um módulo por conceito (IDs, tempo, ganho, teoria, comandos); testes junto do código.

## Por que esta organização

Um conjunto mínimo e estável de tipos evita conversões entre contextos.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Duplicar os tipos em cada contexto | Conversões e divergências entre contextos |
| Colocar no app-core | Os contextos não podem depender da aplicação |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/command.rs](src/command.rs) | Contrato de comandos desfazíveis (Command Pattern) e histórico ilimitado. |
| [src/fade.rs](src/fade.rs) | Curvas de fade compartilhadas entre o domínio (Project) e a engine de áudio. |
| [src/gain.rs](src/gain.rs) | Ganho em decibéis e panorâmica. |
| [src/id.rs](src/id.rs) | Identificadores tipados (UUID v7: ordenáveis por criação e gerados no domínio). |
| [src/lib.rs](src/lib.rs) | Shared Kernel do Ebiki Studio. |
| [src/music.rs](src/music.rs) | Primitivas de teoria musical compartilhadas (Harmony, Notation, Tuning). |
| [src/tempo.rs](src/tempo.rs) | Mapa de tempo com BPM e fórmula de compasso variáveis. |
| [src/time.rs](src/time.rs) | Unidades de tempo e formato de áudio. |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/shared-kernel`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
