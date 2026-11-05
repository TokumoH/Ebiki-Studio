# ADR-0004: Backend de áudio: cpal atrás da porta AudioBackend

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Necessário suportar ASIO/WASAPI, CoreAudio e JACK/PipeWire/ALSA com baixa latência. Drivers próprios foram descartados (inviáveis, exigem assinatura).

## Decisão
Usar `cpal` (com feature `asio` no Windows) encapsulado na trait `AudioBackend`. Buffer adaptativo com detecção de xruns e perfis Gravação/Mixagem; compensação de latência de ida e volta.

## Consequências
- Um código para os três SOs.
- Se `cpal` limitar algo, troca-se a implementação sem afetar o resto.

## Alternativas consideradas
- Bindings diretos por SO desde o início (custo alto).
- PortAudio (dependência C, menos idiomático).
