# VER-2026-11-17-02 — Primeiro uso e avisos (UX-09, UX-12, UX-05, UX-17, UX-21)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-02 |
| Título | Gravar sem faixa armada tem saída; metrônomo mostra o estado; indicador de ocupado; avisos acionáveis |
| Gravidade | média (UX-09, UX-05) · baixa (UX-12, UX-17, UX-21) |
| Tipo | correção de UX |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/core/toasts.ts`, `toasts.test.ts` (novo), `ui/src/app/{Toasts.tsx,Workspace.tsx,StartScreen.tsx,app.css,recordPlan.ts,recordPlan.test.ts}`, `ui/src/contexts/recording/RecordingPanel.tsx`, `ui/src/shared/ui.css`, `ui/e2e/first-record.spec.ts` (novo), READMEs de `ui/src/core`, `ui/src/app`, `ui/e2e` |

## Problema

- **UX-09:**
  - R ou ⏺ num projeto novo mostrava só o erro "Arme ao menos uma faixa…", sem caminho para resolver;
  - repetir empilhava avisos iguais;
  - todo primeiro uso caía aqui.
- **UX-12:** o botão do metrônomo no painel Gravação dizia "Ligado" mesmo desligado.
- **UX-05:**
  - importar, abrir, criar e recuperar não mostravam nada além de `aria-busy`, que nenhum CSS usava;
  - os botões continuavam clicáveis.
- **UX-17:** "Criar projeto" ficava desativado sem explicação até escolher a pasta.
- **UX-21:** os erros eram anunciados de forma educada (`role=status`) em vez de como alerta.

## Era proposital?

- A mensagem de erro do núcleo é correta. Só faltava a interface oferecer a saída.
- O rótulo "Ligado" fixo era um engano de texto em um `Toggle`.
- O `busy` já existia em `core/session.ts`, com o indicador visual previsto no comentário da Fase 1, mas sem CSS.

## Solução

- **UX-09:**
  - `recordPlan()` (função pura) decide entre gravar, armar a faixa selecionada (com microfone presente, como
    Ableton/Logic) ou explicar o que falta;
  - nesse último caso, o aviso traz o botão **Abrir Gravação**;
  - aviso igual não empilha: o anterior sai e o tempo recomeça.
- **UX-12:** o texto mostra o estado ("Ligado"/"Desligado"); o nome acessível fixo é "Metrônomo ligado", com `aria-pressed`.
- **UX-05:**
  - barra fina animada sob a barra superior e cursor de progresso (sem animação com `prefers-reduced-motion`);
  - Importar, Exportar e Salvar ficam desativados enquanto o app trabalha;
  - na tela inicial, "Criar projeto" mostra "Abrindo…" e "Abrir projeto…" fica desativado.
- **UX-17:** texto explicando o que falta, ligado ao botão por `aria-describedby`.
- **UX-21:** erros numa região `role=alert`, e informação/sucesso numa `role=status`.

## Por que esta solução

- Armar a faixa selecionada resolve o caso mais comum com um toque.
- Quando falta o microfone, o aviso leva direto ao lugar certo.
- A barra de progresso indeterminada é o indicador padrão para operações de duração desconhecida.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Armar sempre, mesmo sem microfone | Gravaria silêncio e esconderia o problema |
| Diálogo modal "Arme uma faixa" | Interrompe mais do que um aviso com botão |
| Progresso real por arquivo na importação | Exige evento novo do núcleo; fica registrado como melhoria (M) |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/app/recordPlan.ts](../../ui/src/app/recordPlan.ts) | todo | Decisão ao gravar |
| [ui/src/app/Workspace.tsx](../../ui/src/app/Workspace.tsx) | `toggleRecord`, barra superior | Fluxo de gravar; botões desativados e barra de progresso |
| [ui/src/core/toasts.ts](../../ui/src/core/toasts.ts) | todo | Ação e deduplicação |
| [ui/src/app/Toasts.tsx](../../ui/src/app/Toasts.tsx) | todo | Regiões alerta/status; botão de ação |
| [ui/src/app/StartScreen.tsx](../../ui/src/app/StartScreen.tsx) | botão Criar | Ocupado e explicação |
| [ui/src/contexts/recording/RecordingPanel.tsx](../../ui/src/contexts/recording/RecordingPanel.tsx) | metrônomo | Rótulo com estado |
| [ui/src/app/app.css](../../ui/src/app/app.css) | `.busy-bar` | Indicador |

## Lógica

- **Antes:** R num projeto novo → erro → o usuário procura sozinho.
- **Depois:** R →
  - microfone presente e faixa selecionada: a faixa é armada, um aviso informa e a gravação começa;
  - sem microfone ou sem faixa: um aviso com **Abrir Gravação**.

## Exemplo

Projeto novo com "Faixa 1" selecionada e "Entrada simulada" escolhida: R → "“Faixa 1” armada para gravar (Entrada 1)" →
"● Gravando". Sem entrada: R duas vezes → um aviso só → "Abrir Gravação" ativa a aba Gravação.

## Verificação

- **TDD:** `recordPlan.test.ts` (4 testes), `toasts.test.ts` (3 testes: deduplicação com o tempo recomeçando, ação, limite).
- **E2E `first-record.spec.ts`:** sem microfone (um aviso, atalho), com microfone (arma e grava; metrônomo "Desligado"),
  ocupado (barra e Salvar desativado).
- **Sabotagem:** sem o passo de armar, o E2E "arma a faixa selecionada e grava" falha.
- **Ajuste durante a verificação:** a tipagem do import dinâmico no E2E quebrava o `tsc`. Corrigida, as verificações
  recomeçaram.
- **Verificação 1:** `tsc`; Vitest 48 arquivos / 271 testes; Playwright completo, 66/66.
- **Verificação 2 (outra abordagem):** `vite build`; sabotagem acima; `check_readmes.py` ok.

## Riscos

- Armar automaticamente usa a entrada da faixa ou a Entrada 1. Quem usa outro canal escolhe no painel Gravação (o aviso diz qual).
- O manual da Gravação ganha a nota do armar automático junto com a revisão do S1 (mesma seção).
