# VER-2026-11-17-11 — Áudio: modo silencioso indevido, desconexão de dispositivos e formatos inteiros

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-11 |
| Título | Microfone que falha não derruba a saída; reabrir quando um dispositivo cai ou volta; formatos inteiros; correções da varredura 6 |
| Gravidade | alta ("não consigo escutar nada") |
| Tipo | correção |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("verifique por bugs, não consigo escutar nada…") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `crates/audio-engine/src/{device.rs,telemetry.rs,core.rs}`, `crates/app-desktop/src/{audio.rs,main.rs}`, `ui/src/{ipc/schema.ts,ipc/mock.ts,core/live.ts,app/Workspace.tsx,app/AudioSettingsDialog.tsx,app/app.css,app/helpTopics.ts}`, `ui/src/contexts/recording/{inputRoutes.ts,inputRoutes.test.ts,RecordingPanel.tsx,InputSection.tsx}`, `ui/e2e/audio-devices.spec.ts` (novo), `ui/e2e/README.md`, `docs/manual/README.md`, `docs/relatorios/2026-11-17-varredura-6-audio.md` (novo), `docs/relatorios/README.md` |

## Problema

Ver o relatório `docs/relatorios/2026-11-17-varredura-6-audio.md`, seção 1. Em resumo:
- um microfone ausente ou incompatível derrubava a abertura inteira e deixava o app no modo silencioso (D1);
- a desconexão com o app aberto não era tratada (D2);
- só dispositivos f32 eram aceitos (D3);
- fone sumido (D4) ou buffer fora da faixa (D5) também levavam ao modo silencioso.

## Era proposital?

Não. O `?` na entrada vem da Fase 1, quando a entrada era opcional só no pedido. Os callbacks de erro vazios e a
exigência de f32 não têm decisão registrada.

## Solução

- **D1/D4/D5:** `open_input` isolado; falha → aviso e sem entrada. Fone ausente → padrão do sistema. Buffer limitado à
  faixa do dispositivo (`fit_buffer`).
- **D3:** `pick_config` (f32 > i32 > i24 > i16 > u16) e streams genéricos com conversão em buffers pré-alocados.
- **D2:** os callbacks de erro marcam `LOST_OUTPUT`/`LOST_INPUT` na telemetria. Na thread de telemetria:
  - `check_reopen` só segura a trava do áudio e decide;
  - `reopen` segura áudio e estúdio e reabre;
  - a volta de um dispositivo reabre só se a lista de ids mudou (sem laço: AU-02) e nunca durante a gravação;
  - evento `audio-reopened` → aviso na tela, status e sessão relidos (AU-07).
- **AU-01:** fatia da entrada limitada ao tamanho real.
- **AU-03:** descarte do excesso da fila do microfone.
- **AU-05/AU-06:** rotas só dos canais existentes.
- **AU-10:** modo silencioso visível no diálogo.

## Por que esta solução

O app nunca fica mudo por causa do microfone e se recupera sozinho de desconexões. As decisões viraram funções puras
testáveis sem hardware.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Tentar reabrir a cada 2 s sempre que faltar algo | Laço infinito com microfone presente que não abre (AU-02) |
| Abrir na taxa nativa sem conversor | Tom e velocidade errados (AU-16); o conversor fica como sugestão S-1 |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [crates/audio-engine/src/device.rs](../../crates/audio-engine/src/device.rs) | `pick_config`, `fit_buffer`, `build_output`/`build_input`, `open_input`, `open_streams` | Abertura tolerante e formatos |
| [crates/audio-engine/src/telemetry.rs](../../crates/audio-engine/src/telemetry.rs) | `mark_device_lost`/`take_device_lost` | Sinal de dispositivo perdido |
| [crates/audio-engine/src/core.rs](../../crates/audio-engine/src/core.rs) | `process` | AU-01 |
| [crates/app-desktop/src/audio.rs](../../crates/app-desktop/src/audio.rs) | `reopen_reason`, `check_reopen`, `reopen` | Reabertura |
| [crates/app-desktop/src/main.rs](../../crates/app-desktop/src/main.rs) | thread de telemetria | Supervisão e evento |
| [ui/src/core/live.ts](../../ui/src/core/live.ts) | `audio-reopened` | Aviso e releitura |

## Lógica

- **Antes:** microfone falha → abertura falha → modo silencioso. Dispositivo cai → stream morto para sempre.
- **Depois:** microfone falha → saída abre sozinha, com aviso. Dispositivo cai → telemetria marca → app reabre e avisa.
  Dispositivo volta → a lista muda → app reabre com ele.

## Exemplo

Microfone USB salvo e desconectado antes de abrir o app: antes, nenhum som. Agora o som sai no fone e a barra de status
mostra "Microfone indisponível (microfone não encontrado (desconectado?)): o áudio segue sem entrada.". Ao religar o
microfone, em até 2 s ele volta a ser usado.

## Verificação

- **TDD:**
  - `pick_config` (inteiros aceitos, preferência por f32, canais, taxa fora da faixa);
  - `fit_buffer`;
  - telemetria `device_lost`;
  - `reopen_reason` (5 testes, incluindo o laço do AU-02);
  - AU-01 com 4 combinações;
  - `trackRouteOptions`;
  - E2E `audio-devices.spec.ts`.
- **Sabotagem:**
  - `pick_config` só f32 → teste falha;
  - sem a regra de lista mudada → teste do laço falha;
  - sem o limite do AU-01 → teste de pânico falha.
- **Verificação 1:** `cargo fmt --check`; `cargo clippy --workspace --all-targets -D warnings`; `cargo test --workspace`
  com 67 suítes e 680 testes; `tsc`; Vitest 52 arquivos / 342 testes; Playwright 80/80.
- **Verificação 2 (outra abordagem):** `cargo test --release -p audio-engine --features device` (inclui `no_alloc`);
  sabotagens; `vite build`; `check_readmes.py`.
- **Incidente:** o disco encheu durante a primeira rodada (falhas falsas na compilação e nos E2E). Liberado com
  `cargo clean`, arquivos conferidos, verificações refeitas do zero.

## Riscos

- `open_streams` não roda sem hardware neste ambiente: abertura, conversão de formato e reabertura precisam ser
  confirmadas num computador real.
- A taxa não suportada pelo fone (D6) continua levando ao modo silencioso, agora com mensagem clara. Sugestão S-1.
- Listar dispositivos a cada 2 s acontece só quando falta algo, e só com a trava do áudio.
