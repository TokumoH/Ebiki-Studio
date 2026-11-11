# infra-adapters

| Campo | Valor |
|---|---|
| Id | DIR-crates-infra-adapters |
| Título | infra-adapters |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `3e44c8f`; README por módulo em 2026-11-11 (ver `git log -- crates/infra-adapters/README.md`) |
| Arquivos | 13 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Adaptadores das portas do app-core: DSP (fábrica de processadores) e codecs (mídia).

## Origem

Criado em `3e44c8f 2026-11-05 feat: adaptadores DSP/codecs, exportação (mix, stems), análise de mixagem e forma`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um módulo por responsabilidade técnica; testes de integração em `tests/` com dados e auxiliares em `tests/common`/`tests/data` quando existem.

## Por que esta organização

Isolar o técnico mantém o domínio puro e permite trocar a biblioteca sem mexer nas regras de negócio.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Lógica técnica dentro dos contextos | O domínio deixaria de ser puro e testável sem I/O |
| Um crate técnico único | Dependências pesadas (codecs, SQLite, áudio) entrariam em todos os consumidores |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/dsp_factory.rs](src/dsp_factory.rs) | 'ProcessorFactory' sobre o crate 'dsp'. Parâmetros são achatados em 'Vec<f32>' numa ordem fixa por |
| [src/lib.rs](src/lib.rs) | Adaptadores (hexagonal) que ligam as portas do 'app-core' às bibliotecas técnicas. |
| [src/media.rs](src/media.rs) | 'MediaPort' sobre o crate 'infra-codecs'. |
| [src/notation.rs](src/notation.rs) | 'NotationFormats' sobre o crate 'infra-notation-io' (MusicXML, MXL, MIDI, Guitar Pro 7). |
| [src/synth.rs](src/synth.rs) | 'MidiRenderer' sobre o crate 'synth'. |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/full_chain.rs](tests/full_chain.rs) | Simulação com as implementações reais: codecs + DSP + SQLite + engine. |
| [tests/no_alloc.rs](tests/no_alloc.rs) | O insert de correção de afinação não aloca nem libera memória em 'process'/'set_params'/'reset'. |
| [tests/simulation.rs](tests/simulation.rs) | Simulação de sistema: uma sessão realista montada pelas mesmas portas que a interface usa, |
| [tests/common/](tests/common/) | Subdiretório (coberto por este README) |
| [tests/common/mod.rs](tests/common/mod.rs) | Montagem compartilhada pelos testes de integração: o 'Studio' com as implementações reais |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/infra-adapters`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
