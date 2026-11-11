# relatorios

| Campo | Valor |
|---|---|
| Id | DIR-docs-relatorios |
| Título | relatorios |
| Tipo | relatórios de varredura |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `a570bdb`; README na varredura 4 (ver `git log -- docs/relatorios/README.md`) |
| Arquivos | 6 item(ns) — ver "Onde" |

## Propósito

Relatórios das varreduras (checklist completo, achados, correções e verificações) e capturas usadas neles.

## Origem

Criado em `a570bdb 2026-11-06 docs: relatório final da varredura (2026-11-06) e captura do smoke test desktop`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

`AAAA-MM-DD-varredura-<trabalho>.md`; imagens em `img/`.

## Por que esta organização

Data no nome ordena cronologicamente; um relatório por varredura.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Misturar com `verificacoes/` | Registros são por commit; relatórios cobrem uma varredura inteira |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [img/](img/) | Subdiretório (README próprio) |
| [2026-11-06-varredura-2.md](2026-11-06-varredura-2.md) | Varredura 2 e simulações do sistema — 2026-11-06 |
| [2026-11-06-varredura-3.md](2026-11-06-varredura-3.md) | Varredura 3 — sistema completo após a v0.2.0 |
| [2026-11-06-varredura-final.md](2026-11-06-varredura-final.md) | Relatório final — varredura e entrega do Ebiki Studio |
| [2026-11-06-varredura-identidade.md](2026-11-06-varredura-identidade.md) | Varredura — Identidade Ebiki (menu fixo, ajuda, tutorial, splash, instaladores) — v0.2.0 |
| [2026-11-11-varredura-4.md](2026-11-11-varredura-4.md) | Varredura 4 — 2026-11-11 (17 objetivos do CORE) |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/relatorios` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
