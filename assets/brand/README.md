# Marca — Ebiki Studio

A marca é **obra do autor** e é preservada sem alteração de forma: tinta sumi pura, com os dois tons
codificados no canal alfa (preenchimento ≈ 75 %, traços 100 %). Qualquer textura só **multiplica o alfa**.

| Arquivo | Uso |
|---|---|
| `ebiki-logo-original.svg` | Arquivo original enviado pelo autor (PNG embutido em página A4) |
| `ebiki-mark.png` | Marca recortada (98×210, tinta sumi, fundo transparente) |
| `ebiki-mark@4x.png` | Marca original ampliada 4× (Lanczos), sem textura |
| `ebiki-icon-1024.png` | Ícone do app: marca original em tinta sobre papel Agridoce (`#f3efe9`), cantos arredondados |
| `opcoes-textura.png` | Folha comparativa das opções de pincel (claro e escuro) |
| `opcao-textura-1..4.png` | 1 bordas suaves · 2 pincel seco sutil · 3 kasure leve · 4 tinta absorvida no papel |

- Na UI a marca é uma **máscara CSS**; a cor vem do tema (`--brand-fill`): tinta papel `#eee7f2` no escuro, tinta `#1d2430` no claro.
- **Em uso:** marca original (0). Para adotar uma opção de textura, copiar `opcao-textura-N.png` sobre
  `ebiki-mark@4x.png` e `ui/src/shared/assets/ebiki-mark.png`, recompor o ícone e regenerar:
  `npm run tauri -- icon crates/app-desktop/icons/source.png -o crates/app-desktop/icons` (apagar `android/` e `ios/` gerados).
- Para nitidez máxima, substituir por exportação vetorial ou PNG ≥ 1024 px de altura.
