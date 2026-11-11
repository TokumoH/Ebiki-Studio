# ctx-sync

| Campo | Valor |
|---|---|
| Id | DIR-crates-ctx-sync |
| Título | ctx-sync |
| Tipo | bounded context de domínio (DDD) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `14257f2`; README por módulo em 2026-11-11 (ver `git log -- crates/ctx-sync/README.md`) |
| Arquivos | 10 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Bounded context Sync: letra sincronizada por palavra/sílaba (LRC/SRT), sessão de sincronização por toque e mapa de warp para time-stretch.

## Origem

Criado em `14257f2 2026-11-05 feat: fases 6 e 8 — DSP de altura, contexto Tuning e contexto Sync`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

`domain` (modelo e invariantes) e `commands` (comandos desfazíveis); depende só do `shared-kernel`.

## Por que esta organização

Separar modelo e comandos deixa as invariantes testáveis e cada mudança desfazível (Command Pattern, ADR-0008).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Domínio junto da infraestrutura | Quebraria a pureza (GRASP Pure Fabrication) e os testes sem I/O |
| Um crate de domínio único | Contextos acoplados; o CLAUDE.md exige bounded contexts independentes |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/commands.rs](src/commands.rs) | Comandos desfazíveis do contexto Sync. |
| [src/error.rs](src/error.rs) | Erros do contexto Sync. |
| [src/formats.rs](src/formats.rs) | Exportação/importação de letra em LRC (linha e "enhanced" por palavra/sílaba) e SRT. |
| [src/hyphenation.rs](src/hyphenation.rs) | Separação silábica do português por regras (sem dicionário). |
| [src/lib.rs](src/lib.rs) | Bounded context Sync (Fase 6): letra sincronizada por palavra/sílaba, exportação |
| [src/lyrics.rs](src/lyrics.rs) | Letra sincronizada por linha, palavra e sílaba (tempo musical em ticks). |
| [src/tap.rs](src/tap.rs) | Sessão de sincronização por toque ("tap-to-sync"). |
| [src/warp.rs](src/warp.rs) | Mapa de warp: liga quadros do áudio de origem a posições musicais (ticks) para |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/ctx-sync`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
