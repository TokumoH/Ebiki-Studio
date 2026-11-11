# ctx-tuning

| Campo | Valor |
|---|---|
| Id | DIR-crates-ctx-tuning |
| Título | ctx-tuning |
| Tipo | bounded context de domínio (DDD) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `14257f2`; README por módulo em 2026-11-11 (ver `git log -- crates/ctx-tuning/README.md`) |
| Arquivos | 6 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Bounded context Tuning: mapa de correção de afinação por fonte de áudio (notas detectadas, edições nota a nota, correção por escala) e ajustes do afinador.

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
| [src/book.rs](src/book.rs) | Afinação de um projeto (['TuningBook']): um mapa de correção por fonte de áudio + ajustes do |
| [src/commands.rs](src/commands.rs) | Comandos desfazíveis do contexto Tuning. |
| [src/domain.rs](src/domain.rs) | Modelo do contexto Tuning (puro, sem I/O). |
| [src/lib.rs](src/lib.rs) | Bounded context Tuning (Fase 8): afinador, correção de afinação em tempo real e em |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/ctx-tuning`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
