# VER-2026-11-17-06 — Exportar trecho, BPM editável, remover faixa com desfazer e nomes (UX-19, UX-20, UX-22, UX-23)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-06 |
| Título | Exportar o trecho do loop com o caminho no aviso; BPM no transporte; desfazer ao remover faixa; nomes em PT-BR |
| Gravidade | baixa |
| Tipo | correção de UX |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/app/{ExportDialog.tsx,Tutorial.tsx,tooltips.test.tsx}`, `ui/src/contexts/transport/{TransportBar.tsx,transport.css}`, `ui/src/contexts/project/{Timeline.tsx,timeline.css,ClipInspector.tsx}`, `ui/src/contexts/mixing/MixerPanel.tsx`, `ui/src/core/glossary.ts`, `ui/src/ipc/mock.ts`, `ui/e2e/{workspace-tools,overflow}.spec.ts`, `ui/e2e/README.md`, `docs/manual/README.md` |

## Problema

- **UX-19:**
  - o Exportar sempre exportava o projeto inteiro;
  - o aviso "Mixagem exportada." não dizia onde o arquivo ficou.
- **UX-20:** o BPM do transporte não era editável; só pelo painel Mapa de tempo.
- **UX-22:**
  - o ✕ de remover faixa ficava colado no ▲▼;
  - remover não dava aviso nem "Desfazer" (só Ctrl+Z).
- **UX-23:**
  - "Arranjo" (aba) × "Linha do tempo" (rótulo acessível e texto do painel Clipe);
  - "Mute" e "Pan" em inglês nos rótulos acessíveis;
  - o tutorial dizia que L faz loop de 4 compassos, mas L religa o último trecho.

## Era proposital?

- **UX-19:** o núcleo já aceitava `range` no `export_mix` (Fase 4); só faltava a interface.
- **UX-20, UX-22, UX-23:** não havia decisão registrada.

## Solução

- **UX-19:**
  - campo **Trecho** com "Projeto inteiro" e, se o loop estiver ligado, "Trecho do loop (1–4)";
  - o aviso mostra o caminho e tem **Copiar caminho**;
  - o simulado registra o pedido para o E2E conferir o trecho enviado.
- **UX-20:**
  - clique no BPM abre um campo (aceita vírgula), e Enter grava no trecho de andamento do cursor (`tempo_set`);
  - Esc cancela, e valores fora de 20–400 são ignorados.
- **UX-22:** o ✕ fica afastado do ▲▼, e o aviso "Faixa “X” removida." traz **Desfazer**.
- **UX-23:**
  - rótulos "Arranjo", "Silenciar X" e "Panorâmica X" (legendas com "(M)" e "(pan)" para quem conhece os termos);
  - o tutorial passa a dizer "L liga o loop (▾ escolhe o trecho)".

## Por que esta solução

- Usa o que o núcleo já oferece (trecho na exportação, `tempo_set`, desfazer).
- Editar o BPM no mostrador é o padrão de Reaper, Ableton e Logic.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| "Mostrar na pasta" no aviso de exportação | Exige ampliar a permissão do opener, hoje restrita à página de releases (ADR-0034). Fica como decisão do usuário |
| Diálogo de confirmação ao remover faixa | Mais lento que desfazer; o desfazer já existe no núcleo |
| Trecho de exportação livre (compassos X–Y) | O loop já é o jeito de marcar um trecho; evita duplicar a interface |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/app/ExportDialog.tsx](../../ui/src/app/ExportDialog.tsx) | Trecho, `go` | Trecho e aviso com o caminho |
| [ui/src/contexts/transport/TransportBar.tsx](../../ui/src/contexts/transport/TransportBar.tsx) | `TempoField` | BPM editável |
| [ui/src/contexts/project/Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx) | `removeTrack`, rótulos | Desfazer e nomes |
| [ui/src/contexts/mixing/MixerPanel.tsx](../../ui/src/contexts/mixing/MixerPanel.tsx) | rótulos | Silenciar e Panorâmica |
| [ui/src/core/glossary.ts](../../ui/src/core/glossary.ts) | `mixer.pan`, `mixer.mute`, `transport.tempo` | Legendas |
| [ui/src/ipc/mock.ts](../../ui/src/ipc/mock.ts) | `export_mix` | Registra o pedido (só no simulado) |

## Lógica

- **Antes:** exportação sempre inteira e sem caminho; BPM só no Mapa de tempo; remoção sem aviso.
- **Depois:** o usuário escolhe o trecho, vê o caminho e pode copiá-lo; digita o BPM no transporte; remove a faixa e
  desfaz no aviso.

## Exemplo

Loop em 1–4: Exportar → "Trecho do loop (1–4)" → o núcleo recebe `range = [0, 15360]` (4 compassos × 3840 tiques) e o
aviso diz "Mixagem exportada em /…/Exportar.wav". Clique em "♩ 120,0", digite 96,5 e Enter: "♩ 96,5".

## Verificação

- **E2E:** BPM (editar com vírgula e Esc cancela); remover faixa e Desfazer; exportar o trecho do loop (rádio, aviso com
  o caminho e "Copiar caminho", `range` enviado ao núcleo).
- **Sabotagem:**
  - com o export enviando `null`, o E2E de UX-19 falhou;
  - sem o aviso de remoção, o de UX-22 falhou.
- **Testes ajustados:** os rótulos novos ("Silenciar voz", legendas) em `tooltips.test.tsx` e `overflow.spec.ts`.
- **Verificação 1:** `tsc`; Vitest 50 arquivos / 327 testes; Playwright completo, 76/76.
- **Verificação 2 (outra abordagem):** `vite build`; sabotagens; `check_readmes.py` ok.

## Riscos

- "Copiar caminho" depende da área de transferência do WebView. Se ela falhar, o caminho continua escrito no aviso.
- O BPM editado no transporte muda o trecho de andamento do cursor. Em projetos com várias mudanças de andamento, só
  aquele trecho muda.
