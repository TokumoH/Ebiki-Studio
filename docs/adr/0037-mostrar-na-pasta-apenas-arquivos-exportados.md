# ADR-0037: "Mostrar na pasta" só para arquivos exportados pelo app

- **Status:** Aceita (decisão do usuário em 2026-11-17: "libere o Mostrar na pasta")
- **Data:** 2026-11-17
- **Contexto delimitado:** `app-desktop` (comandos IPC) e UI (`app`)

## Contexto

Depois de exportar, o aviso mostrava só o caminho do arquivo (UX-19). Abrir a pasta exige o gerenciador de arquivos do
sistema. A ADR-0034 restringiu o `tauri-plugin-opener` da interface a uma única URL (a página de releases). Liberar o
comando `reveal_item_in_dir` do plugin para a interface deixaria o WebView revelar **qualquer** caminho do disco.

## Decisão

- Um comando próprio, `export_reveal(path)`, executado no Rust, revela o arquivo com o opener. **Só** vale para
  arquivos exportados nesta execução (`ExportedFiles`, registrado por `export_mix` e `export_stems`), que ainda existam
  e sejam arquivos (pastas nunca).
- A permissão do plugin na interface (`capabilities/default.json`) **não muda**: continua só a URL de releases.
- O aviso de exportação ganha **Mostrar na pasta** (mixagem: o arquivo; stems: o primeiro arquivo).

## Alternativas

| Opção | Por que não |
|---|---|
| `opener:allow-reveal-item-in-dir` na interface | A interface poderia revelar caminhos arbitrários; quebra o mínimo privilégio da ADR-0034 |
| `opener:allow-open-path` para a pasta | Abre (executa) caminhos; risco maior que revelar |
| Não oferecer | O usuário pediu |

## Consequências

- Revelar caminhos fica restrito ao que o próprio app gravou. A comparação usa o caminho canônico, então `..` e links
  não burlam a regra.
- Reiniciar o app esquece a lista: arquivos exportados antes não podem ser revelados pelo aviso, que também some.
- No Linux, o gerenciador de arquivos é aberto via D-Bus (FileManager1) quando disponível, conforme o plugin.
