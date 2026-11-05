# ADR-0017: Letra sincronizada por sílaba em ticks, LRC/SRT e mapa de warp

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Fase:** 6 (Sincronização)

## Contexto
RF-6.1 pede sincronização de letra por palavra/sílaba; RF-6.2 exportação LRC e SRT; RF-6.3
grade de BPM e sincronização de instrumentos; RF-6.4 time-stretch. O projeto usa tempo musical
(ticks, PPQ 960) com mapa de tempo variável (`TempoMap`).

## Decisão
1. Contexto **Sync** (`ctx-sync`), puro, com agregados `LyricTrack` e `WarpMap`.
2. A letra guarda tempos **somente nas sílabas**, em **ticks** (`Span { start, end }`), opcionais.
   Texto de palavra/linha é **derivado** (sem redundância — mesma regra de normalização do banco).
   Palavra/linha recebem intervalo dividido entre sílabas por número de caracteres.
3. Invariante: na ordem de leitura, sílabas com tempo são monotônicas e sem sobreposição.
   Toda operação é atômica (valida a trilha inteira e reverte em caso de erro) e devolve os
   valores anteriores para desfazer (Command Pattern, ADR-0008).
4. Separação silábica do português **por regras** (sem dicionário), com limites documentados.
5. LRC/SRT são **processamento de texto puro** no domínio (sem I/O); segundos ↔ ticks pelo
   `TempoMap`. LRC "enhanced" por palavra ou sílaba (`<mm:ss.xx>`), SRT por linha.
6. Sessão **tap-to-sync** é uma máquina de estados pura (recebe ticks) que produz atribuições;
   um comando grava o resultado de forma desfazível.
7. `WarpMap`: marcadores estritamente crescentes (quadro da fonte ↔ tick relativo ao clipe),
   linear entre marcadores e andamento nativo fora deles; fornece a razão local de stretch
   para o `dsp-pitch`.

## Consequências
- Mudar o BPM do projeto mantém a letra presa à música (ticks), e a exportação reflete o novo
  tempo automaticamente.
- Precisão de ida e volta: 10 ms (LRC) e 1 ms (SRT); ida e volta exata em texto para
  BPM ≥ ~63 (tick ≤ 1 ms) — abaixo disso, ±1 tick.
- Pausas entre sílabas não são representáveis em LRC (formato), apenas no projeto.

## Alternativas consideradas
- Tempos em segundos/amostras: quebraria a letra ao alterar o BPM.
- Tempos em palavra e sílaba ao mesmo tempo: redundância e risco de inconsistência.
- Dicionário de hifenização (TeX/hyphen-pt): mais preciso, porém dependência e licença;
  pode entrar como outra implementação no futuro (a função é isolada).
