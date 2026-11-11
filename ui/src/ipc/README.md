# ipc

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-ipc |
| Título | ipc |
| Tipo | adaptador de IPC (Pure Fabrication da UI) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/ipc/README.md`) |
| Arquivos | 52 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Único ponto da UI que conhece o mecanismo de comunicação: transporte Tauri ou simulado, esquemas (zod) que validam cada resposta do núcleo, API tipada, mocks para testes e o atualizador.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

`transport.ts` (porta), `tauri.ts`/`mock*.ts` (adaptadores), `schema.ts` + `types.ts` (contrato), `api.ts`, fixtures de contrato em `__fixtures__/`.

## Por que esta organização

Camadas com direção única de dependência (app → panels → contexts → core/ipc/shared) mantêm componentes independentes; o teste `architecture.test.ts` impede violações.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Organizar por tipo de arquivo (components/, hooks/…) | Espalharia cada contexto por várias pastas e esconderia as dependências |
| Tudo em `src/` | Sem fronteiras, o acoplamento cresce sem ser notado |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [api.ts](api.ts) | API tipada: uma função por comando do núcleo. Respostas são validadas pelo contrato (zod). |
| [composition.contract.test.ts](composition.contract.test.ts) | Contrato IPC de composição (harmonia + bases): fixtures geradas pelo Rust |
| [contract.test.ts](contract.test.ts) | Contrato IPC: as fixtures são geradas pelo Rust (crates/app-core/tests/contract_fixtures.rs). |
| [edit.contract.test.ts](edit.contract.test.ts) | Contrato IPC da Fase 3 (edição): fixtures geradas por crates/app-core/tests/contract_fixtures.rs. |
| [mock.test.ts](mock.test.ts) | 8 s a 48 kHz = 384000 quadros → ceil(384000 / 1024) = 375 picos (mín., máx.) = 750 valores |
| [mock.ts](mock.ts) | Transporte simulado: um núcleo em memória para rodar a UI no navegador (desenvolvimento e E2E). |
| [mockComposition.test.ts](mockComposition.test.ts) | Testes: transporte simulado de composição; campo de Dó maior e todos os modos com 7 graus |
| [mockComposition.ts](mockComposition.ts) | Simulação dos comandos de composição (harmonia, bases, biblioteca, sintetizador) para o modo |
| [mockEdit.test.ts](mockEdit.test.ts) | Testes: transporte simulado — edição (Fase 3); dividir gera duas partes contíguas e desfazer volta a uma |
| [mockEdit.ts](mockEdit.ts) | Transporte simulado — Fase 3 (edição): mesmas regras visíveis do núcleo (ctx-project/domain/edit.rs). |
| [mockMusic.test.ts](mockMusic.test.ts) | Testes: núcleo simulado — música (contrato da API); partitura: criar, editar voz (um desfazer por gesto), validar, MusicXML e audição |
| [mockMusic.ts](mockMusic.ts) | Núcleo simulado das Fases 6–8 (partituras, letra, afinação, afinador, time-stretch). |
| [schema.ts](schema.ts) | Contrato IPC (espelha os DTOs serde do núcleo Rust). Os tipos TS são inferidos dos esquemas zod, |
| [spectrogram.ts](spectrogram.ts) | Decodificação do espectrograma binário 'AMSG' (gerado por crates/infra-codecs/src/spectrogram.rs). |
| [tauri.ts](tauri.ts) | Adaptador real do transporte IPC (Tauri): invoke, eventos e diálogos de arquivo. |
| [transport.ts](transport.ts) | Porta de transporte: o único ponto da UI que conhece o mecanismo de IPC. |
| [types.ts](types.ts) | ---- Harmonia e bases ---- |
| [updater.ts](updater.ts) | Porta de atualização do app (ADR-0034): a UI só conhece esta interface. A implementação real |
| [updaterTauri.ts](updaterTauri.ts) | Adaptador real da porta de atualização (Pure Fabrication): plugin updater do Tauri. O download |
| [__fixtures__/](__fixtures__/) | Subdiretório (coberto por este README) |
| [__fixtures__/composition-base-options.json](__fixtures__/composition-base-options.json) | Dados/configuração JSON |
| [__fixtures__/composition-bases.json](__fixtures__/composition-bases.json) | Dados/configuração JSON |
| [__fixtures__/composition-error.json](__fixtures__/composition-error.json) | Dados/configuração JSON |
| [__fixtures__/composition-field.json](__fixtures__/composition-field.json) | Dados/configuração JSON |
| [__fixtures__/composition-generated.json](__fixtures__/composition-generated.json) | Dados/configuração JSON |
| [__fixtures__/composition-insert-base-inserted.json](__fixtures__/composition-insert-base-inserted.json) | Dados/configuração JSON |
| [__fixtures__/composition-insert-base-needs-decision.json](__fixtures__/composition-insert-base-needs-decision.json) | Dados/configuração JSON |
| [__fixtures__/composition-inserted-clip.json](__fixtures__/composition-inserted-clip.json) | Dados/configuração JSON |
| [__fixtures__/composition-keys.json](__fixtures__/composition-keys.json) | Dados/configuração JSON |
| [__fixtures__/composition-library-history.json](__fixtures__/composition-library-history.json) | Dados/configuração JSON |
| [__fixtures__/composition-mismatch.json](__fixtures__/composition-mismatch.json) | Dados/configuração JSON |
| [__fixtures__/composition-progressions.json](__fixtures__/composition-progressions.json) | Dados/configuração JSON |
| [__fixtures__/composition-realization.json](__fixtures__/composition-realization.json) | Dados/configuração JSON |
| [__fixtures__/composition-suggest-next.json](__fixtures__/composition-suggest-next.json) | Dados/configuração JSON |
| [__fixtures__/composition-synth-status.json](__fixtures__/composition-synth-status.json) | Dados/configuração JSON |
| [__fixtures__/composition-user-progression.json](__fixtures__/composition-user-progression.json) | Dados/configuração JSON |
| [__fixtures__/edit-requests.json](__fixtures__/edit-requests.json) | Dados/configuração JSON |
| [__fixtures__/edit-session.json](__fixtures__/edit-session.json) | Dados/configuração JSON |
| [__fixtures__/error.json](__fixtures__/error.json) | Dados/configuração JSON |
| [__fixtures__/lyrics-requests.json](__fixtures__/lyrics-requests.json) | Dados/configuração JSON |
| [__fixtures__/meters.json](__fixtures__/meters.json) | Dados/configuração JSON |
| [__fixtures__/music-session.json](__fixtures__/music-session.json) | Dados/configuração JSON |
| [__fixtures__/score-edits.json](__fixtures__/score-edits.json) | Dados/configuração JSON |
| [__fixtures__/score-report.json](__fixtures__/score-report.json) | Dados/configuração JSON |
| [__fixtures__/score.json](__fixtures__/score.json) | Dados/configuração JSON |
| [__fixtures__/session.json](__fixtures__/session.json) | Dados/configuração JSON |
| [__fixtures__/strip-edits.json](__fixtures__/strip-edits.json) | Dados/configuração JSON |
| [__fixtures__/strip-targets.json](__fixtures__/strip-targets.json) | Dados/configuração JSON |
| [__fixtures__/transport.json](__fixtures__/transport.json) | Dados/configuração JSON |
| [__fixtures__/tuner-reading.json](__fixtures__/tuner-reading.json) | Dados/configuração JSON |
| [__fixtures__/tuning-map.json](__fixtures__/tuning-map.json) | Dados/configuração JSON |
| [__fixtures__/voice-edits.json](__fixtures__/voice-edits.json) | Dados/configuração JSON |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/ipc`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
