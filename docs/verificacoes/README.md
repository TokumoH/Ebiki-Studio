# verificacoes

| Campo | Valor |
|---|---|
| Id | DIR-docs-verificacoes |
| Título | verificacoes |
| Tipo | documentação de processo (registros de commit) |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | commit que adotou o CORE de 2026-11-10 (ver `git log -- docs/verificacoes`) |
| Arquivos | `README.md`, `_modelo-registro-de-correcao.md`, `VER-*.md` |

## Propósito

Guarda um **registro por commit**: o que mudou, por quê, alternativas descartadas, onde, como foi
verificado (duas verificações seguidas sem erro) e riscos. É a trilha de auditoria das decisões do
projeto, complementar aos ADRs (decisões de arquitetura) e aos relatórios de varredura.

## Origem

Criado ao incorporar o *Prompt CORE* do usuário ao [`CLAUDE.md`](../../CLAUDE.md) (§6), em
2026-11-10: "todo commit leva documento em `docs/verificacoes/`".

## Organização

- `_modelo-registro-de-correcao.md`: modelo obrigatório (o `_` o mantém no topo da listagem).
- `VER-AAAA-MM-DD-NN-assunto.md`: um registro por commit; `NN` é a sequência do dia.
- Sem subdiretórios.

## Por que esta organização

Nome com data e sequência ordena cronologicamente e é único; o assunto no nome facilita a busca.
Um arquivo por commit casa 1:1 com o histórico do git.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Registrar só na mensagem do commit | Não comporta o formato completo (tabelas, verificação, riscos) nem links |
| Um arquivo por mês | Conflitos de edição e registros difíceis de citar individualmente |
| Misturar com `docs/relatorios/` | Relatórios são de varredura (vários commits); registros são por commit |

## Onde

| Arquivo | O que contém |
|---|---|
| [_modelo-registro-de-correcao.md](_modelo-registro-de-correcao.md) | Modelo do registro |
| [VER-2026-11-10-01-core-no-claude-md.md](VER-2026-11-10-01-core-no-claude-md.md) | Adoção do CORE no CLAUDE.md e criação deste diretório |
| [VER-2026-11-10-02-legendas.md](VER-2026-11-10-02-legendas.md) | Legendas ao passar o mouse (glossário + camada única) |
| [VER-2026-11-10-03-zoom-pela-roda.md](VER-2026-11-10-03-zoom-pela-roda.md) | Zoom pela roda, altura das faixas e camadas acima do dockview |
| [VER-2026-11-10-04-loop-configuravel.md](VER-2026-11-10-04-loop-configuravel.md) | Loop configurável (opções, projeto inteiro, régua) |
| [VER-2026-11-10-05-menu-da-logo-e-entrada-recolhivel.md](VER-2026-11-10-05-menu-da-logo-e-entrada-recolhivel.md) | Menu da logo e entrada de áudio recolhível |

## Lógica

1. Antes do commit: dupla verificação (testes, simulações, sabotagens).
2. Copiar o modelo para `VER-…-NN-assunto.md` e preencher.
3. Commitar o registro **junto** com a mudança; atualizar a tabela "Onde" deste README no mesmo commit.
4. ADRs citados no registro explicam decisões de arquitetura; o registro explica o commit.

## Exemplo

Para saber por que o `CLAUDE.md` mudou em 2026-11-10: abrir
[VER-2026-11-10-01](VER-2026-11-10-01-core-no-claude-md.md) ou rodar
`git log -- docs/verificacoes/VER-2026-11-10-01-core-no-claude-md.md`.

## Verificação

Links relativos conferidos por script (todos existem); todo arquivo do diretório está em "Onde";
nenhum item listado inexistente; sem subdiretórios.

## Riscos

Commits anteriores a 2026-11-10 não têm registro aqui (estão nos relatórios de varredura e ADRs).
