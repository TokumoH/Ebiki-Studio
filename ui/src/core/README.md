# core

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-core |
| Título | core |
| Tipo | estado compartilhado e regras de UI |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/core/README.md`) |
| Arquivos | 13 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Stores (zustand) e funções puras usadas por várias áreas: sessão, telemetria, tema, áudio, loop, compassos, glossário das legendas, avisos.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um arquivo por assunto; funções puras com testes ao lado.

## Por que esta organização

Camadas com direção única de dependência (app → panels → contexts → core/ipc/shared) mantêm componentes independentes; o teste `architecture.test.ts` impede violações.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Organizar por tipo de arquivo (components/, hooks/…) | Espalharia cada contexto por várias pastas e esconderia as dependências |
| Tudo em `src/` | Sem fronteiras, o acoplamento cresce sem ser notado |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [audio.ts](audio.ts) | Estado do dispositivo de áudio compartilhado pela interface (barra de status, Configurações de |
| [audition.ts](audition.ts) | Audição: toca um WAV (ArrayBuffer vindo do núcleo) pela WebAudio, um de cada vez. |
| [bars.test.ts](bars.test.ts) | 4/4 nos compassos 1–2 e 3/4 a partir do compasso 3 (tick 7680) |
| [bars.ts](bars.ts) | Aritmética de compassos sobre o mapa de tempo (pura, sem dependências de apresentação). |
| [composition.ts](composition.ts) | Estado de UI compartilhado entre os contextos de composição (Harmonia → Bases), sem que um |
| [format.test.ts](format.test.ts) | Testes: formata valores para exibição |
| [format.ts](format.ts) | Formatação de valores musicais e de áudio para exibição. |
| [glossary.ts](glossary.ts) | Glossário das legendas ao passar o mouse (mockup 9): um texto curto por setor, efeito, parâmetro e |
| [live.ts](live.ts) | Telemetria do núcleo (posição e medidores, ~30 Hz). |
| [loop.ts](loop.ts) | Loop (compartilhado pelo transporte e pelo arranjo — por isso fica em core): liga/desliga lembrando |
| [session.ts](session.ts) | Estado de aplicação: projeção da sessão vinda do núcleo (fonte da verdade continua no Rust). |
| [theme.ts](theme.ts) | Tema claro/escuro: estado, preferência guardada e aplicação no documento. |
| [toasts.ts](toasts.ts) | Estado dos avisos temporários (notify.success / notify.error). |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/core`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
