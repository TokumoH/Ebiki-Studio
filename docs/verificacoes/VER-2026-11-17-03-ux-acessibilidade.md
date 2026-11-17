# VER-2026-11-17-03 — Acessibilidade (UX-06, UX-10, UX-11)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-03 |
| Título | Foco preso nos diálogos e botão ✕; contraste AA nos dois temas; tutorial com Enter e sem atalhos por baixo |
| Gravidade | média |
| Tipo | correção de acessibilidade |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH ("corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/shared/{components.tsx,components.test.tsx,ui.css,tokens.css,contrast.test.ts (novo),README.md}`, `ui/src/app/Tutorial.tsx`, `ui/e2e/{identity,composition,updates}.spec.ts`, `ui/e2e/README.md`, `docs/diagramas/paletas-ebiki.html` |

## Problema

- **UX-06:** com um diálogo aberto, Tab levava o foco para a tela de trás (menu, renomear projeto), atrás do fundo escuro.
- **UX-10:** no tema claro, os textos de aviso (2,96:1), ok (2,94:1) e "faint" (3,17:1) ficavam abaixo do AA (4,5:1).
  O teste novo achou também `--fg-faint` sobre `--panel-2` no tema escuro (4,27:1).
- **UX-11:** no tutorial, Enter não avançava (o comentário dizia que sim), e Espaço, R e Delete disparavam por baixo.

## Era proposital?

- **UX-06:** não. O `aria-modal` declarava o diálogo modal, mas o foco não era contido.
- **UX-10:** as paletas foram conferidas para fundo e destaque (`docs/diagramas/paletas-ebiki.html`), não para as cores de
  estado usadas como texto.
- **UX-11:** o comentário de `Tutorial.tsx` prometia Enter. Faltou implementar.

## Solução

- **UX-06:** o `Dialog` circula Tab e Shift+Tab entre os focáveis visíveis (`checkVisibility`) e traz Tab de fora para
  dentro. Ganha um **✕** no cabeçalho ("Fechar diálogo", Esc), o último na volta do Tab. O foco inicial continua no
  primeiro campo do conteúdo.
- **UX-10:**
  - tema claro: `--fg-faint #57606f`, `--ok #1a6e49`, `--warn #7d5000`;
  - tema escuro: `--fg-faint #958eac`;
  - `contrast.test.ts` lê `tokens.css` e exige ≥ 4,5:1 para fg, fg-muted, fg-faint, ok e warn sobre os 5 fundos dos dois
    temas (50 pares);
  - os medidores continuam com as cores vivas (`--meter-*`).
- **UX-11:** Enter avança (fora dos botões do cartão). As demais teclas não chegam aos atalhos do app enquanto o tutorial
  está aberto. Tab e a ativação dos botões do cartão seguem normais.

## Por que esta solução

- Prender o foco no componente comum corrige todos os diálogos de uma vez.
- O teste de contraste a partir dos tokens impede regressão quando a paleta mudar.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| `inert` no resto da página | O diálogo é renderizado dentro do mesmo `#root`; exigiria portal e mudaria a árvore de todos os diálogos |
| Cores de estado só como ícone, sem texto | Mensagens como "Sem dispositivo de áudio" precisam de texto |
| Bloquear atalhos do tutorial no Workspace | Espalharia a regra; o tutorial já intercepta o teclado na captura |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/shared/components.tsx](../../ui/src/shared/components.tsx) | `focusables`, `Dialog` | Foco preso e ✕ |
| [ui/src/shared/tokens.css](../../ui/src/shared/tokens.css) | 20, 69, 77–78 | Cores AA |
| [ui/src/shared/contrast.test.ts](../../ui/src/shared/contrast.test.ts) | todo | 50 pares |
| [ui/src/app/Tutorial.tsx](../../ui/src/app/Tutorial.tsx) | teclado | Enter e bloqueio de atalhos |
| [ui/e2e/identity.spec.ts](../../ui/e2e/identity.spec.ts) | fim | 2 E2E |
| [ui/e2e/composition.spec.ts](../../ui/e2e/composition.spec.ts), [updates.spec.ts](../../ui/e2e/updates.spec.ts) | botão Fechar | `exact: true` (o ✕ "Fechar diálogo" também casava) |

## Lógica

- **Antes:** Tab no último botão saía do diálogo. Enter no tutorial parava o transporte. Textos de aviso apagados no tema claro.
- **Depois:** Tab volta ao ✕ do cabeçalho. Enter vai ao passo 2. Os textos passam a 5,2–6,9:1.

## Exemplo

Exportar aberto: Tab 25 vezes mantém o foco sempre dentro do diálogo. "Sem dispositivo de áudio" na barra de status
do tema claro: de 2,96:1 para 5,56:1 (`#7d5000` sobre `#ebe5dc`).

## Verificação

- **TDD:**
  - `components.test.tsx`: Tab/Shift+Tab circulam, Tab vindo de fora entra, ✕ fecha;
  - `contrast.test.ts`: falhou em 16 pares antes da troca das cores e passa nos 50 depois.
- **E2E:** tutorial (Enter, Espaço/R sem efeito, Esc); foco preso no Exportar.
- **Sabotagem:** sem o Tab circular e sem o bloqueio no tutorial, os dois E2E falharam.
- **Ajuste durante a verificação:** dois E2E procuravam "Fechar" sem `exact` e passaram a casar com o ✕. Os seletores
  ficaram exatos e as verificações recomeçaram.
- **Disco:** o disco encheu durante a troca das cores (build paralelo do S1). Liberado com `cargo clean`. Os arquivos
  foram conferidos (`git diff`, contagem de linhas) e o teste de contraste rodou de novo.
- **Verificação 1:** `tsc`; Vitest 49 arquivos / 322 testes; Playwright completo, 68/68.
- **Verificação 2 (outra abordagem):** `vite build`; sabotagens; `check_readmes.py` ok.

## Riscos

- Diálogos próprios que não usam o `Dialog` comum (o cartão do tutorial, os popovers) não prendem o foco. O tutorial
  bloqueia os atalhos, e os popovers fecham com Esc ou clique fora.
- A cor `--ok` mais escura também aparece em ícones e bordas do tema claro (menos viva, mas legível).
