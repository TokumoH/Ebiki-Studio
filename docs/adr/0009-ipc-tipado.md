# ADR-0009: IPC tipado com tauri-specta e channels binários

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
UI e núcleo em linguagens diferentes; medidores e posição exigem alta frequência.

## Decisão
Commands (UI→Rust) e Events (Rust→UI) com tipos TypeScript gerados via `tauri-specta`. Dados de alta frequência (posição, medidores, picos de waveform) por **Tauri Channels** em binário. O núcleo Rust é a fonte da verdade; a UI mantém projeções.

## Consequências
- Quebra de contrato detectada em compilação.
- Dependência de uma ferramenta de geração de tipos.

## Alternativas consideradas
- JSON manual sem geração (divergência silenciosa).
