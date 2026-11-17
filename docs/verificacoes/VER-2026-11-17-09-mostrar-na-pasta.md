# VER-2026-11-17-09 — "Mostrar na pasta" após exportar

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-09 |
| Título | Aviso de exportação abre o gerenciador de arquivos no arquivo exportado (só arquivos exportados pelo app) |
| Gravidade | baixa (conforto) · segurança: decisão registrada (ADR-0037) |
| Tipo | funcionalidade |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("libere o Mostrar na pasta") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `crates/app-desktop/src/{exported.rs (novo),commands.rs,state.rs,main.rs,ipc_tests.rs}`, `crates/app-desktop/README.md`, `ui/src/ipc/{api.ts,mock.ts}`, `ui/src/app/ExportDialog.tsx`, `ui/e2e/workspace-tools.spec.ts`, `docs/adr/0037-…` (novo), `docs/adr/README.md`, `docs/manual/README.md`, `CHANGELOG.md` |

## Problema

Depois de exportar, o aviso só mostrava o caminho (UX-19, "Copiar caminho"). Na varredura 5, "Mostrar na pasta" ficou
para decisão do usuário porque exige revelar caminhos no sistema, e a ADR-0034 restringe o opener da interface à página
de releases.

## Era proposital?

Sim: a restrição do opener é decisão de segurança (ADR-0034). O usuário decidiu liberar a função. A forma escolhida
mantém a restrição da interface (ADR-0037).

## Solução

- `ExportedFiles` (Pure Fabrication em `app-desktop`):
  - guarda os caminhos canônicos exportados nesta execução, até 256;
  - `export_mix` e `export_stems` registram cada arquivo gravado.
- Comando `export_reveal(path)`:
  - recusa com `app.reveal_denied` o que não foi exportado nesta execução, pastas e arquivos apagados;
  - para o resto, chama `opener().reveal_item_in_dir` no Rust.
- `capabilities/default.json` **não muda**: a interface continua sem `reveal_item_in_dir` e sem `open_path`.
- O aviso de exportação ganha **Mostrar na pasta** (mixagem: o arquivo; stems: o primeiro). Se falhar, aparece um aviso
  de erro.

## Por que esta solução

Entrega o que o usuário pediu sem dar ao WebView o poder de revelar caminhos arbitrários (mínimo privilégio).

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Liberar `opener:allow-reveal-item-in-dir` na interface | Revela qualquer caminho; quebra a ADR-0034 |
| `opener:allow-open-path` | Abre (executa) caminhos; risco maior |
| Manter "Copiar caminho" junto | O aviso tem um botão de ação; o caminho continua escrito na mensagem |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [crates/app-desktop/src/exported.rs](../../crates/app-desktop/src/exported.rs) | todo | Registro e regra |
| [crates/app-desktop/src/commands.rs](../../crates/app-desktop/src/commands.rs) | `export_mix`, `export_stems`, `export_reveal` | Registro e comando |
| [crates/app-desktop/src/state.rs](../../crates/app-desktop/src/state.rs) | `AppState` | Campo `exported` |
| [crates/app-desktop/src/main.rs](../../crates/app-desktop/src/main.rs) | `with_commands` | Comando registrado |
| [crates/app-desktop/src/ipc_tests.rs](../../crates/app-desktop/src/ipc_tests.rs) | após `export_stems` | Recusas |
| [ui/src/app/ExportDialog.tsx](../../ui/src/app/ExportDialog.tsx) | `reveal` | Ação no aviso |
| [docs/adr/0037-mostrar-na-pasta-apenas-arquivos-exportados.md](../adr/0037-mostrar-na-pasta-apenas-arquivos-exportados.md) | todo | Decisão |

## Lógica

- **Antes:** aviso com o caminho e "Copiar caminho".
- **Depois:** exportar registra o arquivo. "Mostrar na pasta" chama `export_reveal`. O Rust confere o registro e o
  arquivo; se estiver tudo certo, abre o gerenciador com o arquivo selecionado, e senão recusa.

## Exemplo

Exportar `/home/u/Musica/Demo.wav` e clicar em "Mostrar na pasta" abre a pasta com `Demo.wav` selecionado. Uma chamada
forjada com `/etc/passwd`, uma pasta ou um WAV não exportado devolve "Só dá para mostrar arquivos exportados nesta
sessão.".

## Verificação

- **TDD:**
  - `exported.rs`: 3 testes (só registrados e existentes; mesmo arquivo via `..`; apagado; pasta recusada; capacidade);
  - IPC: `export_reveal` recusa outro arquivo, a pasta e `/etc/passwd`, com o código `app.reveal_denied`;
  - E2E: o aviso traz "Mostrar na pasta" e chama o comando com o arquivo exportado.
- **Sabotagem:** com a regra liberando tudo, os testes de `exported.rs` e o de IPC falharam.
- **Verificação 1:** `cargo fmt --check` (ajuste de formatação aplicado antes); `cargo test -p app-desktop` 20/20; `tsc`;
  Vitest 52 arquivos / 341 testes; Playwright completo, 78/78.
- **Verificação 2 (outra abordagem):** `cargo clippy -p app-desktop --all-targets -D warnings`; sabotagem; `vite build`;
  `check_readmes.py`.

## Riscos

- O gerenciador de arquivos real (Explorer, Finder, D-Bus no Linux) não foi aberto neste ambiente sem interface gráfica.
  A chamada é a do plugin oficial. Falta testar num desktop.
- A lista vale só durante a execução: depois de reiniciar o app, exportações antigas não podem ser reveladas pelo aviso.
