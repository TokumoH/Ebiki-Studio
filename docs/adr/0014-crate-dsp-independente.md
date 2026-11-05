# ADR-0014: Crate `dsp` independente do domínio, com contrato `Processor` de tempo real

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
A Fase 2 (DSP e mixagem) precisa de EQ, dinâmica, efeitos, medidores e metrônomo processando
32+ faixas em tempo real. O contexto Mixing (`ctx-mixing`) já modela os efeitos e seus parâmetros
(`Effect`, `EqParams`, `CompressorParams`…), com `serde`, IDs e regras de domínio. Era preciso
decidir onde fica o processamento e como ele recebe parâmetros sem violar a regra de ouro da
audio thread (sem alocação, lock, I/O ou panic) nem acoplar a camada técnica ao domínio.

## Decisão
1. **Crate `crates/dsp` como Pure Fabrication**, sem lógica de domínio e sem I/O. Depende apenas
   do `shared-kernel` (para `Db` e `Pan`, garantindo uma única lei de panorâmica e uma única
   conversão dB↔linear). **Não depende de `ctx-mixing`.**
2. **Structs de parâmetros próprios** (`CompressorParams`, `EqBandParams`, …), `Copy`, sem `serde`,
   com os mesmos nomes de campos e faixas do contexto Mixing. A **engine** (`audio-engine`) faz o
   mapeamento domínio → DSP na thread de controle; não há módulo de mapeamento no `dsp`.
3. **Contrato único `Processor`** (object safe): `prepare(sr, max_block)` (único ponto que aloca),
   `reset()`, `process(&mut [f32], &mut [f32])` estéreo não intercalado *in place* e
   `latency_samples()` para PDC. `set_params` é inerente a cada processador, sem alocação, com
   limitação de faixa (`clamped`, NaN → padrão) e suavização onde haveria zíper.
4. **Medidores** também implementam `Processor` (análise pura, sinal intacto) e oferecem
   `analyze(&[f32], &[f32])` para fatias imutáveis.
5. **Verificação automática**: `assert_no_alloc` (alocador global num binário de teste próprio,
   modo *warn* com contagem de violações em debug **e** release), `proptest` para finitude e limites
   com parâmetros arbitrários (inclusive NaN/∞), e teste de orçamento de CPU (32 faixas).

## Consequências
- `dsp` pode ser reutilizado por `synth`, simulação offline e futuros hosts de plugin sem arrastar
  o domínio; o domínio pode evoluir (novos campos, serde) sem recompilar o DSP.
- Duplicação controlada dos structs de parâmetros (≈ 10 structs pequenos). Mitigação: mesmos nomes
  e faixas; o teste de mapeamento ficará na engine (ida domínio → DSP para todos os `Effect`).
- O `dsp` limita parâmetros por conta própria (defesa em profundidade): mesmo que a engine envie
  valores inválidos, nada explode.

## Alternativas consideradas
- **`dsp` depender de `ctx-mixing`** e consumir `Effect` diretamente: acopla a camada técnica ao
  domínio (viola a convenção de contextos e o GRASP Pure Fabrication) e traz `serde`/`uuid` para o
  caminho de tempo real.
- **Mover os structs de parâmetros para o `shared-kernel`**: incha o núcleo compartilhado com
  conceitos de um único contexto.
- **Parâmetros como atômicos dentro de cada processador**: decisão adiada para a engine (ADR-0005);
  o `dsp` permanece síncrono e determinístico, e a engine entrega parâmetros entre blocos.
