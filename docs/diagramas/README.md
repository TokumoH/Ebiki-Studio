# diagramas

| Campo | Valor |
|---|---|
| Id | DIR-docs-diagramas |
| Título | diagramas |
| Tipo | diagramas (HTML autocontido) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `d1488f5`; README na varredura 4 (ver `git log -- docs/diagramas/README.md`) |
| Arquivos | 2 item(ns) — ver "Onde" |

## Propósito

Páginas visuais: arquitetura em camadas/componentes e as paletas de cor da marca.

## Origem

Criado em `d1488f5 2026-11-05 docs: adiciona página visual da arquitetura`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Arquivos `.html` autocontidos (sem dependências externas), abertos direto no navegador.

## Por que esta organização

HTML autocontido renderiza em qualquer máquina e no GitHub Pages sem ferramenta extra.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Imagens PNG | Não são editáveis nem pesquisáveis |
| Ferramenta de diagramas proprietária | Exige conta/licença para editar |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [arquitetura.html](arquitetura.html) | Ebiki Studio Arquitetura |
| [paletas-ebiki.html](paletas-ebiki.html) | Paletas do Ebiki Studio |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/diagramas` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
