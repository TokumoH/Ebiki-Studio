# ADR-0018: Graus romanos relativos ao modo, qualidade diatônica derivada e sintaxe de baixo/secundária

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Relacionadas:** ADR-0007 (progressões em graus relativos), ADR-0011 (portas de extensão)

## Contexto
O ADR-0007 decidiu guardar progressões em graus relativos. Faltava definir **relativos a quê**, como
representar qualidade (para não duplicar o que o campo harmônico já determina), inversões (`G/B`) e
dominantes secundárias (`V/V`), e como isso vira texto (RF-5.7: cifra ↔ graus romanos).

Há duas convenções de mercado para acidentes em graus romanos: relativos à **escala maior** (Lá menor:
`i bIII bVI bVII`) ou relativos à **escala do modo** (Lá menor: `i III VI VII`).

## Decisão
1. **Graus relativos à escala do modo da progressão.** `VII` em Lá menor = Sol; `bVII` em Dó maior = Si♭.
   A progressão carrega o seu modo; a tônica é aplicada na leitura (`Progression::chords_in`).
2. **Caixa = terça; sufixo = mesma gramática da cifra.** Maiúsculo começa como tríade maior, minúsculo como
   menor; sufixos `7`, `maj7`, `°`, `ø7`, `+`, `sus4`, `6`, `5`, `(9)`… reutilizam o *parser* de cifra.
3. **Qualidade normalizada (`QualitySpec`).** Ao ler, se a qualidade é igual à tríade/tétrade diatônica do
   grau, ela é gravada como `Diatonic`/`DiatonicSeventh` (no banco: `quality_id NULL` + `diatonic_seventh`).
   Só empréstimos, dominantes secundárias e cores são gravados explicitamente.
4. **Barra:** `/<numeral romano>` = dominante secundária do grau alvo (`V7/ii`, resolvida sobre a escala
   maior do alvo); `/<número arábico>` = baixo no grau da tonalidade (`V/7` = Sol/Si em Dó). Os dois são
   mutuamente exclusivos num mesmo acorde.
5. **Grafia:** graus abaixados são escritos com bemóis, elevados com sustenidos; naturais seguem a armadura
   (relativa maior) da tonalidade.

## Consequências
- Sem redundância: a qualidade só é armazenada quando não é derivável (3FN no `progression_step`).
- Transposição é trivial e a mesma progressão pode ser reinterpretada em outro modo.
- `V/7` (baixo arábico) é uma notação própria do app — documentada na UI (dica de ferramenta).
- Análise de acordes absolutos (`RomanChord::analyze`) não infere dominantes secundárias: `E7` em Dó vira
  `III7` (as sugestões, porém, geram `V7/vi` explicitamente).

## Alternativas consideradas
- Acidentes relativos à escala maior: comum em pop, mas exige `bIII`/`bVI`/`bVII` em toda progressão menor e
  não generaliza para os 21 modos.
- Qualidade sempre explícita: simples, mas grava dados deriváveis (fere a normalização exigida).
- Cifras invertidas com figurado barroco (`V6`, `V64`): conflita com acordes de sexta (`I6`).
