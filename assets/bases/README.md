# bases

| Campo | Valor |
|---|---|
| Id | DIR-assets-bases |
| Título | bases |
| Tipo | catálogo de bases |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `48ed4ba`; README na varredura 4 (ver `git log -- assets/bases/README.md`) |
| Arquivos | 1 item(ns) — ver "Onde" |

## Propósito

Manifesto das bases instrumentais (metadados: BPM, tom, gênero, instrumento).

## Origem

Criado em `48ed4ba 2026-11-05 feat: integra Fases 4 e 5 (harmonia + bases) ponta a ponta no backend`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

`manifest.json` lido pela biblioteca de bases.

## Por que esta organização

Metadados declarativos permitem acrescentar bases sem mudar código.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Metadados no código | Exigiria recompilar para cada base nova |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [manifest.json](manifest.json) | Dados JSON |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- assets/bases` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
