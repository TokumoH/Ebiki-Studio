# Convenções de Código (obrigatórias)

## Rust

- Um crate por bounded context (`crates/ctx-<nome>`) com módulos `domain` (puro, sem I/O) e `commands`
  (operações desfazíveis implementando `shared_kernel::command::Command<Agregado>`).
- Serviços técnicos (Pure Fabrication) em crates próprios: `dsp`, `audio-engine`, `infra-*`, `synth`.
- Crates de contexto **não dependem** de outros contextos — só de `shared-kernel`. Integração entre
  contextos acontece na camada de aplicação (`app-core`).
- Erros por contexto: `enum XxxError` com `thiserror`, método `code(&self) -> &'static str` no formato
  `"<contexto>.<snake_case>"` e `impl From<XxxError> for CommandError`. Mensagens em PT-BR.
- Identificadores com `shared_kernel::define_id!`. Serialização `serde` com `rename_all = "camelCase"`.
- Todo valor numérico vindo de fora é validado ou limitado (`clamped`), inclusive `NaN`/infinito.
- Código de tempo real (`process` de DSP): **sem alocação, sem lock, sem panic, sem I/O**. Buffers
  pré-alocados em `prepare(sample_rate, max_block)`.
- Dependências novas: declarar no `Cargo.toml` do próprio crate (não no workspace raiz), versão estável.
- Antes de commitar: `cargo fmt --all`, `cargo clippy -p <crate> --all-targets -- -D warnings`,
  `cargo test -p <crate>`.

## Testes (TDD — ver `espiral.md`)

- Teste antes do código; cada função pública tem teste que verifica o comportamento, não só que "roda".
- Invariantes matemáticas com `proptest` (ida e volta, monotonicidade, limites).
- Comandos: teste de ida e volta (aplicar → desfazer = estado anterior; refazer = mesmo estado).
- DSP: testes com sinais sintéticos (seno, impulso, ruído determinístico) e asserções numéricas.

## TypeScript / React

- `ui/src/shared` (design system, sem regra de negócio), `ui/src/contexts/<contexto>` (store + componentes
  do contexto), `ui/src/panels` (composição), `ui/src/ipc` (único ponto que fala com o núcleo).
- Um contexto da UI nunca importa outro; composição só em `panels/`.
- Textos de interface em PT-BR; identificadores em inglês.
- Testes com Vitest + Testing Library; E2E com Playwright usando o transporte IPC simulado.

## Documentação

- Cada fase/volta da espiral tem spec em `docs/specs/AAAA-MM-DD-fase-N-<nome>.md` com objetivos,
  **tabela de riscos (risco, mitigação, teste que comprova)**, modelo, diagramas Mermaid e critérios de aceite.
- Decisões relevantes viram ADR em `docs/adr/` (próximo número livre) e entram no índice `docs/adr/README.md`.
