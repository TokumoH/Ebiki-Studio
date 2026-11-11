# scripts

| Campo | Valor |
|---|---|
| Id | DIR-scripts |
| Título | scripts |
| Tipo | scripts de apoio |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `2a4b629`; README na varredura 4 (ver `git log -- scripts/README.md`) |
| Arquivos | 3 item(ns) — ver "Onde" |

## Propósito

Automação local: verificação igual à CI (`ci-local.sh`), índice de ADRs (`adr_index.py`) e tratamento da marca (`brand_smooth_edges.py`).

## Origem

Criado em `2a4b629 2026-11-05 docs: ADR-0013 (ajustes da Fundação), índice de ADRs gerado por script`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Um script por tarefa, executável da raiz do repositório.

## Por que esta organização

Ferramentas fora do código do produto, versionadas e reproduzíveis.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Scripts no `package.json` | Parte é Rust/Python e roda fora da UI |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [adr_index.py](adr_index.py) | Regenera docs/adr/README.md a partir dos arquivos ADR (título e status). |
| [brand_smooth_edges.py](brand_smooth_edges.py) | Suaviza apenas os contornos: cada conjunto de nível {alfa >= t} é suavizado e re-limiarizado. |
| [ci-local.sh](ci-local.sh) | Executa localmente as mesmas verificações da CI (.github/workflows/ci.yml). |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- scripts` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
