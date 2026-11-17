# VER-2026-11-17-05 — Atalhos, números em PT-BR e dock em PT-BR (UX-13, UX-14, UX-15, UX-16)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-05 |
| Título | Editor de atalhos com conflito e restaurar; Delete global; vírgula decimal e rótulos sem maiúsculas; dockview em PT-BR |
| Gravidade | baixa |
| Tipo | correção de UX |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/app/{ShortcutsDialog.tsx,shortcuts.ts,shortcuts.test.ts,menuModel.ts,Workspace.tsx,ExportDialog.tsx,app.css}`, `ui/src/shared/{keys.ts,keys.test.ts,MenuBar.tsx,components.tsx,components.test.tsx,ui.css}`, `ui/src/core/{format.ts,format.test.ts}`, `ui/src/panels/{dockMessages.ts,DockLayout.tsx}`, contextos (TuningPanel, EffectEditor, AdvisorPanel, ClipInspector, ClipView, TransportBar, Timeline), `ui/e2e/{workspace-tools,workspace}.spec.ts`, `docs/manual/README.md`, READMEs |

## Problema

- **UX-13:**
  - dava para dar a mesma tecla a duas ações (vencia a primeira, em silêncio);
  - não havia "Restaurar padrões";
  - aparecia "Space" enquanto o resto do app diz "Espaço";
  - no macOS, ⌘ aparecia como Ctrl;
  - faltavam End, Ctrl+N e Ctrl+O.
- **UX-14:** o menu mostrava Delete como atalho global, mas ele só funcionava com o foco no Arranjo.
- **UX-15:**
  - os rótulos em maiúsculas viravam "10.7 MS", "48 KHZ" (unidade SI errada);
  - os decimais misturavam vírgula e ponto ("44,1 kHz" × "120.0", "0.0 dB").
- **UX-16:** os botões das abas e os anúncios do dockview estavam em inglês ("Close Mixer", "Clipe opened").

## Era proposital?

- O formato gravado dos atalhos ("Ctrl+Shift+K") é proposital (VER-2026-11-05) e continua igual; só a exibição muda.
- As maiúsculas nos rótulos eram estilo visual, sem considerar as unidades.
- O dockview tem a opção `messages` desde a v8, mas ela não foi configurada.

## Solução

- **UX-13:**
  - `findConflict` e `resetShortcuts`;
  - o editor avisa "X já é de “Loop”", com **Substituir** (a outra ação fica sem atalho, "—") e **Cancelar**; Esc cancela a captura;
  - **Restaurar padrões** volta tudo ao original;
  - `displayKeys` (shared) mostra Espaço, setas, Esc e, no macOS, ⌘ ⌥ ⇧, no editor e no menu;
  - atalhos novos: **End** (fim do projeto), **Ctrl+N**, **Ctrl+O**.
- **UX-14:**
  - Delete vira atalho global "Excluir clipe selecionado", editável;
  - o Arranjo mantém só o Backspace local, sem exclusão dupla;
  - o menu passa a mostrar a tecla configurada.
- **UX-15:**
  - rótulos sem `text-transform: uppercase`;
  - `formatDecimal` (core) aplica a vírgula em dB, BPM, latência, qualidade Vorbis, afinador, efeitos, análise e clipe.
- **UX-16:** `DOCK_MESSAGES` em PT-BR, passado ao `DockviewReact`.

## Por que esta solução

- Avisar antes de trocar segue o padrão dos editores de atalhos (VS Code, Reaper).
- Separar formato gravado e exibição mantém os atalhos salvos compatíveis.
- A vírgula decimal é o padrão do PT-BR.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Trocar o formato gravado para "Espaço" | Quebraria os atalhos já salvos pelo usuário |
| `toLocaleString("pt-BR")` | Inclui separador de milhar ("1.024 amostras") e é mais lento em telas a 30 Hz |
| Delete global sem tirar o local | Excluiria duas vezes (o segundo dá erro "clipe não encontrado") |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/app/ShortcutsDialog.tsx](../../ui/src/app/ShortcutsDialog.tsx) | todo | Conflito, restaurar, teclas em PT-BR |
| [ui/src/app/shortcuts.ts](../../ui/src/app/shortcuts.ts) | padrões, `findConflict`, `resetShortcuts` | End, Delete, Ctrl+N/O |
| [ui/src/shared/keys.ts](../../ui/src/shared/keys.ts) | todo | `displayKeys` |
| [ui/src/core/format.ts](../../ui/src/core/format.ts) | `formatDecimal` | Vírgula decimal |
| [ui/src/panels/dockMessages.ts](../../ui/src/panels/dockMessages.ts) | todo | Textos do dock |
| [ui/src/shared/ui.css](../../ui/src/shared/ui.css) | `.field > span` | Sem maiúsculas |

## Lógica

- **Antes:** a tecla repetida era gravada em silêncio, e Delete fora do Arranjo não fazia nada.
- **Depois:** a tecla repetida mostra um aviso e só troca com confirmação. Delete exclui o clipe selecionado de qualquer
  painel. Os números aparecem com vírgula e o dock fala PT-BR.

## Exemplo

Atribuir L ao Metrônomo → "L já é de “Loop”" → Substituir: Metrônomo = L e Loop = "—". *Restaurar padrões*: Metrônomo =
C e Loop = L. Barra de transporte: "♩ 120,0".

## Verificação

- **TDD:**
  - `keys.test.ts` (PT-BR e macOS);
  - `shortcuts.test.ts` (conflito, restaurar, padrões sem repetição e com End/Ctrl+N/Ctrl+O/Delete);
  - `format.test.ts` (`formatDecimal`, `formatDb` com vírgula).
- **E2E:** editor de atalhos (Espaço, conflito, substituir, restaurar); Delete com foco no Mixer (exclui um clipe, sem
  erro); aba "Fechar Mixer" e nenhum "Close …".
- **Sabotagem:** sem `findConflict` e sem `messages`, os E2E de UX-13 e UX-16 falharam.
- **Testes ajustados:** os existentes que conferiam "0.0 dB", "−6.0" e "-2.0 dB" passaram a usar vírgula (mudança pedida).
- **Verificação 1:** `tsc`; Vitest 50 arquivos / 327 testes; Playwright completo, 73/73.
- **Verificação 2 (outra abordagem):** `vite build`; sabotagens; `check_readmes.py` ok.

## Riscos

- Campos editáveis (BPM no Mapa de tempo) continuam aceitando e mostrando ponto, para não mudar a entrada numérica do
  navegador. A latência do diálogo de áudio ganha vírgula junto com o S1.
- Ctrl+N e Ctrl+O podem ser capturados pelo WebView em alguns sistemas antes do app; o menu continua disponível.
