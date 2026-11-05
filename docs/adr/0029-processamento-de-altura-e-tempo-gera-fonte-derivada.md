# ADR-0029: Correção de afinação, time-stretch e partitura renderizada geram fontes derivadas; corretor em tempo real como insert

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Fases 6–8 introduzem processamentos offline caros e destrutivos por natureza (PSOLA nota a nota,
WSOLA, síntese GM de partitura) e um processamento em tempo real (corretor por escala). Era preciso
decidir como o resultado entra no projeto mantendo o desfazer exato (ADR-0008) e o original intacto.

## Decisão
1. **Fonte derivada**: o resultado offline é gravado em `audio/rendered/<uuid>.wav` (WAV float 32, via
   porta `MediaPort::encode`) e registrado como **nova** `AudioSource` (hash de conteúdo; conteúdo
   idêntico reaproveita a fonte existente). O clipe é trocado por um clipe equivalente da fonte nova
   (`DeleteClip` + `AddClip`, mesma faixa/posição/ganho) num único `Composite` desfazível, como
   `Studio::import_audio`. A fonte original e o mapa de correção (chaveado pela fonte **original**)
   permanecem; reaplicar após novas edições parte sempre do original (sem degradação acumulada).
   - Afinação: `tuning_apply{sourceId, clipId}`.
   - Time-stretch: `clip_time_stretch{clipId, sourceBpm, targetBpm?}` (trecho do clipe, razão
     `source/target`, WSOLA multicanal coerente).
   - Partitura: `score_render_to_track{scoreId, trackId?, atTick}` (fonte + faixa/canal opcionais +
     clipe num único desfazer).
2. **Corretor em tempo real** = novo efeito `Effect::PitchCorrector(PitchCorrectorParams)` no contexto
   Mixing (`enabled`, `keyRoot` 0–11, `scaleMask` 12 bits, `retuneMs`, `humanize`, `a4Hz`), mapeado no
   `DspProcessorFactory` para `dsp_pitch::PitchCorrector` (soma mono → correção → mesmo sinal nos dois
   canais; buffers de `MAX_BLOCK` pré-alocados; `latency_samples()` do PSOLA reportada). A persistência
   reaproveita `effect_param` (números inteiros exatos voltam como inteiro JSON para os campos
   `u8`/`u16`).
3. Arquivos de fontes derivadas que deixarem de ser referenciados após desfazer **não** são apagados
   imediatamente (o refazer precisa deles); a limpeza fica para uma rotina futura de "coletar mídia não
   usada" no salvar/fechar.

## Consequências
- Desfazer/refazer é exato e barato (troca de referências), sem re-renderizar.
- Custo de disco: cada aplicação gera um arquivo novo (aceitável em uso pessoal; ver item 3).
- A engine ainda **não** compensa latência de plugins (PDC): o insert de correção atrasa a faixa em
  ≈ 58 ms com os padrões. Documentado como limitação; PDC entra numa volta futura da engine.
- Testes: `app-core/tests/music.rs` (`tuning_analyze_apply_corrects_pitch_and_undo_restores_original_clip`,
  `time_stretch_to_project_tempo_has_expected_length_and_keeps_pitch`,
  `pitch_corrector_insert_pulls_452_hz_to_440_in_the_full_engine`,
  `score_audition_and_render_to_track_is_one_undo`), `infra-adapters/tests/no_alloc.rs`,
  `infra-persistence` (`pitch_corrector_effect_roundtrips_with_integer_params`).
