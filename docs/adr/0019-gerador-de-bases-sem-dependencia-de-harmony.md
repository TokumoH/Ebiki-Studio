# ADR-0019: Gerador de bases recebe acordes realizados; vocabulário compartilhado por código

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Relacionadas:** ADR-0003 (crate por contexto), ADR-0006 (dois SQLite), ADR-0007, ADR-0018

## Contexto
A Fase 4 pede bases geradas pelo app a partir de padrões de acordes e ritmos (RF-4.2). Gerar a parte
harmônica exige transformar graus em notas — conhecimento do contexto **Harmony**. A convenção do projeto
proíbe dependência entre contextos (só `shared-kernel`). Além disso, Harmony e Library precisam do mesmo
vocabulário de **gênero** e compartilham o `library.db`.

## Decisão
1. `ctx-library::generate` recebe **acordes já realizados** (`ChordSlot { start_tick, duration_ticks,
   notes, root }`). A camada de aplicação (`app-core`) compõe: `Progression::realize` (Harmony) →
   `ChordSlot` → `generate` (Library). Nenhum dos dois contextos conhece o outro.
2. O gerador é puro e determinístico: padrões numa grade de semicolcheias; saída em eventos MIDI por canal
   (programas General MIDI base 0; bateria no canal 10/índice 9), em ticks de `shared_kernel::PPQ`.
   Padrões de compasso fixo (valsa/guarânia 3/4; bossa, reggae, funk 4/4) são validados contra a fórmula
   de compasso; padrões genéricos de baixo se adaptam a qualquer numerador.
3. A receita de uma base gerada guarda a progressão como **texto em graus** (mesma sintaxe da ADR-0018);
   a validação semântica desse texto é feita pela aplicação usando Harmony.
4. **Gênero** é vocabulário aberto compartilhado pela tabela `genre`: Harmony expõe o enum `Genre` (códigos
   estáveis, semeados na primeira execução); Library usa `GenreCode` (slug validado) e pode referenciar
   gêneros extras sem alterar Harmony.
5. O DDL único do `library.db` fica em `crates/ctx-harmony/sql/library_v1.sql` (exposto como
   `ctx_harmony::LIBRARY_SCHEMA_V1`) e será embutido pela camada de persistência.

## Consequências
- Contextos independentes e testáveis isoladamente; o gerador pode receber voicings de qualquer origem
  (inclusive uma futura porta de IA).
- Um passo extra de composição na aplicação (mapear `RealizedChord` → `ChordSlot`), trivial.
- O texto de progressão de uma base não é validado dentro de `ctx-library` (só tamanho/caracteres).

## Alternativas consideradas
- `ctx-library` depender de `ctx-harmony`: viola a convenção de contextos independentes.
- Mover teoria (graus → notas) para o `shared-kernel`: incharia o núcleo compartilhado com regra de um
  contexto específico.
- Enum `Genre` duplicado nos dois contextos: divergência garantida ao longo do tempo.
