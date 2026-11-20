# Varredura 6 — bugs de áudio (2026-11-17)

| Campo | Valor |
|---|---|
| Id | REL-2026-11-17-varredura-6-audio |
| Título | "Não consigo escutar nada" e entradas que ficam bugadas depois de desconectar |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("verifique por bugs, não consigo escutar nada, entradas de áudio que estavam conectadas e são desconectadas ficam bugadas, faça uma varredura com foco em bugs de áudio") |
| Versão analisada | v0.4.0 (`a0ad534`) |
| Status | correções críticas implementadas (VER-2026-11-17-11); demais itens como sugestão (seção 3) |

## 1. Causas encontradas para "não ouço nada"

| # | Causa | Onde | Efeito |
|---|---|---|---|
| D1 | Qualquer falha ao abrir o **microfone** abortava a abertura inteira: microfone salvo que foi desconectado, sem a taxa do projeto, ou com o buffer recusado | `audio-engine/src/device.rs` `open_streams` (`find(..)?` e demais `?` da entrada) | O app caía no **modo silencioso**: a saída também morria |
| D2 | Desconexão com o app aberto não era tratada: o erro da entrada era ignorado (`\|_e\| {}`) e o da saída só contava um xrun | `device.rs` (callbacks de erro) | Fone mudo para sempre; microfone "conectado" com medidores parados até reconfigurar |
| D3 | Só dispositivos que oferecem **f32** | `device.rs` `f32_config` | Microfones USB e placas no Linux com 16/24/32 bits inteiros eram recusados. Somado ao D1, deixava o app mudo |
| D4 | Fone escolhido que sumiu derrubava tudo, sem usar o padrão do sistema | `device.rs` (`find` da saída) | Modo silencioso |
| D5 | Buffer pedido fora da faixa aceita pelo dispositivo | `device.rs` (`BufferSize::Fixed` sem limitar) | O stream não abria: modo silencioso |
| D6 | Taxa do projeto não suportada pelo fone (Bluetooth a 44,1 kHz, ou 16 kHz quando o microfone do fone está em uso) | `device.rs` (taxa exata) | Modo silencioso. **Não corrigido**: exige um conversor de taxa no tempo real (sugestão S-1) |

**Dupla verificação do diagnóstico:**
1. Leitura do caminho de abertura e dos callbacks.
2. Varredura independente por um segundo agente, com fuzz de 816 combinações do `process` e teste de alinhamento da
   gravação.

Não foi encontrado nenhum estado de silêncio alcançável pela interface com uma saída funcionando: master audível, solo,
loop e contagem estão corretos.

## 2. Corrigido nesta entrega (VER-2026-11-17-11)

- **D1/D4:** microfone que não abre → o áudio segue **só com a saída** e avisa. Fone que sumiu → saída padrão do
  sistema, com aviso.
- **D2:** `DeviceNotAvailable`/`HostUnavailable` marcam a telemetria. O app reabre o áudio, avisa na tela e relê a
  sessão (take finalizado). Quando o dispositivo que faltava **volta**, o áudio é reaberto com ele.
- **D3:** f32, i32, i24, i16 e u16, com conversão no callback (buffers pré-alocados).
- **D5:** buffer limitado à faixa do dispositivo.
- **AU-01:** pânico latente no `process` com entrada mais curta que a saída.
- **AU-02:** a reabertura automática entrava em laço com um microfone presente que não abre. Agora só reabre quando a
  lista de dispositivos **muda**, e nunca no meio de uma gravação.
- **AU-03:** a fila do microfone acumulava latência: o excesso acima de 2 callbacks é descartado.
- **AU-05:** a entrada da faixa mostrava canais inexistentes. Agora mostra só os do microfone atual, e uma rota que
  sumiu aparece "(indisponível)".
- **AU-06:** a rota do "Ouvir no fone" é refeita quando o número de canais muda.
- **AU-07:** depois de reabrir sozinho, status e sessão são atualizados na tela.
- **AU-10:** o diálogo mostra o modo silencioso e os avisos; "Testar som" fica desativado sem saída.

## 3. Sugestões (aguardam decisão — formato §5 resumido)

| Id | O que | Solução sugerida | Por que | Esforço |
|---|---|---|---|---|
| S-1 (D6, AU-16) | Fone que não aceita a taxa do projeto deixa o app mudo | Conversor de taxa em tempo real na saída (e na entrada), abrindo o dispositivo na taxa nativa | Fones Bluetooth e alguns USB só têm 44,1 kHz ou 16 kHz (modo chamada); hoje a única saída é mudar a taxa do projeto | G |
| S-2 (AU-04) | "Aplicar" sempre grava um buffer fixo (256), mesmo com o driver no padrão; a latência mostrada e a compensação ficam erradas no WASAPI compartilhado | Opção "Padrão do driver (recomendado)" e publicar o tamanho real do callback | Takes atrasados com "Usar estimativa" | M |
| S-3 (AU-09) | Audições de Harmonia/Bases/Partitura tocam pela WebAudio na saída padrão do sistema, não no fone escolhido | Tocar as audições pelo motor (mesma saída) | Confunde o diagnóstico de "não ouço nada" | M |
| S-4 (AU-08) ✅ VER-2026-11-20-01 | Afinador lê a contagem de canais uma vez só; depois de trocar o microfone dá "afinador não está ligado" | Reler o status e religar ao mudar `inputChannels` | Estado velho após troca | P |
| S-5 (AU-11) ✅ VER-2026-11-20-01 | xruns misturam pânico, erro de dispositivo e underrun; "aumente o buffer" é cumulativo | Contadores separados e janela de tempo | Mensagem enganosa | P |
| S-6 (AU-12) ✅ VER-2026-11-20-01 | `stop_recording` espera até 1 s com a trava do estúdio quando o núcleo morreu | Esperar fora da trava | Interface trava ~1 s na reabertura durante gravação | P |
| S-7 (AU-13/14/15) | Proteções latentes: canal ≥ entradas no monitor/gravador (hoje validado antes), ganho de entrada desligado sem aviso após pânico capturado, quadros de entrada extras não gravados (inalcançável hoje) | Defesas e aviso | Robustez | P |

## 4. Verificação

Ver VER-2026-11-17-11:
- Rust: 67 suítes, 680 testes; fmt e clippy limpos; motor em release.
- UI: Vitest 52 arquivos / 342 testes; Playwright 80/80; build.
- Sabotagens confirmadas.

**Não verificado:** dispositivos reais (o ambiente não tem placa de som). A abertura de streams (`open_streams`) só roda
com hardware; as decisões foram extraídas para funções puras e testadas.

---

## Anexo — achados detalhados da varredura independente


Data: 2026-11-17. Escopo: audio-engine (core, input_gain, test_tone, graph, recorder, input_tap,
metronome, null_device, handle, telemetry), app-core (studio, compile, device_monitor, input_gain,
tuning_service), app-desktop (audio.rs, commands.rs, main.rs) e a UI (core/audio.ts,
AudioSettingsDialog, recording/*, tuning, Workspace).
Já conhecido e fora deste relatório: device.rs `open_streams` (falha de entrada derruba tudo, erros de
stream ignorados, só F32, taxa exata) e o fallback do audio.rs. **Observação:** device.rs, audio.rs e
telemetry.rs estavam sendo alterados durante a varredura (WIP do pai). AU-02 e AU-07 tratam desse WIP.

Testes descartáveis (fora do repositório): matriz de 816 casos para `process`
e teste de alinhamento da gravação.

---------------------------------------------------------------------------------------------------
### CONFIRMADOS

### AU-01: pânico no RT quando a entrada tem menos quadros que a saída (média: latente, correção de 1 linha)
- **Onde:** crates/audio-engine/src/core.rs:325-326
  ```rust
  let in_n = in_frames.saturating_sub(done).min(n);
  let raw = &input[done * in_channels..(done + in_n) * in_channels];
  ```
- **O que acontece:** quando `done > in_frames`, `in_n` vale 0, mas o início do slice
  (`done*in_channels`) passa de `input.len()` → pânico. O pânico é capturado no backend: o callback
  inteiro vira silêncio, conta um xrun e isso se repete em todo callback com essa forma. Além disso,
  o `input_scratch` fica perdido (`mem::take`) e o ganho de entrada some até a engine ser recriada.
- **Evidência (fuzz):** 120 de 816 casos entram em pânico, todos nesta linha. Exemplos:
  in_ch=2, in=1023, out=1025 → "range start index 2048 out of range for slice of length 2046";
  in_ch=8, in=3000, out=5000 → "range start index 24576 … length 24000";
  in_ch=100, in=3000, out=5000 → "327500 … 300000". Nenhum outro pânico nem saída silenciosa nas
  combinações in_ch 0/1/2/3/8/65/100/70000 × out_ch 1/2/6 × tamanhos ímpares/maiores que MAX_BLOCK ×
  tocar/gravar/contagem/loop.
- **Alcance hoje:** device.rs completa a entrada com zeros até `frames*in_ch`, então in==out. Na
  versão antiga, só com callback > 16384 quadros. No WIP, `in_buf = CB_FRAMES*in_ch` e a saída é
  processada em pedaços de CB_FRAMES, ou seja, também in==out. **Qualquer mudança no backend que
  entregue só `avail` (sem completar) dispara o pânico** → silêncio + xruns.
- **Correção:** `let a = (done * in_channels).min(input.len()); let raw = &input[a..a + in_n * in_channels];`
  e um teste de regressão (in=1023, out=1025, in_ch=2).

### AU-02: o WIP `supervise()` reinicia o áudio a cada 2 s quando um dispositivo existe mas não abre (alta, WIP)
- **Onde:** crates/app-desktop/src/audio.rs (diff de trabalho: `reopen_reason`, `supervise`, `Missing`) +
  crates/audio-engine/src/device.rs `open_input` (qualquer erro vira `input_missing = true`).
- **Cenário 1:** microfone listado, mas que falha ao abrir. Exemplos: Windows › Privacidade ›
  Microfone bloqueado, dispositivo em modo exclusivo em outro app, taxa não suportada, falha de
  build. `input_missing=true` → 2 s depois `reopen_reason` encontra o id na lista → `DeviceBack` →
  `start()` → falha de novo → laço infinito.
- **Cenário 2:** saída que não abre (ex.: `UnsupportedRate`, ainda fatal para a saída). Cai em
  NullDevice com `missing.silent=true` → `!list.outputs.is_empty()` → `DeviceBack` a cada 2 s, para
  sempre.
- **Efeito de cada reinício:** `detach_engine` (finaliza a gravação em curso, espera até 1 s com o
  lock do studio), nova engine, transporte parado e posição em 0, recompilação de todos os efeitos
  (as caudas somem), estalo e `list_devices()` (lento no WASAPI) dentro dos locks. A reprodução fica
  inutilizável e a UI trava periodicamente.
- **Correção:** separar "não encontrado" (`NotFound`) de "encontrado mas falhou". Reabrir só se o
  conjunto de ids mudou desde a última tentativa (guardar um snapshot), com backoff exponencial.
  Nunca reiniciar sozinho durante gravação. Para `DeviceBack`, abrir primeiro a entrada como teste
  antes de derrubar a engine atual.

### AU-03: a fila de entrada nunca escoa: atraso fixo desde a abertura + deriva entre relógios (média-alta para gravação)
- **Onde:** device.rs (antigo L229-299; no WIP, `open_input` + `render`): a entrada é aberta e
  **iniciada antes** da saída. A saída lê só `min(slots, frames*in_ch)` por callback.
- **O que acontece:**
  1. Tudo que a entrada acumulou entre o `play()` dela e o primeiro callback de saída (montar o
     stream de saída no WASAPI leva dezenas de ms) fica na fila **para sempre**: latência extra
     constante na monitoração (faixa e "Ouvir no fone") e takes atrasados além da compensação.
  2. Com microfone e fone em relógios diferentes (mic USB + saída onboard, caso comum), não há
     ressincronização. Mic 48 003 Hz × saída 48 000 Hz: +3 quadros/s, a fila (0,5 s) enche em
     ~2,2 h; a latência cresce de 0 até 500 ms e depois a entrada descarta pedaços (xruns). Com o mic
     mais lento: falta amostra periodicamente → zeros inseridos → estalos na monitoração e na
     gravação.
- **Evidência:** certo pelo código (não há descarte do excesso nem resampling adaptativo). A
  magnitude depende do hardware (a confirmar em máquina real).
- **Correção:** no 1º callback de saída, descartar a fila. Em cada callback, se
  `slots > alvo` (ex.: 2×frames), pular o excesso (descartar as amostras mais antigas) e contar um
  evento. Para dispositivos diferentes, resampler adaptativo ou um aviso na UI ("use a mesma
  interface para entrada e saída").

### AU-04: o diálogo sempre envia um buffer fixo (256 por padrão); latência estimada e "Usar estimativa" ficam errados (média)
- **Onde:** ui/src/app/AudioSettingsDialog.tsx:66 (`useState(256)`), :79 (`if (s.bufferSize) setBuffer`),
  :94 (`bufferSize: buffer`). audio.rs:58-60 (latência = 2×buffer/sr). RecordingPanel.tsx:144-147.
- **O que acontece:** com o driver no padrão (`bufferSize=null`), o diálogo mostra 256 e "Aplicar"
  grava `Fixed(256)`, que persiste em `audio.request`. No WASAPI compartilhado (cpal 0.18,
  device.rs:847-849: "the callback period is always GetDevicePeriod() regardless of what is
  requested"), o callback real é o período do dispositivo (~480 quadros a 48 kHz). Mesmo assim a UI
  anuncia "buffer 256 · ~10,7 ms", e "Usar estimativa (512)" grava uma compensação de latência muito
  menor que a real (ida e volta típica de 20–40 ms = 960–1900 quadros) → takes atrasados. Em
  ALSA/ASIO, um `Fixed(256)` fora da faixa pode falhar (o WIP limita pelo Range; com `Unknown`
  continua Fixed).
- **Correção:** opção "Padrão do driver" (null) que só muda se a pessoa mexer. Publicar na
  telemetria o tamanho real do callback (`data.len()/out_ch`) e calcular a latência com ele.
  Idealmente, medir a latência (loopback) em vez de 2×buffer.

### AU-05: a entrada da faixa oferece canais que não existem; uma rota antiga parece válida depois da troca de dispositivo (média)
- **Onde:** ui/src/contexts/recording/RecordingPanel.tsx:20 `routeOptions(channels)` (mínimo 2) +
  inputRoutes.ts:30; backend app-core/src/studio.rs:822-831 e compile.rs:139-145.
- **Cenário:** interface 2 canais, faixa com "Entradas 1/2 (estéreo)"; troca para um mic de 1 canal
  (2→1). O seletor continua mostrando "Entradas 1/2 (estéreo)" como opção válida, porque o mínimo é
  2. O `compile` vira mono em silêncio (right filtrado). Ao gravar: erro "A entrada escolhida não
  existe no dispositivo de áudio atual". Com 0 canais, também aparecem 2 entradas. Numa troca 8→2,
  uma faixa em "Entrada 5" perde a monitoração em silêncio e não grava.
- **Correção:** `routeOptions(channels, 1)` e, se a rota atual não couber, mostrar uma opção
  "Entrada X (indisponível)" com aviso. Opcional no backend: avisar no `attach_engine` quais faixas
  armadas ficaram sem rota, ou fazer fallback para mono no canal 0 (decisão do usuário).

### AU-06: "Ouvir no fone" (InputSection) guarda uma rota que não existe mais (média-baixa)
- **Onde:** ui/src/contexts/recording/InputSection.tsx:52-70. `route` só é atualizada quando
  `audioMonitorGet()` traz um monitor. Quando o backend desligou a monitoração
  (`reapply_device_monitor` com a rota fora do dispositivo), `route` continua "0:1".
- **Cenário:** 2→1 canal. O `<select>` controlado com valor fora das opções mostra "Entrada 1
  (mono)", mas o valor real é "0:1". Ligar o toggle → `audio_monitor_set{left:0,right:1}` → toast
  de erro "entrada não existe". Para a pessoa, "o microfone ficou bugado depois de trocar".
- **Correção:** depois do refresh, se `route` não estiver em `options`, voltar para `options[0]`.
  O mesmo vale para `AudioSettingsDialog` (route fixo {0,null}, que está ok).

### AU-07: a UI não fica sabendo de reaberturas automáticas do áudio (média, WIP)
- **Onde:** ui/src/core/audio.ts: o store `useAudio` só é atualizado por chamadas explícitas
  (`refreshAudio`, `setInputDevice`, diálogo). Não existe `listen("audio-status")` (grep: só existe
  `tauri://drag-drop`).
- **Efeito com o WIP `supervise()`:** depois de "microfone desconectado", a barra de status continua
  mostrando o mic. `InputSection`/`RecordingPanel` seguem com `inputChannels=2` (medidores, opções de
  rota, botão de monitoração). Um take finalizado pelo reinício automático (`detach_engine` →
  `stop_recording` → `execute`) não aparece na timeline, porque nenhum SessionView é emitido, e a UI
  continua achando que está gravando.
- **Correção:** emitir `audio-status` (AudioStatus) e `session` (SessionView, se houve take) a partir
  do supervisor. Na UI, `listen` → `useAudio.setStatus` / `useSession.setSession` e um toast com
  `Reopen::message()`.

### AU-08: o afinador usa a contagem de canais lida uma vez só e é desligado em silêncio na troca de dispositivo (baixa)
- **Onde:** ui/src/contexts/tuning/TuningPanel.tsx:33-35 (`api.audioStatus()` só na montagem, com
  mínimo 1); app-core/studio.rs:160 (`self.tuner = None` no attach).
- **Efeito:** depois de 2→1, o seletor continua com "Entrada 2" → `tuner_start(1)` → erro. Com o
  afinador ligado durante a troca, o próximo `tunerRead` dá erro "O afinador não está ligado" (toast
  confuso).
- **Correção:** usar `useAudio` e, ao mudar `inputId`/canais, limitar o canal e religar o afinador
  automaticamente se ainda couber.

### AU-09: as audições (Harmonia, Bases, Partitura) tocam pela WebAudio, na saída padrão do SO (baixa-média para diagnóstico)
- **Onde:** ui/src/core/audition.ts:16-60; ui/src/contexts/notation/ScorePanel.tsx:352-366.
- **Efeito:** com uma saída específica escolhida nas Configurações (ex.: interface USB) e a saída
  padrão do SO muda ou desligada, as audições ficam mudas enquanto a timeline toca. No inverso (engine
  em modo silencioso), a pessoa ouve as audições e conclui que "o som funciona". Isso atrapalha o
  diagnóstico de "não ouço nada".
- **Correção:** tocar as prévias pela engine (um "preview player" fora do master) ou deixar claro na
  UI.

### AU-10: as Configurações de áudio não mostram o modo silencioso; "Testar som" finge tocar (baixa)
- **Onde:** AudioSettingsDialog.tsx: não renderiza `status.mode`/`error`/`warning`. `testTone` mostra
  "Tocando…" e a dica culpa o volume do SO, mesmo com NullDevice.
- **Correção:** banner com `status.error`/`warning` e, no modo "null", desabilitar "Testar som" ou
  explicar.

### AU-11: o contador de xruns mistura coisas e nunca zera (baixa)
- **Onde:** device.rs (pânico capturado → `add_xrun`; erro de stream → `add_xrun`), Workspace.tsx:340.
- **Efeito:** um pânico do RT (ex.: AU-01) ou um erro de driver aparece como "N falha(s) de buffer —
  aumente o buffer", um conselho errado, e o contador é cumulativo.
- **Correção:** contadores separados (underrun / pânico / dispositivo) e mensagem por tipo.

### AU-12: `stop_recording` espera até 1 s com o lock do studio quando o núcleo parou (baixa)
- **Onde:** app-core/src/studio.rs:870-880. Com a saída perdida (callback não roda mais), o estado
  nunca vira Stopped → espera de 1 s. O take é preservado (o `finish` drena a fila). No WIP isso
  acontece na thread de telemetria (supervise) → UI congela ~1 s a cada reabertura.
- **Correção:** se `device_lost`, pular a espera; ou esperar fora do lock.

### AU-13: (defensivo) monitoração da faixa e gravador leem o quadro vizinho quando canal ≥ in_channels (baixa)
- **Onde:** core.rs:561-569 (`input.get(base + m.left)`), recorder.rs:233/244 (`input.get(base + tap.left)`).
- **Efeito:** com in_channels=1 e right=1, lê a amostra do quadro seguinte (lixo deslocado), não
  silêncio. Hoje está protegido por compile.rs:141-143 e studio.rs:822-831 (validação com
  `Studio.input_channels`). Fica alcançável se a engine passar a receber outra contagem de canais
  sem `attach_engine`.
- **Correção:** `if m.left < in_channels` / `r < in_channels` antes de ler.

### AU-14: o rascunho perdido num pânico desliga o ganho de entrada em silêncio (baixa, por desenho)
- **Onde:** core.rs:312-315 e o teste `without_scratch_the_input_passes_without_gain_instead_of_panicking`.
  Depois de um pânico capturado, o ganho escolhido (ex.: +12 dB) deixa de valer até a próxima troca de
  dispositivo, sem aviso. Sugestão: devolver o scratch com um guard (Drop) ou sinalizar na telemetria.

### AU-15: com in_frames > out_frames, os quadros extras de entrada não são gravados (baixa, inalcançável hoje)
- core.rs:338-340: `process_block` só roda para `out_n>0`. Os quadros extras só alimentam medidores e
  tap. O backend atual entrega in==out.

---------------------------------------------------------------------------------------------------
### VERIFICADOS SEM ERRO (para responder a–d)

- Alinhamento da gravação (bin/rec.rs): blocos 480/1500/4096/333 com e sem contagem → quadros gravados
  == posição − record_start, sem buracos, e o 1º quadro gravado é o correto (96001 após 1 compasso de
  contagem).
- `process` com in_ch 0..70000, out_ch 1/2/6, blocos > MAX_BLOCK e ímpares: saída sempre escrita, sem
  silêncio indevido (fora AU-01).
- Silêncio por estado (c): master começa audível (`!master.mute`), volume UNITY. Solo só silencia
  faixas sem solo. `loop_range` inválido é filtrado (compile.rs:164) e validado
  (graph.rs `validate`). A contagem termina sozinha. Play é ignorado só em CountIn/Recording. Depois
  de `attach_engine`, o `sync()` reenvia o grafo. Não achei estado preso alcançável pela UI.
  Ressalvas: o `let _ = studio.attach_engine(..)` (audio.rs:74/94) engole erro de `sync`. Uma faixa
  sem strip no mixer é inaudível (`is_track_audible` → false), mas todos os caminhos que criam faixa
  adicionam strip.
- (d) Entrada some no meio do take: a fila esvazia → a saída completa com zeros → o take continua com
  silêncio (sem aviso; o WIP marca LOST_INPUT). Finalização: o `finish()` drena e finaliza o WAV mesmo
  com a engine morta. Sem `record_start` (contagem não terminou ou punch fora), os arquivos são
  apagados. Latência com in_channels 0: impossível gravar (validação), nada a compensar.
- (a) Painéis por item depois de 2→0/2→1: monitor do dispositivo desligado se a rota não cabe (ok).
  Ganho reaplicado (ok). Afinador anulado (AU-08). Rotas das faixas filtradas no compile e gravação
  recusada (AU-05). Medidores: telemetria limita a in_channels; UI pode ficar com contagem velha
  (AU-07). Nenhum caminho de pânico RT além de AU-01 (revisei slicing/indexação em core, graph
  `mix_into`, metrônomo, tap, recorder, test tone, telemetria).

### A CONFIRMAR / NOTAS PARA O FIX EM ANDAMENTO
- **AU-16:** se o fix do item conhecido (4) abrir o dispositivo numa taxa ≠ projeto, o núcleo toca
  com tom/velocidade errados. Os clipes são decodificados na taxa do projeto, `Click`/`InputGain`/
  `TestTone` usam a taxa da engine e não há resampler no caminho. Além disso, `ensure_rate` compara só
  `request.sample_rate`. Converter para a taxa do dispositivo exige resampler ou recompilar o grafo
  nessa taxa.
- **AU-17 (baixa):** `SetDeviceMonitor(Some)` com outra rota troca de canal na hora, com o ganho
  atual → estalo (core.rs:254).
- Nota cpal 0.18 WASAPI: "Padrão do sistema" (output_id None) usa o endpoint virtual que segue a troca
  de dispositivo padrão do Windows. A troca de padrão **não** é um bug aqui.
- ASIO (`feature asio`) não está ativado no release. Se for ativado, `host()` escolhe ASIO sem
  fallback e máquinas sem driver ASIO caem em NoOutput (silêncio).
