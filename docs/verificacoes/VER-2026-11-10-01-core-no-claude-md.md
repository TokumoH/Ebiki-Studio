# VER-2026-11-10-01 — CORE do usuário incorporado ao CLAUDE.md

| Campo | Valor |
|---|---|
| Id | VER-2026-11-10-01 |
| Título | CORE do usuário incorporado ao CLAUDE.md, mantendo a autonomia |
| Gravidade | nenhuma (processo/documentação) |
| Tipo | processo |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH |
| Status | implementado |
| Commits | este commit |
| Arquivos | `CLAUDE.md`, `docs/verificacoes/README.md`, `docs/verificacoes/_modelo-registro-de-correcao.md`, este registro |

## Problema

O usuário enviou um *Prompt CORE* com regras novas (dupla verificação, formato de sugestão,
registro por commit em `docs/verificacoes/`, README em todo diretório, objetivos de varredura) e
pediu para atualizar o `CLAUDE.md`, **mantendo a autonomia de commits e pushes**.

## Era proposital?

O `CLAUDE.md` anterior refletia o CORE de 2026-11-05 + modo autônomo. Não havia registro por
commit nem regra de README por diretório: não era proposital, só anterior ao pedido.

## Solução

Reescrever o `CLAUDE.md` integrando o CORE novo (§1–§7) às decisões do projeto (§8), com uma tabela
(§3) que explica como cada "nunca sem autorização" é cumprido no modo autônomo. Criar o diretório
`docs/verificacoes/` com README (estrutura exigida) e o modelo de registro.

## Por que esta solução

As duas instruções do usuário se tensionam ("esperar aprovação" × "manter autonomia"); a tabela
torna explícito o critério: pedidos novos esperam aprovação, o que já foi aprovado segue autônomo,
sempre com registro e dupla verificação.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Colar o CORE no fim do arquivo antigo | Duplicaria regras e deixaria contradições (autonomia × esperar) |
| Remover o modo autônomo | Contraria o pedido explícito do usuário |
| Criar README em todos os ~100 diretórios agora | Fora do escopo deste commit; a regra vale para diretórios criados e os antigos ficam como pendência da varredura (registrado no CLAUDE.md) |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [CLAUDE.md](../../CLAUDE.md) | todo | Reescrito: papel, arquitetura+DevSecOps, autonomia (tabela), dupla verificação, formato, registro, README, varredura (17 objetivos), decisões, fluxo |
| [docs/verificacoes/README.md](README.md) | todo | Novo |
| [docs/verificacoes/_modelo-registro-de-correcao.md](_modelo-registro-de-correcao.md) | todo | Novo modelo |

## Lógica

Antes: CORE de 2026-11-05 (sem registro por commit). Depois: todo commit → registro neste diretório;
todo diretório novo → README; dupla verificação explícita com regra de nits.

## Exemplo

O próximo commit (legendas e navegação) já sai com `VER-2026-11-10-02`.

## Verificação

- **Verificação 1:** releitura do CORE item a item contra o novo `CLAUDE.md` (todas as regras
  presentes: papel, arquitetura, DevSecOps, autorização, dupla verificação, nits, formato, registro,
  README, 17 objetivos de varredura, fluxo, "por quê").
- **Verificação 2 (outra abordagem):** script que confere links relativos dos três documentos
  (todos existem) e lista de diretórios sem README (100 antigos — registrados como pendência).

## Riscos

Os ~100 diretórios antigos sem README continuam pendentes até a próxima varredura.
