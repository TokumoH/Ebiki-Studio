# ctx-harmony

| Campo | Valor |
|---|---|
| Id | DIR-crates-ctx-harmony |
| Título | ctx-harmony |
| Tipo | bounded context de domínio (DDD) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `0e96dcf`; README por módulo em 2026-11-11 (ver `git log -- crates/ctx-harmony/README.md`) |
| Arquivos | 18 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Bounded context Harmony: escalas, modos, campos harmônicos, acordes, cifra/graus romanos, progressões, sugestões e realização de voicings.

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
| [sql/](sql/) | Subdiretório (coberto por este README) |
| [sql/library_v1.sql](sql/library_v1.sql) | ===================================================================================== |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/commands.rs](src/commands.rs) | Comandos desfazíveis sobre o agregado ['HarmonyWorkspace']. |
| [src/lib.rs](src/lib.rs) | Bounded context Harmony: teoria musical aplicada à composição. |
| [src/domain/](src/domain/) | Subdiretório (coberto por este README) |
| [src/domain/catalog.rs](src/domain/catalog.rs) | Catálogo embutido de progressões "com boa química", por gênero. |
| [src/domain/chord.rs](src/domain/chord.rs) | Acordes: qualidade, tensões, baixo (acorde invertido) e cifra popular. |
| [src/domain/error.rs](src/domain/error.rs) | Erros do contexto Harmony. |
| [src/domain/field.rs](src/domain/field.rs) | Campo harmônico derivado dos intervalos do modo (nunca armazenado por tom — ADR-0007). |
| [src/domain/mod.rs](src/domain/mod.rs) | Modelo de domínio do contexto Harmony. |
| [src/domain/progression.rs](src/domain/progression.rs) | Progressões harmônicas em graus relativos + ritmo harmônico (duração em tempos). |
| [src/domain/realize.rs](src/domain/realize.rs) | Realização de progressões: acordes → notas MIDI (voicing fechado) + tempo em ticks. |
| [src/domain/roman.rs](src/domain/roman.rs) | Graus romanos relativos a um modo (ADR-0007, ADR-0018). |
| [src/domain/scale.rs](src/domain/scale.rs) | Escalas e modos como conjuntos de intervalos e tonalidade ('Key'). |
| [src/domain/suggest.rs](src/domain/suggest.rs) | Motor de sugestões harmônicas atrás da porta ['SuggestionProvider'] (pronta para IA — ADR-0011). |
| [src/domain/workspace.rs](src/domain/workspace.rs) | Agregado ['HarmonyWorkspace']: progressões cadastradas pelo usuário (RF-5.2). |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/ctx-harmony`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
