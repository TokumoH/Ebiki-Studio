# public

| Campo | Valor |
|---|---|
| Id | DIR-ui-public |
| Título | public |
| Tipo | arquivos estáticos da UI |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `310f0ed`; README na varredura 4 (ver `git log -- ui/public/README.md`) |
| Arquivos | 1 item(ns) — ver "Onde" |

## Propósito

Arquivos servidos como estão pelo Vite (ex.: a marca usada na tela de carregamento).

## Origem

Criado em `310f0ed 2026-11-06 feat(ui): menu fixo, central de ajuda, tutorial de início e tela de carregamento`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Arquivos referenciados por caminho absoluto (`/splash-mark.png`).

## Por que esta organização

A tela de carregamento aparece antes do JavaScript; precisa de arquivo estático.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Importar a imagem no JS | Só carregaria depois do bundle, tarde demais para o splash |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [splash-mark.png](splash-mark.png) | Imagem PNG |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- ui/public` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
