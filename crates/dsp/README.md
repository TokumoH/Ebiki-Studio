# dsp

| Campo | Valor |
|---|---|
| Id | DIR-crates-dsp |
| Título | dsp |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `2625757`; README por módulo em 2026-11-11 (ver `git log -- crates/dsp/README.md`) |
| Arquivos | 22 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Pure Fabrication de processamento de áudio em tempo real: filtros, dinâmica, efeitos, medidores e metrônomo. Sem lógica de domínio e sem I/O.

## Origem

Criado em `2625757 2026-11-05 feat(dsp): crate de processamento de áudio em tempo real (Fase 2)`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [src/biquad.rs](src/biquad.rs) | Filtros biquad (RBJ Audio EQ Cookbook) e equalizador paramétrico de até 8 bandas. |
| [src/chorus.rs](src/chorus.rs) | Chorus estéreo: linha de atraso modulada por LFO senoidal (base de 7 ms ± profundidade), |
| [src/compressor.rs](src/compressor.rs) | Compressor *feed-forward* com joelho suave, envelope de ataque/release no domínio do ganho |
| [src/deesser.rs](src/deesser.rs) | De-esser de banda dividida. |
| [src/delay.rs](src/delay.rs) | Delay estéreo com realimentação (< 1), filtro passa-baixa (*high-cut*) no laço, mix e modo |
| [src/gate.rs](src/gate.rs) | Noise gate com limiar, ataque, *hold*, release e faixa (atenuação máxima). |
| [src/lib.rs](src/lib.rs) | # dsp — processamento de áudio em tempo real (Pure Fabrication) |
| [src/limiter.rs](src/limiter.rs) | Limitador *brickwall* com *lookahead* (~1,5 ms). |
| [src/meters.rs](src/meters.rs) | Medidores: pico amostral (com *hold* e decaimento), RMS em janela, *true peak* (4×), |
| [src/metronome.rs](src/metronome.rs) | Gerador de clique do metrônomo com disparo sample-accurate. |
| [src/pan.rs](src/pan.rs) | Lei de panorâmica (potência constante, −3 dB no centro), coerente com 'shared_kernel::Pan'. |
| [src/reverb.rs](src/reverb.rs) | Reverb algorítmico estilo Freeverb (Jezar, domínio público): 8 filtros pente com |
| [src/saturation.rs](src/saturation.rs) | Saturação suave por 'tanh' com *drive*, filtro de tom e mix, com compensação de ganho. |
| [src/signals.rs](src/signals.rs) | Geradores de sinais sintéticos determinísticos e medidas offline. |
| [src/smoothing.rs](src/smoothing.rs) | Suavização de parâmetros para evitar ruído de "zíper" em mudanças bruscas. |
| [src/util.rs](src/util.rs) | Funções auxiliares numéricas usadas por todos os processadores. |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/no_alloc.rs](tests/no_alloc.rs) | Garante que 'process', 'set_params' e leituras de medidores não alocam. |
| [tests/performance.rs](tests/performance.rs) | Orçamento de CPU: 32 faixas com a cadeia completa de efeitos + master, bloco de 128 amostras |
| [tests/properties.rs](tests/properties.rs) | Propriedades de robustez: para qualquer parâmetro (inclusive NaN/∞/valores absurdos) e |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/dsp`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
