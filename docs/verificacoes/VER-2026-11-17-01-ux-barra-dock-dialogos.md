# VER-2026-11-17-01 — Barra superior, dock e diálogos em telas pequenas (UX-02, UX-03, UX-07)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-01 |
| Título | Barra superior utilizável em janela estreita; Arranjo com espaço em janela baixa; rodapé dos diálogos fixo |
| Gravidade | alta (UX-02) · média (UX-03, UX-07) |
| Tipo | correção de UX |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/contexts/transport/{TransportBar.tsx,transport.css}`, `ui/src/app/{Workspace.tsx,app.css}`, `ui/src/panels/dockControl.ts`, `dockControl.test.ts` (novo), `ui/src/shared/ui.css`, `ui/e2e/overflow.spec.ts`, `ui/src/panels/README.md` |

## Problema

- **UX-02:** em 640 px de largura (zoom 200%):
  - os botões ▶ ⏹ ⏺ encolhiam para 8 a 13 px;
  - o grupo Loop cobria o Importar;
  - o Seguir ficava coberto.

  Em 800 px os botões de transporte já ficavam estreitos.
- **UX-03:** o grupo de baixo do dock tinha 330 px fixos. Com 480 px de altura, o Arranjo ficava com cerca de 40 px.
- **UX-07:** o diálogo inteiro rolava. Em Atalhos de teclado, "Fechar" ficava abaixo da janela.

## Era proposital?

- `overflow: hidden` na barra é proposital (VER-2026-11-11-01), mas faltava impedir que os botões encolhessem.
- Os 330 px vêm do layout da Fase 1, pensados para 1080p.
- O `overflow: auto` no `.dialog` é da Fase 1, antes de existirem diálogos altos.

## Solução

- **UX-02:**
  - nada na barra encolhe (`flex-shrink: 0`);
  - até 1100 px, Metrônomo, Seguir e o trecho do Loop viram só ícone (o nome segue no `aria-label` e na legenda);
  - até 900 px somem Importar e Exportar, que continuam no menu da logo, em Arquivo e em Ctrl+I/Ctrl+E;
  - até 760 px some o andamento;
  - até 720 px some o nome do projeto, que continua no menu da logo;
  - `▾` do loop com no mínimo 24 px.
- **UX-03:** `bottomDockHeight()` = metade da área dos painéis, entre 120 e 330 px.
- **UX-07:** o `.dialog` não rola. Só o `.dialog-body` rola; título e rodapé ficam fixos.

## Por que esta solução

Esconder rótulos e itens duplicados por largura é o padrão das barras de ferramentas (Reaper, VS Code). O transporte
continua inteiro e clicável em qualquer tamanho.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Menu "⋯" para Importar/Exportar | Repetiria o menu da logo, que já tem os dois a um clique |
| Quebrar a barra em duas linhas | Tira altura do Arranjo justamente nas telas pequenas |
| Dock em 40% (primeira tentativa) | Em 1280×720 a Partitura ficava com 247 px e o E2E de inserir notas falhou (a pauta saía da área). Com metade, ela tem 309 px e o Arranjo continua com mais de 120 px em 853×480 |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/contexts/transport/transport.css](../../ui/src/contexts/transport/transport.css) | início | Sem encolher; rótulos por largura |
| [ui/src/contexts/transport/TransportBar.tsx](../../ui/src/contexts/transport/TransportBar.tsx) | Metrônomo, Loop, Seguir | Rótulo em `.tb-label` |
| [ui/src/app/app.css](../../ui/src/app/app.css) | `.topbar` | Importar/Exportar e nome por largura |
| [ui/src/panels/dockControl.ts](../../ui/src/panels/dockControl.ts) | `bottomDockHeight` | Altura proporcional |
| [ui/src/shared/ui.css](../../ui/src/shared/ui.css) | `.dialog` | Corpo rolável, cabeçalho e rodapé fixos |
| [ui/e2e/overflow.spec.ts](../../ui/e2e/overflow.spec.ts) | fim | 5 E2E novos |

## Lógica

- **Antes:** a barra espremia os botões e deixava um elemento cobrir o outro.
- **Depois:** os itens mantêm o tamanho, e os secundários e os rótulos somem por faixa de largura. O dock calcula a
  altura pela área disponível. Nos diálogos, só o corpo rola.

## Exemplo

Em 640×360: transporte com 32 px por botão, Loop e ▾ inteiros, Seguir só com ícone, Salvar visível e o menu até
"Ajuda". Em 853×480: o Arranjo fica com 189 px, contra cerca de 40 px antes.

## Verificação

- **E2E novos:**
  - barra e menu sem controle estreito (menos de 22 px), fora da janela ou coberto, em 640×360, 800×600 e 1024×640;
  - diálogo alto com o botão Fechar dentro da janela e corpo `overflow: auto`;
  - Arranjo com pelo menos 120 px em 853×480.
- **Vitest:** `bottomDockHeight`.
- **Sabotagem:** com 330 px fixos e o `overflow: auto` antigo, os E2E de UX-03 e UX-07 falharam. Antes da correção, o
  E2E de UX-02 listava 11, 5 e 1 problemas nas três larguras.
- **Ajuste durante a verificação:** 40% → 50% (tabela acima). As verificações recomeçaram do zero.
- **Verificação 1:** `tsc`; Vitest 46 arquivos / 264 testes; Playwright completo, 63/63.
- **Verificação 2 (outra abordagem):** `vite build`; capturas da barra em 640, 900 e 1280 px conferidas visualmente;
  `check_readmes.py` ok.

## Riscos

- Abaixo de 600 px a barra corta o fim (Salvar) por causa do `overflow: hidden`; o mínimo prático do app é 640 px.
- Um layout salvo pelo usuário mantém a altura salva; a nova altura vale para o layout padrão e para *Restaurar layout padrão*.
