# specs

| Campo | Valor |
|---|---|
| Id | DIR-docs-specs |
| Título | specs |
| Tipo | especificações por fase |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `684fd38`; README na varredura 4 (ver `git log -- docs/specs/README.md`) |
| Arquivos | 9 item(ns) — ver "Onde" |

## Propósito

Especificações de cada fase/área (fundação, captura, DSP/mixagem, harmonia/bases, sincronização/afinação, notação, formatos, UI).

## Origem

Criado em `684fd38 2026-11-05 feat(shared-kernel): IDs, tempo musical, mapa de tempo, ganho/pan, teoria básica e histórico de comandos`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

`AAAA-MM-DD-fase-N-assunto.md`, uma por fase ou área.

## Por que esta organização

Cada fase tem escopo, critérios de aceite e testes previstos antes da implementação (TDD).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Uma spec única | Mistura fases com ritmos e revisões diferentes |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [2026-11-05-fase-0-fundacao.md](2026-11-05-fase-0-fundacao.md) | Fase 0 — Fundação (Especificação) |
| [2026-11-05-fase-2-dsp-mixagem.md](2026-11-05-fase-2-dsp-mixagem.md) | Fase 2 — DSP e Mixagem: crate `dsp` (Especificação) |
| [2026-11-05-fase-5-harmonia-e-4-bases.md](2026-11-05-fase-5-harmonia-e-4-bases.md) | Fase 5 — Harmonia & Composição + Fase 4 — Biblioteca de Bases (Especificação do domínio) |
| [2026-11-05-fase-6-8-sincronizacao-e-afinacao.md](2026-11-05-fase-6-8-sincronizacao-e-afinacao.md) | Fases 6 e 8 — Sincronização e Afinação (Especificação) |
| [2026-11-05-fase-7-notacao.md](2026-11-05-fase-7-notacao.md) | Fase 7 — Notação: partitura e tablatura (Especificação) |
| [2026-11-05-fase-9-formatos.md](2026-11-05-fase-9-formatos.md) | Fase 9 — Formatos de Áudio (Especificação) |
| [2026-11-05-ui-workspace.md](2026-11-05-ui-workspace.md) | Interface — Workspace e Componentes (Spec + Mockups) |
| [2026-11-06-fase-1-captura.md](2026-11-06-fase-1-captura.md) | Fase 1 — Captura (especificação) |
| [2026-11-06-fase-3-edicao.md](2026-11-06-fase-3-edicao.md) | Fase 3 — Edição / Pós-produção (especificação) |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- docs/specs` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
