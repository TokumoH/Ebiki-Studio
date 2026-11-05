# ADR-0021: Modelo de notação próprio (pauta + tablatura sobre as mesmas notas) e I/O isolado

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
A Fase 7 pede editor completo de partitura e tablatura; a Fase 9 pede MIDI, MusicXML e Guitar Pro.
A ADR-0010 já define que o modelo `Score` é próprio (Rust) e o alphaTab é só apresentação.
Faltava decidir a forma do modelo e onde fica a conversão de formatos.

## Decisão
1. **Crate `ctx-notation`** (domínio puro, só depende de `shared-kernel`): agregado `Score` →
   `Part` → `Measure` → `Voice` → `Event` → `Note`.
   - Uma **pauta** (`Staff`) pode mostrar notação padrão, tablatura ou as duas, **sobre as mesmas
     notas**: a nota guarda a altura real e, opcionalmente, a posição (corda, traste). Assim não há
     duas fontes de verdade para a mesma música.
   - Durações exatas com frações (`Frac`), sem erro acumulado em quiálteras.
   - Atributos de compasso (armadura, fórmula, repetições) repetidos em todas as partes, com invariante
     verificada; a fórmula de compasso vem dos compassos e o `TempoMap` do `shared-kernel` é derivado
     (`Score::tempo_map`).
   - Spanners (ligadura de expressão, crescendo, diminuendo) referenciam `EventId` estáveis.
   - Toda edição é `Command<Score>` com inverso exato (ADR-0008).
2. **Crate `infra-notation-io`** (Pure Fabrication): MIDI (`midly`), MusicXML (`quick-xml`, `zip`)
   e GPIF (ADR-0020). Sem regra de domínio: o arranjo de notas temporizadas em compassos/vozes
   (quantização, vozes, ligaduras nas barras) é serviço de domínio em `ctx_notation::arrange`.
3. **Segurança na entrada**: arquivo ≤ 32 MiB, entrada de zip ≤ 64 MiB descompactada (leitura com
   `take`, sem confiar no cabeçalho), ≤ 2.000 entradas, XML com profundidade ≤ 128 e ≤ 2 milhões de
   elementos, entidades externas nunca resolvidas, ≤ 500 mil notas MIDI, limites do domínio
   (partes, compassos, vozes, eventos, notas por acorde). Entrada malformada vira `IoError`, nunca
   pânico (testes de fuzz com `proptest`).
4. Dependências fixadas para MSRV estável: `quick-xml 0.37`, `zip 2.4` (só `deflate`),
   `midly 0.5` sem a feature `parallel`.

## Consequências
- O editor (UI) opera sobre o `Score` via IPC; o alphaTab recebe o `Score` convertido para render.
- MusicXML é o formato de troca principal (exportação completa). Recursos sem equivalente direto
  ficam documentados como limitações na spec da Fase 7.
- Partes com notação + tablatura duplicada em duas pautas de MusicXML (estilo MuseScore) são lidas
  como duas pautas independentes.

## Alternativas consideradas
- Duas representações separadas (pauta e tablatura): duplicação e risco de divergência.
- Usar o modelo do alphaTab como domínio: acopla o núcleo a uma biblioteca de UI em TypeScript.
- Ticks inteiros como única medida de duração: quiálteras não exatas (ex.: 7:4 de semifusa).
