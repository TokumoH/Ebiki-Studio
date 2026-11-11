# ctx-notation

| Campo | Valor |
|---|---|
| Id | DIR-crates-ctx-notation |
| Título | ctx-notation |
| Tipo | bounded context de domínio (DDD) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `02718c6`; README por módulo em 2026-11-11 (ver `git log -- crates/ctx-notation/README.md`) |
| Arquivos | 15 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Bounded context Notation: partitura e tablatura (modelo, edição desfazível, auxílios, transposição e reprodução).

## Origem

Criado em `02718c6 2026-11-05 feat(notação): contexto Notation (partitura + tablatura) e I/O MIDI/MusicXML/Guit`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [src/analysis.rs](src/analysis.rs) | Auxílios de escrita: validação de compasso, preenchimento de pausas e notas fora da escala. |
| [src/arrange.rs](src/arrange.rs) | Arranjo de notas temporizadas em compassos e vozes (serviço de domínio). |
| [src/book.rs](src/book.rs) | Coleção de partituras de um projeto (['ScoreBook']) e comandos sobre ela. |
| [src/commands.rs](src/commands.rs) | Comandos desfazíveis sobre o agregado ['Score']. |
| [src/duration.rs](src/duration.rs) | Figuras rítmicas (semibreve … semifusa de 128), pontos de aumento e quiálteras. |
| [src/error.rs](src/error.rs) | Erros do contexto Notation. |
| [src/frac.rs](src/frac.rs) | Fração racional exata (em unidades de semibreve) para somar durações sem erro de arredondamento. |
| [src/lib.rs](src/lib.rs) | Bounded context Notation (Fase 7): partitura e tablatura. |
| [src/model.rs](src/model.rs) | Agregado ['Score'] e suas entidades/value objects. |
| [src/pitch.rs](src/pitch.rs) | Alturas grafadas (nome + acidente + oitava) e armaduras de clave com grafia enarmônica. |
| [src/playback.rs](src/playback.rs) | Partitura → eventos MIDI temporizados para reprodução (soundfont GM) e exportação. |
| [src/tab.rs](src/tab.rs) | Tablatura ⇄ altura: altura a partir de corda/traste/capotraste e melhor posição para uma altura. |
| [src/transpose.rs](src/transpose.rs) | Transposição cromática com grafia enarmônica pela armadura de destino. |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/ctx-notation`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
