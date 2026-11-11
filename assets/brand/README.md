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

- **Cor (decisão do autor, 2026-11-06):** sem cor — tinta papel `#eee7f2` no tema escuro e tinta sumi `#111014` no claro. Na UI a marca é uma **máscara CSS** com a cor em `--brand-fill`.
- **Suavização das bordas** (`scripts/brand_smooth_edges.py ebiki-mark.png saida.png 4.5`, requer numpy/scipy/Pillow): cada conjunto de nível `{alfa ≥ t}` (128 níveis) é
  suavizado (gaussiana σ = 4,5 px em 8×) e re-limiarizado. Tons planos, degradês e pontos internos ficam
  idênticos (diferença máxima no interior < 1 %); só os contornos em escada viram curvas de pincel.
- Para trocar o ícone: recompor `ebiki-icon-1024.png` e regenerar:
  `npm run tauri -- icon crates/app-desktop/icons/source.png -o crates/app-desktop/icons` (apagar `android/` e `ios/` gerados).
- Para nitidez máxima, substituir por exportação vetorial ou PNG ≥ 1024 px de altura.

## Onde

| Arquivo | O que contém |
|---|---|
| [LICENSE.md](LICENSE.md) | Licença da marca (nome e logo reservados; não cobertos pela GPL) |
| [bordas-suavizadas-comparacao.png](bordas-suavizadas-comparacao.png) | Comparação antes/depois da suavização das bordas |
| [ebiki-icon-1024.png](ebiki-icon-1024.png) | Ícone do app em 1024 px (fonte dos ícones do Tauri) |
| [ebiki-logo-original.svg](ebiki-logo-original.svg) | Logo original em vetor |
| [ebiki-mark-suave@8x.png](ebiki-mark-suave@8x.png) | Marca com bordas suavizadas em 8× |
| [ebiki-mark.png](ebiki-mark.png) | Marca em tamanho base |
| [ebiki-mark@4x.png](ebiki-mark@4x.png) | Marca em 4× |
| [opcoes-cores.png](opcoes-cores.png) | Opções de cor estudadas para a marca |
