# infra-notation-io

| Campo | Valor |
|---|---|
| Id | DIR-crates-infra-notation-io |
| Título | infra-notation-io |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `02718c6`; README por módulo em 2026-11-11 (ver `git log -- crates/infra-notation-io/README.md`) |
| Arquivos | 19 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Pure Fabrication de I/O de notação: MIDI (SMF), MusicXML (.musicxml/.mxl) e Guitar Pro 7 (.gp/gpif).

## Origem

Criado em `02718c6 2026-11-05 feat(notação): contexto Notation (partitura + tablatura) e I/O MIDI/MusicXML/Guit`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [src/archive.rs](src/archive.rs) | Leitura/escrita de zip (.mxl, .gp) com proteção contra zip bomb. |
| [src/error.rs](src/error.rs) | Erros de I/O de notação e limites de segurança. |
| [src/gp.rs](src/gp.rs) | Guitar Pro 7+ ('.gp'): zip com 'Content/score.gpif' (XML "GPIF"). Importa o subconjunto central: |
| [src/lib.rs](src/lib.rs) | I/O de notação (Pure Fabrication): converte o agregado 'Score' (ctx-notation) de/para formatos |
| [src/midi.rs](src/midi.rs) | Standard MIDI File ⇄ 'Score' (crate 'midly'). |
| [src/musicxml.rs](src/musicxml.rs) | MusicXML 4.0 (partwise) — importação e exportação, '.musicxml' (texto) e '.mxl' (zip). |
| [src/xml.rs](src/xml.rs) | Árvore XML mínima (DOM) com limites de profundidade/tamanho e escritor com escape. |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/fuzz.rs](tests/fuzz.rs) | Testes estilo fuzz: bytes aleatórios e arquivos válidos corrompidos nunca causam pânico. |
| [tests/guitar_pro.rs](tests/guitar_pro.rs) | reprodução: ligadura C4 (colcheia de tercina + mínima) e repetição 3x |
| [tests/midi.rs](tests/midi.rs) | grafia pela armadura (Sib maior): Bb, não A# |
| [tests/musicxml.rs](tests/musicxml.rs) | a tablatura corresponde à altura escrita |
| [tests/common/](tests/common/) | Subdiretório (coberto por este README) |
| [tests/common/mod.rs](tests/common/mod.rs) | Partituras sintéticas compartilhadas pelos testes de integração. |
| [tests/data/](tests/data/) | Subdiretório (coberto por este README) |
| [tests/data/sample.gpif](tests/data/sample.gpif) | Guitar Pro (gpif) de teste |
| [tests/data/sample.musicxml](tests/data/sample.musicxml) | MusicXML de teste |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/infra-notation-io`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
