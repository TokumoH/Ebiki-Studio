# VER-2026-11-11-02 — README por módulo em todo o repositório

| Campo | Valor |
|---|---|
| Id | VER-2026-11-11-02 |
| Título | README por módulo (crates e camadas da UI) + verificador automático |
| Gravidade | nenhuma (documentação/processo) |
| Tipo | documentação |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH ("siga com sua sugestão de README por módulo") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `CLAUDE.md`, `scripts/check_readmes.py` (novo), `scripts/ci-local.sh`, `.github/workflows/ci.yml`, 29 READMEs novos (19 crates, `crates/`, `ui/`, `ui/src/`, 7 camadas da UI), `assets/brand/README.md`, `scripts/README.md`, 9 arquivos TS com linha de cabeçalho |

## Problema

A regra "todo diretório tem README" deixava ~80 pastas de código sem README; um README completo por
pasta geraria dezenas de textos quase iguais. Varredura 4 propôs README por módulo; o usuário aprovou.

## Era proposital?

Não havia READMEs de código porque a regra só foi adotada em 2026-11-10 (VER-2026-11-10-01).

## Solução

- Regra no CLAUDE.md §6: documentação/apoio com README por diretório; **código com README por módulo**
  (crate ou camada da UI), cobrindo as subpastas na tabela "Onde"; ocultos e build fora.
- 29 READMEs gerados com a estrutura completa; "Onde" lista **todo arquivo versionado** do módulo e das
  subpastas, com a descrição do comentário de cabeçalho de cada arquivo (`//!`, `//`, `--`).
- `scripts/check_readmes.py`: confere cobertura de diretórios, listagem de arquivos e links; roda no
  `ci-local.sh` (`docs`) e no job de auditoria da CI.
- 9 arquivos TS sem cabeçalho ganharam uma linha de descrição (útil no código e no README).

## Por que esta solução

Cumpre o objetivo da regra (achar onde cada coisa está) com um README por unidade de arquitetura e um
verificador que impede a documentação de envelhecer sem ninguém notar.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| README completo em cada uma das ~80 pastas | Texto repetitivo e manutenção cara (decisão do usuário) |
| Só README nos crates, sem camadas da UI | A UI tem regras de camada próprias que precisam de explicação |
| Gerar READMEs na CI | Esconderia mudanças da revisão; o verificador apenas acusa |
| Contar arquivos não versionados | Artefatos de build (`tsconfig.tsbuildinfo`) apareciam como "não listados" |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [CLAUDE.md](../../CLAUDE.md) | §6, §7 | Regra de README por módulo e verificador |
| [scripts/check_readmes.py](../../scripts/check_readmes.py) | todo | Verificador (só arquivos versionados) |
| [scripts/ci-local.sh](../../scripts/ci-local.sh) | etapa `docs` | Roda o verificador |
| [.github/workflows/ci.yml](../../.github/workflows/ci.yml) | job `audit` | Passo "READMEs" |
| [crates/README.md](../../crates/README.md), [ui/README.md](../../ui/README.md), [ui/src/README.md](../../ui/src/README.md) | todo | Índices |
| `crates/*/README.md`, `ui/src/*/README.md` | todo | READMEs dos módulos |

## Lógica

Antes: 48 − 29 = 19 READMEs; ~80 pastas sem cobertura. Depois: cada pasta tem README ou aparece em
"Onde" do módulo; o verificador falha se surgir arquivo/pasta não listado ou link quebrado.

## Exemplo

`crates/audio-engine/README.md` lista `src/core.rs` — "Núcleo de tempo real. Regras: sem alocação, sem
bloqueio, sem liberar memória, sem pânico." — e as demais 15 entradas, tiradas dos `//!` dos arquivos.

## Verificação

- **Verificação 1:** `check_readmes.py` → "READMEs ok (48 arquivos)"; `tsc`; Vitest 254/254 (cabeçalhos não
  mudam comportamento).
- **Verificação 2 (outra abordagem):** script independente via `git ls-files` confirma 0 arquivos
  versionados fora dos READMEs; build de produção; E2E de identidade, workspace e cobertura de legendas 14/14.
- **Sabotagem:** arquivo novo em `crates/dsp/src` e pasta nova em `ui/src/core` → o verificador acusa os 3
  itens; removidos → volta a "ok".
- Ajuste encontrado na verificação: o verificador contava artefatos ignorados pelo git → passou a usar
  só arquivos versionados.

## Riscos

- Os geradores usados para criar os READMEs não foram versionados (uso único); daqui em diante os
  READMEs são mantidos à mão, com o verificador como rede de segurança.
- Commits só de documentação não disparam a CI (paths-ignore); o verificador roda quando há mudança de
  código e no `ci-local.sh`.
