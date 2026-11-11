# img

| Campo | Valor |
|---|---|
| Id | DIR-docs-relatorios-img |
| Título | img |
| Tipo | imagens de relatórios |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `a570bdb`; README na varredura 4 (ver `git log -- docs/relatorios/img/README.md`) |
| Arquivos | 1 item(ns) — ver "Onde" |

## Propósito

Capturas citadas nos relatórios de varredura (ex.: smoke test do app desktop).

## Origem

Criado em `a570bdb 2026-11-06 docs: relatório final da varredura (2026-11-06) e captura do smoke test desktop`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

PNG com data e assunto.

## Por que esta organização

Mantém as imagens perto dos relatórios que as citam.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Usar `docs/img` | Lá ficam imagens da documentação pública, não evidências de varredura |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [2026-11-06-smoke-desktop.png](2026-11-06-smoke-desktop.png) | Imagem PNG |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/relatorios/img` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
