# ADR-0028: Afinador por derivação lock-free da entrada na engine

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
O RF-8.1 pede um afinador de tempo real para a entrada da faixa armada. O `dsp-pitch` já tem
`TunerProcessor` (YIN, sem alocação). Faltava decidir **onde** a análise roda e como o áudio da
entrada chega a ela.

## Alternativas
1. **WebAudio/`getUserMedia` na UI**: duplica a captura (o dispositivo já está aberto pela engine
   cpal, possivelmente em modo exclusivo/ASIO), depende de permissão de microfone do WebView (instável
   no WebKitGTK/WKWebView) e mede um sinal diferente do que é gravado.
2. **Analisar dentro do callback de áudio** (`TunerProcessor` no núcleo): custo do YIN (~640 k MAC por
   quadro) na thread de tempo real, competindo com a mixagem de 32+ faixas.
3. **Derivação (tap) SPSC da entrada + análise na thread de controle (escolhida)**.

## Decisão
- `audio-engine::input_tap`: `InputTapRt` (núcleo) copia o canal escolhido para uma fila `rtrb`
  (`write_chunk_uninit`, sem alocação); com a fila cheia, o excedente é descartado (o afinador só
  precisa das amostras recentes). Mensagens `AttachInputTap`/`DetachInputTap`; o tap antigo volta pela
  fila de lixo (nunca é liberado no tempo real). `EngineHandle::attach_input_tap(canal, capacidade)`
  devolve o `InputTapReader`.
- `app-core::tuning_service::TunerSession` drena a fila a cada `tuner_read` (a UI chama ~20 Hz),
  alimenta o `TunerProcessor` só com as últimas ~250 ms e devolve nota/cents/frequência/confiança.
  A referência do Lá4 vem do agregado `TuningBook.tuner` (persistido).
- Comandos IPC: `tuner_start{inputChannel}`, `tuner_read`, `tuner_stop`.

## Consequências
- Thread de áudio continua sem alocação/bloqueio (`audio-engine/tests/no_alloc.rs` cobre captura,
  troca e remoção do tap).
- Latência da leitura ≈ período de consulta da UI (50 ms) + janela do YIN (~21 ms a 48 kHz).
- Trocar de dispositivo recria a engine: o tap antigo some com ela, o `Studio` desliga o afinador
  (`attach_engine`) e `tuner_read` responde `app.tuner_off` até um novo `tuner_start` (a UI deve
  religar o afinador após `audio_configure`).
- Teste ponta a ponta: `app-core/tests/music.rs::tuner_reads_the_armed_input_through_the_engine_tap`.
