# VER-2026-11-17-04 — M/S do Mixer à vista e clique posiciona o cursor (UX-04, UX-08)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-04 |
| Título | Valor e M/S grudados no pé do canal do Mixer; clicar na faixa ou no clipe move o cursor |
| Gravidade | média |
| Tipo | correção de UX |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/contexts/mixing/{MixerPanel.tsx,mixer.css}`, `ui/src/contexts/project/Timeline.tsx`, `ui/e2e/{overflow,follow}.spec.ts`, `ui/e2e/README.md` |

## Problema

- **UX-04:** em 1280×720 os botões M e S de cada canal do Mixer ficavam abaixo do fim do painel. Cada canal rola sozinho,
  e era preciso rolar o canal para achá-los.
- **UX-08:** só a régua posicionava o cursor. Clicar no vazio da faixa ou num clipe não mexia nele, então "S divide no
  cursor" obrigava a ir até a régua.

## Era proposital?

- **UX-04:** a rolagem por canal é proposital (VER-2026-11-06, canais com muitos efeitos), mas os controles mais usados
  ficavam no fim da rolagem.
- **UX-08:** não. O clique na faixa só selecionava (Fase 3). Reaper e Audacity também posicionam o cursor.

## Solução

- **UX-04:** valor e M/S num rodapé `.strip-foot` com `position: sticky` no fim do canal, sempre visíveis. Efeitos,
  envios e pan rolam acima dele.
- **UX-08:**
  - clique no vazio da faixa (botão esquerdo) leva o cursor para a grade (Alt = livre);
  - clique sem arrastar num clipe também, além de selecioná-lo;
  - arrastar continua movendo o clipe, e o botão direito não mexe no cursor.

## Por que esta solução

- O rodapé fixo mantém o layout e a rolagem do canal sem cortar o fader.
- Posicionar no clique é o padrão dos DAWs e encurta o fluxo de "dividir aqui".

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Fader menor para caber tudo | Perde precisão de volume |
| M/S no topo do canal | Foge da convenção de console (M/S perto do fader) |
| Posicionar o cursor também ao arrastar | Conflita com mover o clipe |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/contexts/mixing/MixerPanel.tsx](../../ui/src/contexts/mixing/MixerPanel.tsx) | rodapé do canal | `.strip-foot` |
| [ui/src/contexts/mixing/mixer.css](../../ui/src/contexts/mixing/mixer.css) | `.strip-foot` | Sticky no pé |
| [ui/src/contexts/project/Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx) | `onClipPointerDown`, `clickTick`, faixa | Clique posiciona o cursor |
| [ui/e2e/overflow.spec.ts](../../ui/e2e/overflow.spec.ts), [follow.spec.ts](../../ui/e2e/follow.spec.ts) | fim | 2 E2E |

## Lógica

- **Antes:** M/S abaixo da área visível. Clique na faixa ou no clipe → só seleção.
- **Depois:** M/S sempre no pé do canal. Clique → seleção + cursor no ponto clicado, arredondado ao tempo.

## Exemplo

Projeto novo: clicar na faixa sobre o compasso 3 leva o cursor a `003.1.000`. Clicar no meio de um clipe que começa no
compasso 3 leva a `003.3.000`.

## Verificação

- **E2E:**
  - Mixer em 1280×720: o centro de "Mute voz" e "Solo voz" no painel Mixer é o próprio botão (nada por cima, dentro da área);
  - clique na faixa e no clipe muda a posição.
- **Sabotagem:**
  - com o rodapé `static`, o E2E do Mixer falhou;
  - sem o seek no clique, o E2E de posição falhou.
- **Verificação 1:** `tsc`; Vitest 49 arquivos / 322 testes; Playwright completo, 70/70.
- **Verificação 2 (outra abordagem):** `vite build`; sabotagens; `check_readmes.py` ok.

## Riscos

- O clique na faixa durante a reprodução também muda a posição (como no Audacity). Quem quer só selecionar a faixa usa o
  cabeçalho.
- Canais muito baixos (menos de 150 px) mostram só o rodapé e o fader; os efeitos rolam acima.
