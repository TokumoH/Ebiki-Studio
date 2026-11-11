# requisitos

| Campo | Valor |
|---|---|
| Id | DIR-docs-requisitos |
| Título | requisitos |
| Tipo | requisitos |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `4be4b31`; README na varredura 4 (ver `git log -- docs/requisitos/README.md`) |
| Arquivos | 1 item(ns) — ver "Onde" |

## Propósito

Levantamento de requisitos funcionais e não funcionais, decisões de escopo e riscos.

## Origem

Criado em `4be4b31 2026-11-05 docs: adiciona levantamento de requisitos e decisões no CLAUDE.md`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Documento datado; revisões anotadas no próprio documento com link para o ADR que mudou a decisão.

## Por que esta organização

Requisitos aprovados são a base do modo autônomo (CLAUDE.md §3).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Requisitos em issues | Ficam espalhados e sem visão de conjunto |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [2026-11-05-levantamento-requisitos.md](2026-11-05-levantamento-requisitos.md) | Ebiki Studio — Levantamento de Requisitos |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/requisitos` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
