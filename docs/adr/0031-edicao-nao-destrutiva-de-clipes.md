# ADR-0031: Edição de clipes não destrutiva (janela + processamento na engine) e espectrograma como fabricação pura

- **Data:** 2026-11-06
- **Status:** Aceita

## Contexto
A Fase 3 (RF-3.1 a RF-3.5) pede cortar, dividir, fades/crossfades, normalizar, ganho de clipe,
inverter fase, reverter e um espectrograma selecionável. O RNF-07 exige que os arquivos de origem
**nunca** sejam modificados, e o desfazer precisa continuar exato e barato (ADR-0008). Havia duas
famílias de solução:

| Alternativa | Prós | Contras |
|---|---|---|
| A. Renderizar cada edição numa fonte derivada (como afinação/time-stretch, ADR-0029) | engine inalterada | cada fade/corte gera arquivo; desfazer troca fontes; lento para arrastes contínuos; degrada em cadeia |
| B. **Clipe = janela + parâmetros**, aplicados na reprodução | instantâneo, sem arquivos novos, desfazer = troca de valores, arrastes contínuos viáveis | engine precisa aplicar curva por amostra sem alocar |

## Decisão
1. **Alternativa B.** O agregado `Project` ganha o Value Object `ClipEdit` (`fade_in_frames`,
   `fade_out_frames`, `fade_in_shape`, `fade_out_shape`, `polarity_inverted`, `reversed`) no `Clip`
   (serde *flatten*, campos com `default` → JSON antigo continua válido). Invariante:
   `fade_in + fade_out ≤ length` (com `checked_add`), validada em todo `insert/replace/restore`.
2. Cortar e dividir só mudam a janela `[offset, offset + length)` e o início. A divisão recebe um **tick**
   (cursor) e calcula a amostra de corte pelo mapa de tempo, então a segunda parte começa exatamente na
   amostra correspondente: **dividir não altera o áudio** (provado por render offline). Em clipe
   reverso, a borda esquerda da timeline corresponde ao fim da região na fonte.
3. Comandos desfazíveis no contexto Project: `TrimClip`, `SplitClip` (id da segunda parte criado no
   construtor → refazer idêntico), `SetClipFades`, `SetClipPolarity`, `SetClipReversed`, `Crossfade`
   (clipes da mesma faixa com sobreposição **parcial**; fades de saída/entrada = sobreposição).
   Base comum privada `EditOne` (substitui o clipe e guarda o anterior).
4. **Normalizar** é caso de uso de aplicação (`edit_service::clip_normalize`): mede o pico da região
   referenciada na fonte decodificada e emite `SetClipGain` (alvo padrão −1 dBFS, limitado ao fader).
5. **Engine:** `ClipRender` recebe fades/forma/polaridade/reverso; `mix_into` usa laço simples quando não
   há fades nem reverso e, caso contrário, `gain_at`/`source_index` por amostra (puro, sem alocação —
   `tests/no_alloc.rs` cobre faixas com fades e reverso). A curva (`FadeShape`: linear, equal power) vive
   no **Shared Kernel** para que domínio, engine e UI usem a mesma fórmula.
6. **Espectrograma (RF-3.5)** é Pure Fabrication em `infra-codecs::spectrogram`: STFT (rustfft), janela de
   Hann, magnitude em dB quantizada para `u8` (−100..0 dBFS), eixo **log-frequência** (20 Hz..Nyquist),
   cache binário `cache/spectrogram/<hash>-<fft>-<hop>-<bins>.spec` com checksum (regenerável, igual ao
   cache de picos). Chega ao `Studio` por um método com implementação padrão em `MediaPort`
   (sem mudar `StudioDeps`) e à UI pelo comando binário `source_spectrogram`.
7. **Persistência:** migração `project_v3.sql` adiciona colunas a `clip` (todas dependem só da chave →
   FNBC preservada); forma da curva com `CHECK` (domínio fechado) em vez de tabela de referência.

## Consequências
- Edição instantânea e desfazível; nenhum arquivo novo é escrito por cortar/dividir/fades/normalizar.
- Custo de CPU por amostra apenas nos clipes com fade/reverso (caminho rápido preservado).
- Fades que cruzam o ponto de divisão são encurtados (entrada fica na 1ª parte, saída na 2ª).
- A UI aproxima o mapa de tempo como constante dentro do clipe ao arrastar bordas; o núcleo
  revalida (fonte da verdade).
- Testes: `ctx-project` (domínio/comandos), `audio-engine/tests/clip_edit.rs` (rampas, polaridade,
  reverso, crossfade, divisão idêntica), `no_alloc.rs`, `infra-persistence` (v2→v3), `infra-codecs`
  (espectrograma/cache), `app-core/tests/edit.rs`, `app-desktop/ipc_tests.rs`, UI (Vitest + E2E
  `ui/e2e/edit.spec.ts`).
