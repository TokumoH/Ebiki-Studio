# ADR-0025: SoundFont opcional via `rustysynth`; audição como WAV em memória; arraste vira áudio

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Relacionadas:** ADR-0005 (grafo lock-free), ADR-0009 (IPC tipado), ADR-0011 (portas), ADR-0019

## Contexto
A Fase 5 pede **ouvir** progressões e bases (RF-5.5, RF-4.3) e **arrastá-las para a timeline**
(RF-5.6). O núcleo já tem a porta `MidiRenderer` e o `SimpleSynth` embutido (timbres aproximados). Um
timbre realista exige SoundFont (`.sf2`), mas o app não pode distribuir bancos de terceiros nem quebrar
quando o arquivo some ou está corrompido. A engine de tempo real só toca clipes de áudio (não há faixa
MIDI nesta volta).

## Decisão
1. **`synth::SoundFontSynth::load(path)`** usa o crate `rustysynth` 1.3 (Rust puro, MIT, sem C/FFI).
   O arquivo é validado na carga; erros viram `SynthError` tipado (`synth.io`,
   `synth.invalid_soundfont`, `synth.unsupported_rate`, `synth.too_long`). Toda chamada à biblioteca
   roda sob `catch_unwind`: arquivo malicioso **nunca derruba o app**.
2. **`infra_adapters::ConfigurableSynthRenderer`** implementa `MidiRenderer`: usa o SoundFont quando
   configurado, senão o `SimpleSynth` (fallback permanente). Troca em tempo de execução (`RwLock`); a
   escolha fica no `library.db` (`setting['synth.soundfont']`, vazio = simples) e é reaplicada na
   inicialização (falha silenciosa → simples).
3. **Audição**: renderização offline → **WAV PCM 16 bits estéreo em memória** devolvido pela IPC como
   `tauri::ipc::Response` (bytes → `ArrayBuffer` → `AudioContext.decodeAudioData`). Limite de
   120 s (excedente cortado) e cauda de 1 s. Nada é gravado em disco.
4. **Arrastar para a timeline**: a progressão/base é **renderizada para áudio** na taxa e no mapa de
   tempo do projeto (rebaseado no tick de destino, respeitando mudanças de BPM posteriores), gravada em
   `audio/generated/<id>.wav` (porta `MediaPort::encode`, WAV 24 bits) e registrada como `AudioSource`
   + `Clip` (+ faixa e canal novos, se preciso) num **único** `Composite` desfazível. Conteúdo idêntico
   reutiliza a fonte existente (deduplicação por hash).
5. Testes de SoundFont usam um `.sf2` mínimo **gerado pelo próprio teste** (senoide 440 Hz), sem arquivos
   de terceiros.

## Consequências
- O som melhora sem custo de licença para o projeto (o usuário escolhe seu `.sf2`).
- O clipe inserido é áudio "congelado": editar a harmonia depois exige reinserir (faixa MIDI fica para
  uma volta futura, quando a engine tiver instrumentos em tempo real).
- WAV 16 bits em memória: ~11 MB por minuto estéreo a 48 kHz — aceitável para pré-escuta curta.

## Alternativas consideradas
- `fluidsynth` (C, LGPL): dependência nativa nos 3 SOs e licença mais restritiva para empacotar.
- Tocar a audição pela engine de tempo real: exigiria grafo temporário e conflitaria com o projeto aberto.
- Enviar `Vec<f32>` em JSON: 4–8× maior e lento para serializar.
