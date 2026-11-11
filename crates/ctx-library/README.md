# ctx-library

| Campo | Valor |
|---|---|
| Id | DIR-crates-ctx-library |
| Título | ctx-library |
| Tipo | bounded context de domínio (DDD) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `0e96dcf`; README por módulo em 2026-11-11 (ver `git log -- crates/ctx-library/README.md`) |
| Arquivos | 11 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Bounded context Library: catálogo de bases instrumentais (CC0, geradas e do usuário), busca, gerador de bases MIDI e política de BPM/tom.

## Origem

Criado em `0e96dcf 2026-11-05 feat: contextos Harmony e Library (Fases 5 e 4 — domínio)`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [src/commands.rs](src/commands.rs) | Comandos desfazíveis sobre o agregado ['BaseCatalog']. |
| [src/lib.rs](src/lib.rs) | Bounded context Library: biblioteca de bases instrumentais (Fase 4). |
| [src/domain/](src/domain/) | Subdiretório (coberto por este README) |
| [src/domain/base.rs](src/domain/base.rs) | Entidade ['Base'] e seus objetos de valor. |
| [src/domain/catalog.rs](src/domain/catalog.rs) | Agregado ['BaseCatalog']: a biblioteca de bases com busca e filtros (RF-4.4). |
| [src/domain/error.rs](src/domain/error.rs) | Erros do contexto Library. |
| [src/domain/generator.rs](src/domain/generator.rs) | Gerador de bases (RF-4.2): acordes já realizados + padrão rítmico/estilo + bateria + baixo → |
| [src/domain/mismatch.rs](src/domain/mismatch.rs) | Política de divergência de BPM/tom ao inserir uma base no projeto (RF-4.5). |
| [src/domain/mod.rs](src/domain/mod.rs) | Modelo de domínio do contexto Library. |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/ctx-library`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
