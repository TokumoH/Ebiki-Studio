# e2e

| Campo | Valor |
|---|---|
| Id | DIR-ui-e2e |
| Título | e2e |
| Tipo | testes de ponta a ponta |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README na varredura 4 (ver `git log -- ui/e2e/README.md`) |
| Arquivos | 16 item(ns) — ver "Onde" |

## Propósito

Testes Playwright que exercitam a interface real (modo simulado, sem hardware de áudio): fluxos dos painéis, legendas, camadas, loop, zoom, atualizações, partitura etc.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arranjo, mixer, efeitos, gravação, mapa de tempo, análise, exportação, áudio e atalhos`. README adicionado na varredura de 2026-11-11 (regra do CLAUDE.md §6: todo diretório tem README).

## Organização

Um `*.spec.ts` por área/funcionalidade; cada arquivo cria um projeto novo e não depende dos outros.

## Por que esta organização

Testes independentes e por área são fáceis de rodar isoladamente (`npx playwright test e2e/loop.spec.ts`).

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Um arquivo gigante | Lento para depurar e para rodar só uma área |
| Testes dependentes entre si | Falha em cascata e ordem frágil |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [audio-settings.spec.ts](audio-settings.spec.ts) | 2 teste(s): configurações de áudio: o foco fica no seletor escolhido enquanto a telemetria chega; configurações  |
| [composition.spec.ts](composition.spec.ts) | 5 teste(s): harmonia: tom + gênero → ouvir → inserir → desfazer; harmonia: próximo acorde monta sequência e inse |
| [edit.spec.ts](edit.spec.ts) | Fase 3 — edição: fade pelo inspetor, dividir no cursor (S), menu de contexto e desfazer. |
| [identity.spec.ts](identity.spec.ts) | 7 teste(s): tela de carregamento com a logo aparece e sai sozinha; menu fixo: Exibir troca o tema e Alt leva o t |
| [layers.spec.ts](layers.spec.ts) | Camadas: diálogos e menus ficam acima dos divisores do dockview (antes um divisor cobria parte dos |
| [loop.spec.ts](loop.spec.ts) | 4 teste(s): opções de loop: compassos X até Y, projeto inteiro, desligar e religar o mesmo trecho; sem clipes, ' |
| [music.spec.ts](music.spec.ts) | 5 teste(s): partitura: criar e entrar notas pelo teclado, render alphaTab, validar; letra: colar texto, sincroni |
| [overflow.spec.ts](overflow.spec.ts) | derivado do catálogo: painéis novos entram na verificação automaticamente |
| [recording-input.spec.ts](recording-input.spec.ts) | 2 teste(s): Gravação: escolher a entrada no painel, ver o nível e ouvir a entrada; Gravação: o bloco Entrada de  |
| [score-entry.spec.ts](score-entry.spec.ts) | 3 teste(s): Digitar notas: o exemplo C,C,C,x,x,x,D,D,D entra de uma vez e sai com um Ctrl+Z; Digitar notas: erro |
| [timeline-zoom.spec.ts](timeline-zoom.spec.ts) | 3 teste(s): Ctrl+roda aproxima no tempo mantendo o ponto sob o mouse; Alt+roda muda a altura de todas as faixas; |
| [tooltips-coverage.spec.ts](tooltips-coverage.spec.ts) | Cobertura das legendas: todo controle visível em todos os painéis precisa ter explicação. |
| [tooltips.spec.ts](tooltips.spec.ts) | 2 teste(s): legendas: aba do setor, parâmetro de efeito e botão com título; legendas podem ser desligadas em Exi |
| [updates.spec.ts](updates.spec.ts) | 4 teste(s): Ajuda › Verificar atualizações: salva o projeto antes de instalar e reinicia; sem versão nova e sem  |
| [workspace-tools.spec.ts](workspace-tools.spec.ts) | 5 teste(s): menu Janela lista os painéis e reabre um painel fechado pelo teclado; renomeia o projeto clicando no |
| [workspace.spec.ts](workspace.spec.ts) | 6 teste(s): tela inicial valida o nome e não estoura em 1280×720; fluxo: criar → importar → mixar → desfazer → s |

## Lógica

Os arquivos são citados por links relativos a partir do [CLAUDE.md](../../CLAUDE.md), dos ADRs, do manual e dos registros em `docs/verificacoes/`. Ao adicionar, remover ou renomear um arquivo aqui, atualize a tabela "Onde" no mesmo commit.

## Exemplo

Para encontrar um item: abra a tabela "Onde" acima ou rode `git log --oneline -- ui/e2e` para ver quando e por que cada arquivo mudou.

## Verificação

Gerado e conferido por script na varredura 4: todo arquivo do diretório está em "Onde", nenhum item listado inexiste e os links relativos abrem; subdiretórios têm README próprio.

## Riscos

As descrições da tabela "Onde" vêm do título/cabeçalho de cada arquivo; se um título ficar desatualizado, a descrição também fica.
