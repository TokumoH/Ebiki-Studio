# processo

| Campo | Valor |
|---|---|
| Id | DIR-docs-processo |
| Título | processo |
| Tipo | documentação de processo |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `da94f1b`; README na varredura 4 (ver `git log -- docs/processo/README.md`) |
| Arquivos | 3 item(ns) — ver "Onde" |

## Propósito

Como o trabalho é feito: convenções de código e commits, modelo em espiral das fases e o procedimento de varredura.

## Origem

Criado em `da94f1b 2026-11-05 docs: registra modo autônomo e procedimento de varredura`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Um arquivo por processo, citados pelo CLAUDE.md.

## Por que esta organização

Processos estáveis ficam versionados e linkáveis a partir do CLAUDE.md.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Tudo dentro do CLAUDE.md | O CLAUDE.md fica longo demais para ser lido em toda sessão |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [convencoes.md](convencoes.md) | Convenções de Código (obrigatórias) |
| [espiral.md](espiral.md) | Processo em Espiral com Arquitetura de Testes |
| [varredura.md](varredura.md) | Procedimento de Varredura (obrigatório ao fim de cada grande trabalho) |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/processo` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
