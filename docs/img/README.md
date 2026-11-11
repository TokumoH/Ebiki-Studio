# img

| Campo | Valor |
|---|---|
| Id | DIR-docs-img |
| Título | img |
| Tipo | imagens da documentação |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `598e797`; README na varredura 4 (ver `git log -- docs/img/README.md`) |
| Arquivos | 2 item(ns) — ver "Onde" |

## Propósito

Capturas de tela usadas no README público e no manual.

## Origem

Criado em `598e797 2026-11-06 chore: licença GPL-3.0, README público e medidas de segurança e privacidade`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

PNG com nome do assunto (`workspace.png`, `harmonia.png`).

## Por que esta organização

Imagens fora do código, referenciadas por link relativo.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Imagens na raiz do repositório | Poluem a raiz |
| Hospedagem externa | Links quebram e dependem de terceiros |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [harmonia.png](harmonia.png) | Imagem PNG |
| [workspace.png](workspace.png) | Imagem PNG |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/img` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
