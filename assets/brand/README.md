# Marca — Ebiki Studio

A marca é **obra do autor** e é preservada sem alteração de forma: tinta sumi pura, com os dois tons
codificados no canal alfa (preenchimento ≈ 75 %, traços 100 %). Qualquer textura só **multiplica o alfa**.

| Arquivo | Uso |
|---|---|
| `ebiki-logo-original.svg` | Arquivo original enviado pelo autor (PNG embutido em página A4) |
| `ebiki-mark.png` | Marca recortada (98×210, tinta sumi, fundo transparente) |
| `ebiki-mark@4x.png` | Marca original ampliada 4× (Lanczos), sem textura |
| `ebiki-icon-1024.png` | Ícone do app: marca original em tinta sobre papel Agridoce (`#f3efe9`), cantos arredondados |
| `ebiki-mark-suave@8x.png` | **Marca em uso**: bordas suavizadas como contorno de pincel (784×1680) |
| `bordas-suavizadas-comparacao.png` | Original × suavização leve × média (adotada) |
| `opcoes-cores.png` | 8 opções de cor sólida sobre a marca suavizada (papel, noite e ícone) |

- Na UI a marca é uma **máscara CSS**; a cor vem do tema (`--brand-fill`): tinta papel `#eee7f2` no escuro, tinta `#1d2430` no claro.
- **Suavização das bordas** (`scripts/brand_smooth_edges.py ebiki-mark.png saida.png 4.5`, requer numpy/scipy/Pillow): cada conjunto de nível `{alfa ≥ t}` (128 níveis) é
  suavizado (gaussiana σ = 4,5 px em 8×) e re-limiarizado. Tons planos, degradês e pontos internos ficam
  idênticos (diferença máxima no interior < 1 %); só os contornos em escada viram curvas de pincel.
- Para trocar o ícone: recompor `ebiki-icon-1024.png` e regenerar:
  `npm run tauri -- icon crates/app-desktop/icons/source.png -o crates/app-desktop/icons` (apagar `android/` e `ios/` gerados).
- Para nitidez máxima, substituir por exportação vetorial ou PNG ≥ 1024 px de altura.
