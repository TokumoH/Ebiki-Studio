# arquitetura

| Campo | Valor |
|---|---|
| Id | DIR-docs-arquitetura |
| Título | arquitetura |
| Tipo | documentação de arquitetura |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `8376655`; README na varredura 4 (ver `git log -- docs/arquitetura/README.md`) |
| Arquivos | 1 item(ns) — ver "Onde" |

## Propósito

Visão de arquitetura do sistema: contexto C4, camadas (hexagonal + DDD), bounded contexts, engine de áudio, persistência, UI/IPC e roteiro de fases.

## Origem

Criado em `8376655 2026-11-05 docs: adiciona arquitetura geral e ADRs 0001-0012`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Um documento datado por versão da arquitetura; mudanças pontuais viram ADR em `docs/adr/` e atualizam o documento.

## Por que esta organização

A visão geral fica num lugar só; decisões incrementais ficam rastreáveis nos ADRs.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Só ADRs | Falta a visão de conjunto |
| Um arquivo por camada | Fragmenta a leitura de quem chega ao projeto |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [2026-11-05-arquitetura-geral.md](2026-11-05-arquitetura-geral.md) | Ebiki Studio — Arquitetura Geral |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/arquitetura` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
