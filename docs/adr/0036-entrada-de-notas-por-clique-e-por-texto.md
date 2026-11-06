# ADR-0036: Entrada de notas por clique na pauta e por texto corrido

- **Status:** Aceita
- **Data:** 2026-11-06
- **Contexto delimitado:** UI `notation` (o domínio e os comandos de edição não mudam)

## Contexto

O usuário achou a composição em partitura confusa (só teclado e botões de nota) e pediu interação
direta: **clicar para adicionar notas** e **adicionar em lote por texto** (`C,C,C,x,x,x,D,D,D`).
Mockup 8 aprovado.

## Decisão

1. **Texto** (`noteText.ts`, função pura testada): tokens separados por vírgula/espaço/`|`/`;`;
   notas A–G ou solfejo, `x` = pausa, acidentes `# b ♯ ♭ n ♮`, oitava 0–8 (sem oitava: a mais
   próxima da anterior, como no teclado), `:valor[.]` e acordes `C+E+G`. Erro devolve trecho,
   posição e deslocamento; **nada é inserido com erro**; lote limitado a 512 eventos.
2. **Inserção em lote** reaproveita o mesmo `enter()` do teclado (mesmas regras de compasso), com
   política `overflow: "next"` — o que não cabe no fim do compasso vai ao próximo — e **um gesto de
   desfazer** para o lote todo (`score_edit` passou a aceitar `gesture`, como `score_edit_voice`).
3. **Clique na pauta**: o alphaTab informa sistema, compasso, pauta e batida (`boundsLookup`), mas
   não a posição das linhas. As **linhas da pauta são lidas do SVG desenhado** (retângulos finos e
   largos); 5 linhas igualmente espaçadas = pauta, 6 = tablatura (clique recusado com dica). A
   altura sai de `staffPitch(clave, meias-distâncias, armadura, acidente)` — função pura testada.
   Sistema e pauta são escolhidos pelo **mais próximo na vertical** (os limites do alphaTab são justos
   e deixariam de fora linhas suplementares). A nota entra no primeiro tempo livre (pausa) a partir
   da batida apontada.

## Alternativas

| Opção | Por que não |
|---|---|
| Calcular a geometria das linhas pelas constantes internas do alphaTab | Frágil entre versões e escalas; o SVG desenhado é a fonte de verdade |
| Editor de notas próprio (canvas) em vez do alphaTab | Duplicaria a renderização de partitura/tablatura já validada |
| Lote como texto ABC/LilyPond completo | Sintaxe maior que a pedida; a escolhida cobre notas, pausas, oitava, duração e acordes |

## Consequências

- Mudanças no desenho do alphaTab (atualização da biblioteca) podem afetar o clique: o E2E
  `score-entry.spec.ts` cobre a leitura das linhas e a nota resultante.
- Clique na tablatura e edição por arraste de notas ficam para depois.
