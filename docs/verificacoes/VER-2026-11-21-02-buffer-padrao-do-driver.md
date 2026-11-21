# VER-2026-11-21-02 — Buffer "Padrão do driver" e latência pelo callback real (S-2 / AU-04)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-21-02 |
| Título | Opção "Padrão do driver (recomendado)" no buffer; latência e estimativa de compensação pelo tamanho real do callback |
| Gravidade | média (takes atrasados com "Usar estimativa"; latência mostrada errada) |
| Tipo | correção |
| Data | 2026-11-21 |
| Usuário responsável | TokumoH (aprovou S-2 em 2026-11-21) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `crates/audio-engine/src/{telemetry.rs,device.rs}`, `crates/app-desktop/src/{audio.rs,commands.rs}`, `ui/src/app/{AudioSettingsDialog.tsx,AudioSettingsDialog.test.tsx,Workspace.tsx,helpTopics.ts}`, `ui/src/contexts/recording/RecordingPanel.tsx`, `ui/src/ipc/{schema.ts,mock.ts}`, `docs/manual/README.md` |

## Problema

- "Aplicar" sempre enviava `bufferSize` (256 por padrão), mesmo com o driver no padrão, e isso ficava salvo.
- No WASAPI compartilhado o callback real é o período do sistema (o cpal 0.18 documenta isso). A interface mostrava
  "~10,7 ms", e "Usar estimativa" (2×256 = 512) compensava bem menos que a ida e volta real: takes atrasados.

## Era proposital?

Não. O seletor da Fase 1 não tinha a opção "sem buffer fixo", e o tamanho real do callback nunca foi publicado.

## Solução

- **Motor:** `Telemetry::set_callback_frames`/`callback_frames`, gravado a cada callback (atômico, sem alocação).
- **App:**
  - `AudioStatus.callback_frames`;
  - `live_status()` (usado por `audio_status`) lê o tamanho real e, com buffer `None`, calcula a latência por ele
    (`round_trip_ms`).
- **Interface:**
  - opção **"Padrão do driver (recomendado)"** (valor 0 → `null`), que passa a ser o padrão;
  - rótulo com a latência real e "(driver: N amostras)";
  - barra de status com "driver N";
  - "Usar estimativa" = 2 × (buffer fixo ou callback real).
- Ajuda e manual atualizados.

## Por que esta solução

- O padrão do driver é o mais estável (o sistema escolhe o período).
- Publicar o tamanho real corrige a latência mostrada e a compensação sem adivinhar o driver.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Continuar forçando 256 | Mantém a latência errada e pode causar estalos em drivers que não gostam de 256 |
| Medir a latência de ida e volta com um pulso | Mais precisa, mas exige cabo de retorno; continua disponível no ajuste manual |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [crates/audio-engine/src/telemetry.rs](../../crates/audio-engine/src/telemetry.rs) | `callback_frames` | Tamanho real |
| [crates/audio-engine/src/device.rs](../../crates/audio-engine/src/device.rs) | `render` | Registra o tamanho |
| [crates/app-desktop/src/audio.rs](../../crates/app-desktop/src/audio.rs) | `round_trip_ms`, `live_status` | Latência real |
| [crates/app-desktop/src/commands.rs](../../crates/app-desktop/src/commands.rs) | `audio_status` | Status ao vivo |
| [ui/src/app/AudioSettingsDialog.tsx](../../ui/src/app/AudioSettingsDialog.tsx) | buffer | Opção do driver |
| [ui/src/contexts/recording/RecordingPanel.tsx](../../ui/src/contexts/recording/RecordingPanel.tsx) | estimativa | Callback real |

## Lógica

- **Antes:** buffer 256 sempre salvo; latência e estimativa calculadas por 256, quaisquer que fossem as amostras reais
  do driver.
- **Depois:** buffer `null` por padrão; o driver escolhe o período; o callback real (por exemplo, 480) define a
  latência (20 ms a 48 kHz) e a estimativa (960 amostras).

## Exemplo

Windows, WASAPI compartilhado, período de 10 ms (480 amostras a 48 kHz):
- **antes:** a interface mostrava "~10,7 ms", e a estimativa era 512;
- **agora:** "~20,0 ms (driver: 480 amostras)", e a estimativa é 960.

## Verificação

- **TDD:**
  - `callback_frames_unknown_until_the_device_runs`;
  - `round_trip_counts_input_and_output`;
  - `driver_default_buffer_uses_the_real_callback_size`;
  - teste do diálogo "Aplicar não grava um tamanho fixo";
  - teste de latência atualizado (driver 480 → ~21,8 ms a 44,1 kHz; 256 → ~11,6 ms).
- **Sabotagem:**
  - `live_status` sem a regra do driver → teste Rust falha;
  - diálogo enviando 256 → teste do diálogo falha.
- **Verificação 1:** `cargo fmt --check`; clippy (`app-desktop`, `audio-engine`) sem avisos; `cargo test -p
  app-desktop -p audio-engine` ok; `tsc`; Vitest 53 arquivos / 346 testes; Playwright 81/81.
- **Verificação 2 (outra abordagem):** sabotagens; `vite build`; `check_readmes.py`.

## Riscos

- Quem já tinha salvo 256 continua com 256 até escolher "Padrão do driver" no diálogo (não mudamos a configuração salva
  sozinhos).
- O tamanho do callback pode variar em alguns drivers. Usamos o último valor visto, suficiente para a estimativa.
