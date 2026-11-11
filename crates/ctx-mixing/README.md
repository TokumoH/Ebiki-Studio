# ctx-mixing

| Campo | Valor |
|---|---|
| Id | DIR-crates-ctx-mixing |
| Título | ctx-mixing |
| Tipo | bounded context de domínio (DDD) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `8259681`; README por módulo em 2026-11-11 (ver `git log -- crates/ctx-mixing/README.md`) |
| Arquivos | 8 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Bounded context Mixing: canais, buses, sends, efeitos, presets e auxílio de mixagem por regras.

## Origem

Criado em `8259681 2026-11-05 feat: contextos Project e Mixing (domínio, comandos, efeitos, presets, advisor)`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

`domain` (modelo e invariantes) e `commands` (comandos desfazíveis); depende só do `shared-kernel`.

## Por que esta organização

Separar modelo e comandos deixa as invariantes testáveis e cada mudança desfazível (Command Pattern, ADR-0008).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Domínio junto da infraestrutura | Quebraria a pureza (GRASP Pure Fabrication) e os testes sem I/O |
| Um crate de domínio único | Contextos acoplados; o CLAUDE.md exige bounded contexts independentes |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/advisor.rs](src/advisor.rs) | Auxílio de mixagem por regras. A porta ['MixAdvisor'] permite trocar por IA no futuro. |
| [src/commands.rs](src/commands.rs) | Comandos desfazíveis sobre o ['Mixer']. |
| [src/domain.rs](src/domain.rs) | Mixer: canais por faixa, buses, master, sends e roteamento. |
| [src/effects.rs](src/effects.rs) | Efeitos nativos e seus parâmetros. Todo parâmetro é limitado ('clamped') a uma faixa segura. |
| [src/lib.rs](src/lib.rs) | Bounded context Mixing. |
| [src/presets.rs](src/presets.rs) | Presets de cadeia de efeitos por instrumento (foco em voz, violão e guitarra). |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/ctx-mixing`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
