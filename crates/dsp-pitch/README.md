# dsp-pitch

| Campo | Valor |
|---|---|
| Id | DIR-crates-dsp-pitch |
| Título | dsp-pitch |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `14257f2`; README por módulo em 2026-11-11 (ver `git log -- crates/dsp-pitch/README.md`) |
| Arquivos | 12 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Pure Fabrication de DSP de altura: detecção (YIN), afinador, pitch shift (PSOLA), correção de afinação, análise de notas e time-stretch (WSOLA).

## Origem

Criado em `14257f2 2026-11-05 feat: fases 6 e 8 — DSP de altura, contexto Tuning e contexto Sync`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [src/analysis.rs](src/analysis.rs) | Análise offline: trilha de altura, segmentação em notas e render de correções nota a nota. |
| [src/corrector.rs](src/corrector.rs) | Correção de afinação em tempo real por escala (estilo "auto-tune"). |
| [src/detector.rs](src/detector.rs) | Detecção de altura (porta 'PitchDetector', ADR-0011) e implementação YIN. |
| [src/lib.rs](src/lib.rs) | DSP de altura do Ebiki Studio (Pure Fabrication, depende apenas do 'shared-kernel'). |
| [src/shifter.rs](src/shifter.rs) | Pitch shift em tempo real (porta 'PitchShifter') e implementação PSOLA síncrona ao período. |
| [src/signal.rs](src/signal.rs) | Geradores de sinais sintéticos determinísticos (testes, simulação e calibração). |
| [src/stretch.rs](src/stretch.rs) | Time-stretch offline (WSOLA) e pitch shift offline (stretch + reamostragem). |
| [src/tuner.rs](src/tuner.rs) | Afinador: frequência → nota mais próxima + desvio em cents, e processador em tempo real. |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/no_alloc.rs](tests/no_alloc.rs) | Garante que os processadores de tempo real não alocam após a preparação. |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/dsp-pitch`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
