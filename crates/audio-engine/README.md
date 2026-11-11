# audio-engine

| Campo | Valor |
|---|---|
| Id | DIR-crates-audio-engine |
| Título | audio-engine |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `83d852a`; README por módulo em 2026-11-11 (ver `git log -- crates/audio-engine/README.md`) |
| Arquivos | 22 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Pure Fabrication: engine de áudio em tempo real (grafo compilado, transporte, metrônomo, gravação, render offline).

## Origem

Criado em `83d852a 2026-11-05 feat(audio-engine): grafo compilado lock-free, transporte, metrônomo, gravação co`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [src/core.rs](src/core.rs) | Núcleo de tempo real. Regras: sem alocação, sem bloqueio, sem liberar memória, sem pânico. |
| [src/device.rs](src/device.rs) | Backend de dispositivo real (cpal: ASIO/WASAPI, CoreAudio, ALSA/JACK). Feature 'device'. |
| [src/graph.rs](src/graph.rs) | Grafo de renderização compilado (imutável em estrutura, montado fora da thread de áudio). |
| [src/handle.rs](src/handle.rs) | Lado de controle da engine. |
| [src/input_tap.rs](src/input_tap.rs) | Derivação (tap) de um canal de entrada para análise na thread de controle (afinador). |
| [src/lib.rs](src/lib.rs) | Engine de áudio do Ebiki Studio (Pure Fabrication). |
| [src/metronome.rs](src/metronome.rs) | Clique de metrônomo sintetizado (sem alocação). |
| [src/null_device.rs](src/null_device.rs) | Dispositivo "nulo": thread que chama o núcleo em tempo real simulado, sem placa de som. |
| [src/offline.rs](src/offline.rs) | Render offline com o mesmo núcleo do tempo real (exportação e simulação). |
| [src/recorder.rs](src/recorder.rs) | Gravação: o núcleo copia a entrada para filas lock-free; threads de disco gravam WAV float 32-bit. |
| [src/telemetry.rs](src/telemetry.rs) | Telemetria lock-free publicada pela thread de áudio. |
| [src/transport.rs](src/transport.rs) | Estados do transporte (ver diagrama na arquitetura, seção 5.5). |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/clip_edit.rs](tests/clip_edit.rs) | Fase 3: processamento não destrutivo do clipe na engine (ganho, fades, polaridade, reverso). |
| [tests/engine.rs](tests/engine.rs) | Regressão: um clique cujo tick cai no fim de um bloco mas cujo quadro arredonda |
| [tests/input_monitor.rs](tests/input_monitor.rs) | Entrada ao vivo: medidores por canal e "Ouvir entrada" (monitor direto do dispositivo). |
| [tests/no_alloc.rs](tests/no_alloc.rs) | R1: o núcleo não pode alocar nem liberar memória no callback. |
| [tests/recording.rs](tests/recording.rs) | 2 canais de entrada intercalados: canal 0 = seno, canal 1 = 0,5 constante |
| [tests/common/](tests/common/) | Subdiretório (coberto por este README) |
| [tests/common/mod.rs](tests/common/mod.rs) | Rampa 0, 1, 2, … (identifica a amostra de origem pelo valor). |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/audio-engine`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
