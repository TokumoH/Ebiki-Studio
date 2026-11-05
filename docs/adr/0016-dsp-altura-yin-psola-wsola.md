# ADR-0016: DSP de altura — YIN, PSOLA de latência constante e WSOLA

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Fases:** 6 (time-stretch) e 8 (afinação)

## Contexto
As Fases 6 e 8 exigem: afinador em tempo real, correção de afinação em tempo real (estilo
"auto-tune"), análise/edição nota a nota em pós-produção e time-stretch sem alterar o tom.
Restrições: código de tempo real sem alocação/lock/pânico, latência total alvo ≤ 15 ms
(o corretor é um caso especial e reporta a própria latência), sem dependências pesadas e
portas prontas para IA (ADR-0011: `PitchDetector`).

## Decisão
Criar o crate `dsp-pitch` (Pure Fabrication, depende só do `shared-kernel`) com:

1. **Detecção: YIN** (função diferença + CMND + limiar absoluto + interpolação parabólica),
   atrás da porta `PitchDetector`. Faixa 60–1500 Hz configurável; janela = período máximo.
2. **Pitch shift em tempo real: TD-PSOLA com grãos reamostrados**, atrás da porta
   `PitchShifter`. Marcas de síntese a cada novo período, marca de análise como múltiplo
   inteiro do período a partir da anterior (coerência de fase e preservação do tempo),
   grão Hann de meia largura `P/razão` lido em `x(a + t·razão)`. Latência **constante**
   `L = ⌈1,5·Pmax + Pmax/razão_min⌉ + 4` (a 48 kHz: ≈ 58 ms com piso de 60 Hz e razão
   mínima 0,5 — padrão; ≈ 34 ms com piso de 80 Hz e razão mínima 0,8, típico de correção vocal). Razão contínua por amostra (curva para render offline).
3. **Correção em tempo real** (`PitchCorrector`): escala como máscara de 12 bits, nota-alvo
   com histerese, `humanize` como fração do desvio preservada e `retune_ms` como constante de
   tempo de um filtro de 1ª ordem na correção em cents.
4. **Análise offline**: trilha YIN a cada 5 ms, mediana de 5 quadros, segmentação com
   confirmação de mudança (3 quadros) e tolerância a lacunas; render de correções por nota
   (deslocamento em cents + achatamento de deriva) usando o mesmo PSOLA com latência compensada.
5. **Time-stretch offline: WSOLA** (quadro 2048, salto N/2, busca ±512 com correlação
   normalizada, grossa+fina), deslocamento único para todos os canais (coerência estéreo).
   Pitch shift offline = WSOLA + reamostragem sinc-Blackman com anti-aliasing.

## Consequências
- Sem FFT nem crates externos de DSP; custo previsível e testável com sinais sintéticos.
- O PSOLA com reamostragem desloca formantes junto (efeito "esquilo" perceptível acima de
  ~±3 semitons). Para correção de afinação (±1 semitom) é inaudível; preservação de formantes
  (`formant_semitones` já existe no domínio) fica para uma volta futura (LPC/envelope cepstral).
- A latência do corretor (~34–58 ms) é maior que a meta de monitoração; aceitável para uso em
  mixagem/insert com compensação de atraso (PDC) — documentado na spec como risco R3.
- WSOLA é ótimo para material monofônico/rítmico; material polifônico denso pode exibir
  "phasing" leve. Um phase vocoder com phase locking pode entrar como outra implementação
  de `TimeStretcher` sem mudar o domínio.

## Alternativas consideradas
- **MPM (McLeod)**: semelhante ao YIN; YIN tem mais referências e limiar interpretável.
- **Phase vocoder para tempo real**: latência maior (janelas de 2048–4096) e artefatos em
  transientes; PSOLA é o padrão para voz monofônica.
- **Shifter de linha de atraso (dois taps cruzados)**: simples, porém com batimento/comb quando
  o tamanho da janela não é síncrono ao período.
- **Crates externos (rubberband, rustfft)**: dependências nativas/licença (GPL) ou desnecessárias.
