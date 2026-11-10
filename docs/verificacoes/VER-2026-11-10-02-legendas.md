# VER-2026-11-10-02 — Legendas ao passar o mouse em todo o app

| Campo | Valor |
|---|---|
| Id | VER-2026-11-10-02 |
| Título | Legendas ao passar o mouse (glossário + camada única de tooltip) |
| Gravidade | nenhuma (funcionalidade) |
| Tipo | funcionalidade |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH (pedido e aprovação do mockup 9; "verifique para adicionar mais legendas") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/core/glossary.ts`, `ui/src/shared/TooltipLayer.tsx`, `ui/src/app/tooltips.ts`, `ui/src/app/App.tsx`, `ui/src/app/Workspace.tsx`, `ui/src/app/menuModel.ts`, `ui/src/shared/components.tsx`, `ui/src/shared/ui.css`, painéis (Mixer/efeitos, Gravação, Mapa de tempo, Análise, Bases, Harmonia, Letra, Afinação, Transporte, Exportar), testes e docs |

## Problema

O usuário pediu legendas ao passar o mouse em setores (Mixer, Gravação…), efeitos (equalizador,
reverb…) e itens (limiar, ataque, liberação…) "e todas as outras features e itens". Antes, só havia
alguns `title` nativos soltos (100 no código), sem explicação do que cada controle faz.

## Era proposital?

Não. A UI tinha `title` só para atalhos (ex.: "Salvar (Ctrl+S)"); nenhum ADR ou spec decidia não
explicar controles. A central de ajuda (ADR-0033) explica fluxos, não cada controle.

## Solução

- **Glossário único** em português ([core/glossary.ts](../../ui/src/core/glossary.ts)): ~190 entradas
  (título, explicação, faixa/valor típico) para setores, efeitos, todos os parâmetros de efeito, mixer,
  transporte, arranjo, gravação, tempo, menus, partitura, letra, afinação, harmonia, bases, exportação,
  análise e barra de status; padrões do nome acessível (`Armar …`, `Mute …`) cobrem controles repetidos.
- **Camada genérica** ([shared/TooltipLayer.tsx](../../ui/src/shared/TooltipLayer.tsx)): um único
  ouvinte no documento, atraso de 0,5 s (0,25 s por foco de teclado), Esc/roda/clique fecham,
  `aria-describedby` enquanto aberta; `title` nativo vira legenda (sem balão duplicado).
- **Resolvedor do app** ([app/tooltips.ts](../../ui/src/app/tooltips.ts#L26)): `data-tip` (chave) →
  menu (`data-menu-id`) → aba do setor → nome acessível → título nativo. Chave inexistente não aparece crua.
- Liga/desliga em *Exibir › Legendas ao passar o mouse* (lembrado no armazenamento local).

## Por que esta solução

Uma camada e um glossário: o texto fica num lugar só (revisável), os componentes só ganham uma chave
quando o contexto importa (ex.: "Limiar" do compressor × do gate), e a cobertura é testável.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| `title` nativo em cada controle | Atraso e estilo do sistema, sem formatação (título/faixa), não aparece por teclado, texto espalhado em dezenas de arquivos |
| Componente `<Tooltip>` envolvendo cada controle | Centenas de embrulhos, re-render por controle e risco de esquecer controles novos |
| Biblioteca (Tippy/Radix) | Dependência a mais para algo pequeno; a camada única cobre o caso com ~140 linhas |
| Texto da legenda no próprio componente | Duplicaria explicações repetidas (Mute/Solo/Armar em vários painéis) e dificultaria revisar o português |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/core/glossary.ts](../../ui/src/core/glossary.ts#L229) | 1–243 | Novo: glossário, `tipByKey`, `tipByName`, `tipForPanelTitle` |
| [ui/src/shared/TooltipLayer.tsx](../../ui/src/shared/TooltipLayer.tsx#L46) | 1–137 | Nova camada (busca de alvo, posição, acessibilidade) |
| [ui/src/app/tooltips.ts](../../ui/src/app/tooltips.ts#L26) | 1–78 | Resolvedor, preferência liga/desliga, gancho só no modo simulado |
| [ui/src/app/App.tsx](../../ui/src/app/App.tsx#L26) | 9–26 | Monta a camada na raiz |
| [ui/src/shared/components.tsx](../../ui/src/shared/components.tsx) | `Field` | Prop `tip` |
| [ui/src/contexts/mixing/EffectEditor.tsx](../../ui/src/contexts/mixing/EffectEditor.tsx) | cabeçalho, `Slider` | Chave por efeito e por parâmetro |
| Painéis (Gravação, Tempo, Análise, Bases, Harmonia, Letra, Afinação, Transporte, Exportar, Workspace) | pontuais | `data-tip` nos controles ambíguos |
| [ui/src/app/menuModel.ts](../../ui/src/app/menuModel.ts) | Exibir | Item "Legendas ao passar o mouse" |
| [ui/src/shared/ui.css](../../ui/src/shared/ui.css) | fim | Estilo `.tooltip`, `.has-tip` |
| [ui/src/app/tooltips.test.tsx](../../ui/src/app/tooltips.test.tsx) | 1–123 | Testes unitários |
| [ui/e2e/tooltips.spec.ts](../../ui/e2e/tooltips.spec.ts), [ui/e2e/tooltips-coverage.spec.ts](../../ui/e2e/tooltips-coverage.spec.ts) | todo | E2E e cobertura |

## Lógica

Antes: ponteiro sobre "Ataque" → nada (ou balão nativo do navegador com o atalho).
Depois: `pointerover` → sobe até 6 ancestrais procurando legenda → espera 0,5 s → mostra
"**Ataque** · Tempo para o compressor agir… · 0,1–200 ms · voz: 5–15 ms" abaixo do controle (acima se
não couber), presa à janela; some ao sair, com Esc, roda ou clique.

## Exemplo

Sonda E2E que percorre os 11 painéis e lista controles visíveis sem legenda: **76 → 0** depois das
entradas novas (menus, zoom, metrônomo, punch, tempo, letra, afinador, harmonia, bases).

## Verificação

- **Verificação 1:** `tsc` limpo; Vitest **229/229**; Playwright **45/45** (inclui 2 E2E novos e o de cobertura).
- **Verificação 2 (outra abordagem):** build de produção ok e **gancho de teste ausente** do bundle
  (`grep __ebikiResolveTip dist` = 0 arquivos); teste de arquitetura 4/4; capturas nos temas escuro e claro.
- **Sabotagens:** (1) removida a entrada `rec.punchSet` → o teste de cobertura falha apontando
  `Gravação :: button "Definir"`; (2) a mesma sabotagem revelou que chave inexistente aparecia crua
  ("rec.punchSet") como legenda → corrigido (chave desconhecida resolve para nada) e coberto por teste.
- Cobertura garantida por teste: todo setor, efeito e parâmetro de efeito tem entrada.

## Riscos

- Diálogos não abertos pela sonda (ex.: criar partitura, gerar base) usam o título nativo ou padrões;
  controles novos nesses diálogos não são vigiados pelo teste de cobertura.
- Textos de valores típicos são orientações gerais de produção, não regras.
