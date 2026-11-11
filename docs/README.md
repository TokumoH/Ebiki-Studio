# docs

| Campo | Valor |
|---|---|
| Id | DIR-docs |
| Título | docs |
| Tipo | documentação (raiz) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `4be4b31`; README na varredura 4 (ver `git log -- docs/README.md`) |
| Arquivos | 10 item(ns) — ver "Onde" |

## Propósito

Toda a documentação do projeto: requisitos, especificações por fase, arquitetura, decisões (ADRs), manual, processos, relatórios de varredura, registros de commit e diagramas.

## Origem

Criado em `4be4b31 2026-11-05 docs: adiciona levantamento de requisitos e decisões no CLAUDE.md`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Um subdiretório por tipo de documento, cada um com README próprio. Nomes de arquivo com data `AAAA-MM-DD-assunto.md` quando o documento é datado.

## Por que esta organização

Separar por tipo deixa claro onde procurar (decisão → `adr/`, por que um commit → `verificacoes/`) e casa com o fluxo do CLAUDE.md.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Um único documento grande | Difícil de manter e de citar por link |
| Documentação junto do código | Requisitos, ADRs e relatórios cruzam vários crates e a UI |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [adr/](adr/) | Subdiretório (README próprio) |
| [arquitetura/](arquitetura/) | Subdiretório (README próprio) |
| [diagramas/](diagramas/) | Subdiretório (README próprio) |
| [img/](img/) | Subdiretório (README próprio) |
| [manual/](manual/) | Subdiretório (README próprio) |
| [processo/](processo/) | Subdiretório (README próprio) |
| [relatorios/](relatorios/) | Subdiretório (README próprio) |
| [requisitos/](requisitos/) | Subdiretório (README próprio) |
| [specs/](specs/) | Subdiretório (README próprio) |
| [verificacoes/](verificacoes/) | Subdiretório (README próprio) |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
