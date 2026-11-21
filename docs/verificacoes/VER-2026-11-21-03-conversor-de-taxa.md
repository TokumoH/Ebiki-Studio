# VER-2026-11-21-03 — Conversor de taxa em tempo real (S-1 / D6 / AU-16)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-21-03 |
| Título | Fone ou microfone que não aceita a taxa do projeto abre na taxa mais próxima, com conversão em tempo real |
| Gravidade | alta (fone Bluetooth/USB deixava o app em modo silencioso; microfone era descartado) |
| Tipo | correção |
| Data | 2026-11-21 |
| Usuário responsável | TokumoH (aprovou S-1 em 2026-11-21) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `crates/audio-engine/src/{resampler.rs (novo),device.rs,lib.rs}`, `crates/audio-engine/tests/{rate_conversion.rs (novo),no_alloc.rs}`, `crates/audio-engine/README.md`, `crates/app-desktop/src/audio.rs`, `ui/src/ipc/{schema.ts,mock.ts}`, `ui/src/app/{AudioSettingsDialog.tsx,AudioSettingsDialog.test.tsx,helpTopics.ts}`, `ui/src/core/{audio.ts,audio.test.ts (novo),README.md}`, `ui/src/contexts/recording/RecordingPanel.tsx`, `docs/manual/README.md`, `docs/relatorios/2026-11-17-varredura-6-audio.md`, `CHANGELOG.md`, `docs/verificacoes/README.md` |

## Problema

`open_streams` ([device.rs](../../crates/audio-engine/src/device.rs)) exigia que o dispositivo aceitasse a taxa
**exata** do projeto (`pick_config(..., req.sample_rate, ...)`):

- **Saída:** fones Bluetooth que só abrem a 44,1 kHz (ou a 16 kHz quando o microfone deles está em uso, perfil
  HFP) e algumas placas USB → `UnsupportedRate` → o app caía no `NullDevice` (**modo silencioso**). A única saída
  era trocar de fone ou criar o projeto a 44,1 kHz.
- **Entrada:** a mesma regra; desde o VER-2026-11-17-11 o microfone que falha não derruba mais a saída, mas é
  descartado ("Microfone indisponível (o microfone não funciona a 48 kHz)…").

A engine (`EngineCore`) **não pode** simplesmente rodar na taxa do dispositivo: os clipes são decodificados na taxa
do projeto, e metrônomo, ganho de entrada e tom de teste usam a taxa da engine — mudar a taxa mudaria altura e
velocidade (AU-16 da varredura 6).

## Era proposital?

Era uma limitação conhecida, não uma regra: o relatório
[2026-11-17-varredura-6-audio.md](../relatorios/2026-11-17-varredura-6-audio.md) registra D6 como **"Não corrigido:
exige um conversor de taxa no tempo real (sugestão S-1)"** e a nota AU-16 descarta abrir o dispositivo em outra taxa
**sem** conversor. O VER-2026-11-17-11 e o CHANGELOG da 0.4.1 ("um fone que não aceita a taxa do projeto ainda
precisa trocar de fone ou de taxa") deixaram o caso em aberto de propósito, à espera desta sugestão. Nenhum ADR
proíbe conversão de taxa; o ADR-0005 (núcleo sem alocação) é respeitado (ver Verificação).

## Solução

1. **Novo Pure Fabrication [`resampler.rs`](../../crates/audio-engine/src/resampler.rs)** (sem dependências novas):
   - `Resampler`: streaming, f32 intercalado, multicanal, razão arbitrária entre duas taxas inteiras. Sinc com janela
     de Kaiser (β = 7, ≈ 70 dB), **48 coeficientes** subindo a taxa e `48 × from/to` reduzindo (corte e janela
     escalam juntos), 256 fases tabeladas com interpolação linear entre fases, linhas normalizadas (DC exato).
     Posição **racional exata** (`ipos + frac/to` com `from/to` reduzidos pelo MDC): a contagem de quadros nunca
     deriva. NaN/∞ viram 0 na entrada. Tudo alocado em `new`; `push`/`pull` só copiam e deslocam.
   - `OutputRateAdapter` (pull): o callback pede N quadros na taxa do dispositivo; o adaptador calcula
     (`input_needed`) exatamente quantos quadros da taxa do projeto faltam, chama a engine em blocos ≤ `CB_FRAMES` e
     entrega **sempre** N quadros.
   - `InputRateAdapter` (push): converte o bloco do microfone para a taxa do projeto antes da fila de entrada.
2. **[device.rs](../../crates/audio-engine/src/device.rs):**
   - `pick_rate` (pura): a taxa do projeto se alguma configuração utilizável a aceita; senão a aceita mais próxima,
     preferindo as taxas comuns (48/44,1/96/88,2/32/… kHz) às pontas de faixa incomuns e, no empate, a maior.
   - Saída: taxa igual → caminho direto de antes (sem custo); diferente → engine → `OutputRateAdapter` → dispositivo.
     A telemetria `callback_frames` e o prazo de xrun usam os quadros e a taxa **do dispositivo**.
   - Entrada: `InputRateAdapter` entre o callback do microfone e a fila (gravação, medidores, monitores e afinador
     continuam vendo a taxa do projeto).
   - `RunningInfo.device_sample_rate` e `input_device_rate`; avisos `rate_warning` ("Fone a 44,1 kHz: o áudio é
     convertido de 48 kHz em tempo real." / "Microfone a 16 kHz: a entrada é convertida para 48 kHz em tempo real.").
3. **[audio.rs](../../crates/app-desktop/src/audio.rs):** `AudioStatus.device_sample_rate` (`deviceSampleRate`);
   latência estimada (`round_trip_ms` na abertura e em `live_status`) pela taxa do dispositivo (`latency_rate`). O
   aviso entra em `status.warning` (já junta `RunningInfo.warnings`; aparece na barra de status e no diálogo).
4. **UI:** `deviceSampleRate` no schema/mock; topo do diálogo "Sistema de áudio: WASAPI · 48 kHz · convertendo para
   44,1 kHz" (`conversionNote`); latência do diálogo na taxa do fone; "Usar estimativa" da Gravação
   (`latencyEstimateSamples`) converte os quadros do dispositivo para amostras do projeto. Ajuda, manual, CHANGELOG
   e relatório da varredura atualizados.

## Por que esta solução

- **A engine continua na taxa do projeto**: nenhuma mudança em `core.rs`/`handle.rs` (outra sessão mexe neles),
  altura e velocidade intactas, `ensure_rate` continua correto.
- **Sinc com Kaiser** é o padrão de conversores de qualidade (libsamplerate, speexdsp, soxr usam variações): com 48
  coeficientes, o erro medido contra o seno ideal é ≤ 1,1e-4 (≈ −73 dB da amplitude) a 1 kHz, a resposta é plana
  (0,00 dB) até 0,8 do Nyquist da menor taxa e a rejeição de alias, −77,2 dB.
  Custo: ~48 MAC por amostra por canal (≈ 5 M MAC/s em estéreo a 48 kHz), irrelevante perto do mix.
- **Filtro que cresce ao reduzir a taxa:** com tamanho fixo, a transição (≈ 0,09 ciclo/amostra **de entrada**)
  ficava larga em relação à saída em reduções grandes — 192 → 48 kHz errava 2,5 % a 15 kHz (achado da verificação
  com outros parâmetros, ver abaixo). Escalando, a transição fica fixa em relação à taxa de saída.
- **Pull exato na saída:** o callback sempre recebe todos os quadros (sem silêncio inserido, sem FIFO crescendo) e a
  engine é chamada só com o necessário; a posição racional elimina deriva em sessões longas.
- **48 coeficientes em vez de 16–32:** a largura da transição da janela de Kaiser é inversamente proporcional ao
  tamanho (com 32 seria ~50 % mais larga, e a banda plana terminaria antes); com 48 a resposta medida é 0,00 dB até
  0,8 do Nyquist (≈ 17,6 kHz a 44,1 kHz), −0,6 dB a 0,85 e −6 dB a 0,9, com ~0,5 ms de atraso — suficiente para
  monitoração e reprodução em fones Bluetooth (os codecs cortam ~16–20 kHz).

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Rodar a engine na taxa do dispositivo | Clipes, metrônomo, tom e ganho estão na taxa do projeto: altura/velocidade erradas (AU-16) ou recompilar/redecodificar tudo |
| Crate `rubato` | Dependência nova e maior (FFT); a API de blocos fixos complica o pull exato; o sinc próprio tem ~200 linhas testadas e sem alocação verificada |
| Interpolação linear/cúbica (Hermite) | Sem filtro anti-alias: ao reduzir 48 → 44,1 kHz, conteúdo acima de 22 kHz volta como alias audível; pré-filtro separado custaria o mesmo que o sinc |
| Sinc de 16–32 coeficientes | Transição ~50 % (32) a ~3× (16) mais larga para economizar ~2–3 M MAC/s, sem ganho perceptível |
| Taxa fixa de abertura (sempre 48 kHz) | Não resolve fones só a 44,1/16 kHz |
| Avisar e manter o modo silencioso | É o problema relatado |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [crates/audio-engine/src/resampler.rs](../../crates/audio-engine/src/resampler.rs#L64) | 1–321 (testes 322–) | Novo: `Resampler` (64), `OutputRateAdapter` (222), `InputRateAdapter` (276) + 12 testes |
| [crates/audio-engine/src/device.rs](../../crates/audio-engine/src/device.rs#L173) | 173–234 | `COMMON_RATES`, `pick_rate`, `khz`, `rate_warning` |
| [crates/audio-engine/src/device.rs](../../crates/audio-engine/src/device.rs#L403) | 403–441 | Entrada: escolhe a taxa e envolve `push` no `InputRateAdapter` |
| [crates/audio-engine/src/device.rs](../../crates/audio-engine/src/device.rs#L474) | 474–483, 518–596 | Saída: taxa do dispositivo, `engine_render` + `OutputRateAdapter`, telemetria em quadros do dispositivo, `RunningInfo` |
| [crates/audio-engine/src/device.rs](../../crates/audio-engine/src/device.rs#L674) | 673–745 | Testes de `pick_rate` e `rate_warning` |
| [crates/audio-engine/src/lib.rs](../../crates/audio-engine/src/lib.rs) | `pub mod resampler` | Módulo público (não depende da feature `device`) |
| [crates/audio-engine/tests/rate_conversion.rs](../../crates/audio-engine/tests/rate_conversion.rs) | todo | Novo: engine real + conversor mantém 440 Hz em 44,1/16/96 kHz; contagem exata; microfone convertido |
| [crates/audio-engine/tests/no_alloc.rs](../../crates/audio-engine/tests/no_alloc.rs#L113) | 111–146 | `rate_conversion_does_not_allocate` |
| [crates/app-desktop/src/audio.rs](../../crates/app-desktop/src/audio.rs#L41) | 39–50, 170–190, 243, 505–542 | `device_sample_rate`, `latency_rate`, latência pela taxa do dispositivo + 2 testes |
| [ui/src/ipc/schema.ts](../../ui/src/ipc/schema.ts#L212) | 211–212 | `deviceSampleRate` |
| [ui/src/ipc/mock.ts](../../ui/src/ipc/mock.ts#L53) | 53, 230 | `deviceSampleRate: null` |
| [ui/src/app/AudioSettingsDialog.tsx](../../ui/src/app/AudioSettingsDialog.tsx#L37) | 36–39, 148–152, 173, 276–277 | `conversionNote`, latência na taxa do fone |
| [ui/src/app/AudioSettingsDialog.test.tsx](../../ui/src/app/AudioSettingsDialog.test.tsx) | teste "fone em outra taxa" | Topo com "convertendo para 44,1 kHz" e latência a 44,1 kHz |
| [ui/src/core/audio.ts](../../ui/src/core/audio.ts#L22) | 17–27 | `latencyEstimateSamples` |
| [ui/src/core/audio.test.ts](../../ui/src/core/audio.test.ts) | todo | Novo: estimativa por buffer, callback e fone convertido |
| [ui/src/contexts/recording/RecordingPanel.tsx](../../ui/src/contexts/recording/RecordingPanel.tsx#L44) | 4, 42–44 | Usa `latencyEstimateSamples` |
| [ui/src/app/helpTopics.ts](../../ui/src/app/helpTopics.ts#L163) | 163 | "Fone em outra taxa" substitui "Modo silencioso mesmo com fone" |
| [docs/manual/README.md](../manual/README.md) | 91–95 | Fone/microfone em outra taxa: convertido |
| [CHANGELOG.md](../../CHANGELOG.md) | Não lançado › Corrigido | Item novo |
| [docs/relatorios/2026-11-17-varredura-6-audio.md](../relatorios/2026-11-17-varredura-6-audio.md) | D6, S-1, AU-16 | Marcados como resolvidos |
| [crates/audio-engine/README.md](../../crates/audio-engine/README.md), [ui/src/core/README.md](../../ui/src/core/README.md) | Onde | Arquivos novos |

## Lógica

**Antes**

```
pick_config(configs, 48000) → None (fone só 44,1 kHz) → UnsupportedRate → NullDevice (mudo)
microfone só 16 kHz → "Microfone indisponível" → sem entrada
```

**Depois**

```
pick_rate(configs, 48000) = 44100 → pick_config(configs, 44100)
saída:   callback(N quadros @44,1k) → OutputRateAdapter.fill
            need = input_needed(N)            (exato: ⌊(frac+(N−1)·from)/to⌋ + ipos + half + 1 − len)
            engine_render(≤4096 quadros @48k) × k  → push
            pull(N)                           → N quadros @44,1k (sempre)
entrada: callback(M quadros @16k) → InputRateAdapter.process → push(M) → pull(tudo) → fila @48k
taxa igual: caminho direto de antes (sem adaptador)
```

Cada quadro de saída `j` fica na posição `j·from/to` da entrada; o filtro usa `half − 1` quadros antes e `half`
depois (`half` = 24 subindo; `⌈24·from/to⌉` reduzindo). Depois de cada `pull`, o que ficou para trás é descartado
com `copy_within` (sem alocar).

## Exemplo

Projeto a 48 kHz, fone Bluetooth a 44,1 kHz, callback de 441 quadros (10 ms):

- **Antes:** modo silencioso, "o dispositivo não suporta 48000 Hz".
- **Agora:** o fone abre a 44,1 kHz; cada callback pede à engine ~480 quadros a 48 kHz (o primeiro, 478 + 27 + 1 =
  506: a antecedência do filtro, 0,56 ms) e recebe 441 quadros; o tom de teste continua 440 Hz
  (`rate_conversion.rs`). Status: `deviceSampleRate: 44100`, aviso "Fone a 44,1 kHz: o áudio é convertido de 48 kHz
  em tempo real.", latência 2 × 441 / 44 100 = 20,0 ms; "Usar estimativa" = 960 amostras do projeto (antes seriam
  882, quadros do fone tratados como amostras do projeto).

## Verificação

- **TDD:** testes de `pick_rate`/`rate_warning`, do conversor e de `conversionNote`/`latencyEstimateSamples` escritos
  com a implementação; todos falham com as sabotagens abaixo.
- **Medições** (`check_sine`, 1 kHz, amplitude 0,5, blocos irregulares 1…4096):

  | Conversão | Erro máx. vs. seno ideal | Amplitude | Frequência (cruzamentos) |
  |---|---|---|---|
  | 48 → 44,1 kHz (entrada e saída) | 8,3e-5 | +0,001 dB | 999,85–999,90 Hz |
  | 44,1 → 48 kHz (entrada e saída) | 9,6e-5 | ±0,0001 dB | 999,5 Hz |
  | 48 → 16 kHz | 3,2e-5 | −0,0006 dB | 999,5 Hz |
  | 16 → 48, 48 → 96, 96 → 44,1 kHz | ≤ 1,1e-4 | ≤ 0,001 dB | 999,5–1000 Hz |
  | Alias (23 kHz a 48 kHz → 44,1 kHz) | −77,2 dB | — | — |
  | Banda (44,1↔48, 48→16, 192→48 kHz) | 0,00 dB até 0,8 Nyquist; −0,6 dB a 0,85; −6 dB a 0,9; −23 dB a 0,95 | — | — |

  (A frequência por cruzamentos de zero tem resolução de ±0,5 Hz em 1 s; o erro contra o seno ideal, que exige
  frequência, fase e amplitude certas, é a medida forte.)

- **Verificação 1** (depois da mudança do filtro escalável, refeita do zero):
  - `cargo fmt --all -- --check` ✔; `cargo clippy --workspace --all-targets -- -D warnings` ✔ e
    `cargo clippy -p audio-engine --features device --all-targets -- -D warnings` ✔;
  - `cargo test -p audio-engine --features device` ✔ (33 unitários, 10 + 11 + 8 + 8 + 3 + 2 + 5 de integração);
  - `cargo test --release -p audio-engine --features device` ✔ (inclui `no_alloc`: núcleo + conversor sem alocar);
  - `cargo test -p app-desktop` ✔ (29, com os 2 novos);
  - UI: `npx tsc --noEmit` ✔, `npx vitest run` ✔ (54 arquivos, 350 testes), `npx vite build` ✔, Playwright
    `audio-settings.spec.ts` + `audio-devices.spec.ts` ✔ (7);
  - `python3 scripts/check_readmes.py` ✔.
- **Verificação 2 (outra abordagem/parâmetro):** varredura temporária (`zz_tmp_sweep.rs`, removida antes do commit)
  em modo release com **3 canais** (L, −L, DC 0,25), blocos 3/4096/17/1000/1, razões 44,1→22,05, 22,05→44,1,
  48→47,999 (razão quase 1, MDC = 1), 8→192, 192→48 e 32→44,1 kHz, frequências 100 Hz, 3 kHz e 0,32 × a menor taxa;
  no caminho de saída, DC exato depois do transitório e contagem `rendered` exata. Na **primeira rodada** achou o
  problema da transição larga em reduções grandes (192 → 48 kHz: erro 1,76e-2 a 15,36 kHz) → filtro escalável →
  erro 6,9e-5; todos os casos ≤ 1,8e-4 e contagem exata. **Rodada final**, depois da verificação 1 limpa: a mesma
  varredura de novo (tudo ✔, 192 → 48 kHz a 15,36 kHz: 6,9e-5), a **suíte Playwright inteira** (81 ✔, não só os
  specs de áudio) e a releitura do diff de `device.rs` (caminho direto preservado com taxa igual; quadros da engine
  por chamada ≤ `CB_FRAMES`, então `in_buf` e o descarte de atraso da entrada — AU-03 — seguem com os mesmos
  tamanhos de antes). Também a revisão manual dos invariantes: `ipos ≥ lag`
  sempre (início em `lag`, só cresce até o descarte de `ipos − lag`), capacidade
  `⌈max·step⌉ + 2·taps + 4` ≥ o que `input_needed` pede depois de um `pull` (sobra ≤ `taps` quadros), saída da
  entrada `⌈(max + 2·taps)·to/from⌉ + 2`.
- **Sabotagens** (cada uma falha pelo menos um teste; código restaurado depois):

  | Sabotagem | Teste que acusa |
  |---|---|
  | `vec!` dentro de `pull` | `rate_conversion_does_not_allocate` (release) |
  | Sem corte proporcional (anti-alias) | `content_above_the_new_nyquist_is_rejected` |
  | Sem interpolação entre fases | `other_common_ratios_work` |
  | Sem filtro de NaN | `nan_input_does_not_propagate` |
  | `input_needed` com 1 a menos | 5 testes (contagem, latência, identidade, senos) |
  | Linhas sem normalizar | `dc_is_preserved` (tolerância apertada de 1e-4 para 2e-6 para acusar) |
  | Filtro de tamanho fixo | `large_downsampling_keeps_the_passband`, `latency_is_known_and_small` |
  | `pick_rate` só taxa exata | `unsupported_rate_picks_the_closest_supported_one`, `rate_ignores_unusable_configs` |
  | Empate escolhe a menor / sem preferência por taxas comuns | `unsupported_rate_picks_the_closest_supported_one` |
  | `latency_rate` = taxa do projeto | `converted_device_latency_uses_the_device_rate` |
  | `deviceRate` = taxa do projeto / `conversionNote` vazia (UI) | "fone em outra taxa: mostra a conversão…" |

- **Nota de ambiente:** no worktree, `ui/node_modules` é um link para o checkout principal (disco); o
  `registry.test.ts` acusa "Denied ID … Bravura.woff2" pelo `server.fs.strict` do Vite (arquivo fora da raiz). Com um
  config temporário que só desliga `fs.strict`, todos os 54 arquivos passam; não é efeito desta mudança.

### Integração na `branchTrabalho` (depois do S-3)

O trabalho foi feito numa cópia isolada a partir de `1ec2ac7` e aplicado sobre `421e3d5` (S-3). Os conflitos
ficaram só na documentação (`CHANGELOG.md`, `docs/verificacoes/README.md`, `ui/src/core/README.md`) e foram
resolvidos mantendo as linhas dos dois lados. A audição do S-3 é gerada na taxa da engine (a do projeto) e passa
pelo mesmo conversor de saída, sem mudança de código.

- **Verificação 1 no estado combinado:**
  - Rust: `cargo fmt --check`; `clippy --workspace` e `clippy -p audio-engine --features device` com
    `-D warnings`; testes `audio-engine --features device` (84), `app-core` + `app-desktop` (96); `no_alloc` e
    `rate_conversion` em release; contrato IPC.
  - Docs: `check_readmes.py`.
  - UI: `tsc -b`, vitest (55 arquivos, 356 testes), `vite build`, Playwright completo (81).
  - Tudo sem erros.
- **Verificação 2 (sabotagens no estado combinado):**
  - Regra de empate de `pick_rate` invertida: o teste `unsupported_rate_picks_the_closest_supported_one` falha.
  - `input_needed` com um quadro a menos: 4 testes do conversor falham.
  - Os dois arquivos foram restaurados (`git diff` limpo) e os testes da engine rodaram de novo (84, sem erros).

## Riscos

- **Sem placa de som no contêiner:** os streams reais (cpal) não foram abertos; o caminho de dispositivo foi
  verificado por compilação, clippy e pelos adaptadores com engine real/falsa. Conferir num fone Bluetooth
  (Windows/macOS) antes da release: abertura a 44,1 kHz, aviso e ausência de estalos.
- **Banda passante:** plana até 0,8 do Nyquist da menor taxa (≈ 17,6 kHz a 44,1 kHz; ≈ 6,4 kHz a 16 kHz); acima,
  atenuação progressiva (−6 dB a 0,9 do Nyquist). A 16 kHz (HFP) o som fica abafado por limite do próprio fone.
- **Latência extra** de ~0,5 ms por sentido (1,5 ms com 16 kHz), fora da estimativa mostrada (documentado em
  `latency_rate`); a compensação manual cobre.
- **Relógios diferentes** entre entrada e saída continuam tratados pelo descarte de excesso (AU-03); a conversão não
  muda isso.
- **Custo de CPU** cresce com a razão ao reduzir (192 → 48 kHz: 192 coeficientes ≈ 18 M MAC/s em estéreo); ainda
  pequeno, mas não medido em máquina fraca.
- `DeviceError::UnsupportedRate` agora só aparece quando nenhuma configuração utilizável existe.
