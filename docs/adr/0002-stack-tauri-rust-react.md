# ADR-0002: Stack: Tauri + Rust + React/TypeScript

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
App desktop multiplataforma com áudio em tempo real de baixa latência (< 10 ms) e interface rica (timeline, mixer, partitura).

## Decisão
Usar **Tauri** como shell, **Rust** para domínio, aplicação e engine de áudio, e **React + TypeScript** para a UI.

## Consequências
- Áudio nativo sem garbage collector; segurança de memória.
- UI web facilita componentes visuais ricos.
- Duas linguagens no projeto; contrato IPC precisa ser tipado (ADR-0009).

## Alternativas consideradas
- C++/JUCE: padrão de áudio, mas UI trabalhosa e menor segurança para manter camadas.
- Electron + Web Audio: latência e consumo de memória inadequados.
