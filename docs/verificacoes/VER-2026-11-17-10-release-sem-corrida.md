# VER-2026-11-17-10 — Release sem corrida entre os sistemas (v0.4.0 saiu duplicada e incompleta)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-10 |
| Título | Workflow de release: rascunho único, instaladores só enviam arquivos, publicação no fim com latest.json completo |
| Gravidade | alta (atualizador sem Windows/macOS na v0.4.0) |
| Tipo | correção de CI/release |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("publique a v0.4.0") |
| Status | implementado; v0.4.0 refeita com `replace_existing` (decisão do usuário em 2026-11-17: "Refazer a v0.4.0") |
| Commits | este commit |
| Arquivos | `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `scripts/release_latest_json.py` (novo), `scripts/test_release_latest_json.py` (novo), `scripts/ci-local.sh`, `scripts/README.md` |

## Problema

Run 6 do `release.yml` (v0.4.0):
- Windows e Linux terminaram no mesmo segundo e **cada um criou uma release `v0.4.0`** (ids 447867896 e 447867860).
- Os arquivos do Windows foram para uma release e os do Linux para a outra.
- O macOS compilou e assinou, mas falhou ao atualizar a release (`already_exists tag_name`).
- O `SHA256SUMS.txt` não foi gerado.
- O `latest.json` servido aos apps tem **só Linux**: quem usa Windows ou macOS não recebe a atualização e pode ver erro
  ao verificar atualizações.

## Era proposital?

Não. Cada job da matriz chamava a `tauri-action` com `tagName`: ela cria a release se não existir e mescla o
`latest.json`. Com os jobs em paralelo, a criação e a mescla disputam entre si. Nas versões 0.2.x e 0.3.0 os jobs
terminaram em momentos diferentes (sorte de tempo).

## Solução

1. **preparar** (job único):
   - valida a versão e as notas do CHANGELOG;
   - cria a release como **rascunho** e passa o id adiante;
   - se já existir release com a tag, **falha**, a menos que o disparo manual peça `replace_existing`. Nesse caso apaga
     as releases da tag e a tag, e recria.
2. **instaladores** (3 sistemas): `tauri-action` com `releaseId` do rascunho e `includeUpdaterJson: false`. Só enviam
   arquivos.
3. **publicar** (depois dos 3):
   - baixa os arquivos do rascunho pela API, por id;
   - monta o `latest.json` com `scripts/release_latest_json.py`, que **falha se faltar qualquer plataforma**;
   - gera o `SHA256SUMS.txt`, envia os dois e publica (`draft=false`, `make_latest`).

   Se algum sistema falhar, nada é publicado.
4. O teste do gerador roda no `ci-local.sh docs` e no job de auditoria da CI.

## Por que esta solução

- Uma única origem para a release e para o `latest.json` elimina a corrida.
- O rascunho só vira público com tudo pronto, então quem atualiza nunca vê release pela metade.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Rodar os 3 sistemas em sequência (`max-parallel: 1`) | Triplica o tempo da release e ainda depende da mescla do latest.json |
| Re-executar só o job do macOS | Falharia de novo (duas releases com a mesma tag) |
| Deixar a `tauri-action` gerar o latest.json com `releaseId` | Ainda mescla em paralelo; a corrida do latest.json continuaria |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [.github/workflows/release.yml](../../.github/workflows/release.yml) | todo | `prepare` → `bundle` → `publish`; entrada `replace_existing` |
| [scripts/release_latest_json.py](../../scripts/release_latest_json.py) | todo | Gerador do latest.json |
| [scripts/test_release_latest_json.py](../../scripts/test_release_latest_json.py) | todo | Testes |
| [scripts/ci-local.sh](../../scripts/ci-local.sh), [.github/workflows/ci.yml](../../.github/workflows/ci.yml) | modo `docs`, job de auditoria | Rodam os testes |

## Lógica

- **Antes:** 3 jobs, cada um com "cria ou atualiza a release + mescla o latest.json", em paralelo.
- **Depois:** 1 rascunho → 3 uploads → 1 latest.json + SHA256SUMS → publicar.

## Exemplo

Com os arquivos da v0.3.0 e os `.sig` reais baixados, o gerador produz um `latest.json` com as mesmas 13 plataformas e
**URLs e assinaturas idênticas** ao publicado em 2026-11-11. Com só os arquivos do Linux (como a v0.4.0 quebrada), ele
falha listando Windows e macOS.

## Verificação

- **Verificação 1:**
  - `test_release_latest_json.py`, 3 testes (reproduz a v0.3.0; recusa release só com Linux; recusa instalador sem
    assinatura);
  - `bash -n` nos 10 blocos `run` do workflow;
  - YAML de `release.yml` e `ci.yml` válidos.
- **Verificação 2 (outra abordagem):**
  - simulação do job "publicar" com os arquivos reais da v0.3.0: resultado igual ao `latest.json` publicado;
  - durante a simulação, um `.sig` falhou no download e o gerador recusou com "faltam: windows-x86_64-msi", a trava
    agindo num caso real;
  - `scripts/ci-local.sh docs` sem erros;
  - `check_readmes.py` ok.

## Primeira execução (run 7) e ajuste

- `prepare` funcionou: apagou as duas releases e a tag e criou o rascunho.
- Os 3 sistemas enviaram os arquivos.
- O gerador montou o `latest.json` com as 13 plataformas, mas o envio falhou com `already_exists`. Na `tauri-action` v1 a
  opção se chama `uploadUpdaterJson`, e não `includeUpdaterJson`, então a action enviou o próprio `latest.json`.
- Nada foi publicado (a trava funcionou: o rascunho ficou parado).
- **Ajuste:** opção com o nome certo (conferido no `action.yml` da v1). O job final apaga `latest.json` e
  `SHA256SUMS.txt` que já estejam no rascunho antes de enviar os seus.
- **Reverificação:** YAML, `bash -n` nos blocos e testes do gerador.

## Riscos

- O workflow novo só é provado de verdade na próxima execução: não dá para rodar Actions localmente. Os comandos `gh api`
  seguem a API REST documentada (criar rascunho, baixar asset por id, enviar para `uploads.github.com`, PATCH para
  publicar).
- `replace_existing` apaga releases e a tag. Só é usado com pedido explícito, no disparo manual.
- A v0.4.0 refeita inclui o "Mostrar na pasta" (VER-2026-11-17-09), que entrou nas notas da 0.4.0 no lugar de "Não lançado".
