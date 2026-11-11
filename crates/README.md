# crates

| Campo | Valor |
|---|---|
| Id | DIR-crates |
| Título | crates |
| Tipo | workspace Rust (índice dos módulos) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `684fd38`; README por módulo em 2026-11-11 (ver `git log -- crates/README.md`) |
| Arquivos | 19 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Todos os crates do backend: domínio (bounded contexts), serviços técnicos, aplicação e o app desktop.

## Origem

Criado em `684fd38 2026-11-05 feat(shared-kernel): IDs, tempo musical, mapa de tempo, ganho/pan, teoria básica `. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um crate por bounded context (`ctx-*`), por serviço técnico (`dsp*`, `infra-*`, `synth`, `audio-engine`), mais `shared-kernel`, `app-core` e `app-desktop` (ADR-0003). Cada crate tem README próprio.

## Por que esta organização

Crates separados tornam as dependências explícitas e verificáveis (a CI barra dependências proibidas).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Um crate monolítico | Dependências entre camadas ficariam invisíveis |
| Repositórios separados | Mudanças cruzadas virariam vários PRs sincronizados |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [app-core/](app-core/) | Camada de aplicação: sessão (agregados abertos), casos de uso, histórico e portas para infraestrutura. (README próprio) |
| [app-desktop/](app-desktop/) | Ebiki Studio — aplicativo desktop (Tauri): adaptador de entrada (comandos IPC) e composition root. (README próprio) |
| [audio-engine/](audio-engine/) | Pure Fabrication: engine de áudio em tempo real (grafo compilado, transporte, metrônomo, gravação, render offline). (README próprio) |
| [ctx-harmony/](ctx-harmony/) | Bounded context Harmony: escalas, modos, campos harmônicos, acordes, cifra/graus romanos, progressões, sugestões e realização de voicings. (README próprio) |
| [ctx-library/](ctx-library/) | Bounded context Library: catálogo de bases instrumentais (CC0, geradas e do usuário), busca, gerador de bases MIDI e política de BPM/tom. (README próprio) |
| [ctx-mixing/](ctx-mixing/) | Bounded context Mixing: canais, buses, sends, efeitos, presets e auxílio de mixagem por regras. (README próprio) |
| [ctx-notation/](ctx-notation/) | Bounded context Notation: partitura e tablatura (modelo, edição desfazível, auxílios, transposição e reprodução). (README próprio) |
| [ctx-project/](ctx-project/) | Bounded context Project: projeto, faixas, clipes, fontes de áudio e mapa de tempo. (README próprio) |
| [ctx-recording/](ctx-recording/) | Bounded context Recording: armar faixas, entradas, monitoração, punch, metrônomo e takes. (README próprio) |
| [ctx-sync/](ctx-sync/) | Bounded context Sync: letra sincronizada por palavra/sílaba (LRC/SRT), sessão de sincronização por toque e mapa de warp para time-stretch. (README próprio) |
| [ctx-tuning/](ctx-tuning/) | Bounded context Tuning: mapa de correção de afinação por fonte de áudio (notas detectadas, edições nota a nota, correção por escala) e ajustes do afinador. (README próprio) |
| [dsp/](dsp/) | Pure Fabrication de processamento de áudio em tempo real: filtros, dinâmica, efeitos, medidores e metrônomo. Sem lógica de domínio e sem I/O. (README próprio) |
| [dsp-pitch/](dsp-pitch/) | Pure Fabrication de DSP de altura: detecção (YIN), afinador, pitch shift (PSOLA), correção de afinação, análise de notas e time-stretch (WSOLA). (README próprio) |
| [infra-adapters/](infra-adapters/) | Adaptadores das portas do app-core: DSP (fábrica de processadores) e codecs (mídia). (README próprio) |
| [infra-codecs/](infra-codecs/) | Pure Fabrication de codecs: probe/decodificação, conversão de taxa, importação para o projeto, picos de waveform e exportação (WAV, AIFF, FLAC, MP3, OGG). (README próprio) |
| [infra-notation-io/](infra-notation-io/) | Pure Fabrication de I/O de notação: MIDI (SMF), MusicXML (.musicxml/.mxl) e Guitar Pro 7 (.gp/gpif). (README próprio) |
| [infra-persistence/](infra-persistence/) | Pure Fabrication: persistência SQLite (project.db por projeto e library.db do usuário), migrações e backups. (README próprio) |
| [shared-kernel/](shared-kernel/) | Tipos compartilhados entre os bounded contexts: IDs, tempo musical, ganho, teoria básica e o contrato de comandos desfazíveis. (README próprio) |
| [synth/](synth/) | Pure Fabrication: síntese General MIDI offline (sintetizador simples embutido; SoundFont opcional). (README próprio) |

## Lógica

Direção das dependências: `ctx-*` → `shared-kernel`; serviços técnicos → `shared-kernel` (e o contexto que formatam, como `infra-notation-io` → `ctx-notation`); `app-core` → contextos e engine; `infra-adapters`/`infra-persistence` → `app-core` (implementam as portas); `app-desktop` compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
