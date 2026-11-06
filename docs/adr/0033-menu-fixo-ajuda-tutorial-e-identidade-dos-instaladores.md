# ADR-0033: Menu fixo, central de ajuda, tutorial e identidade visual dos instaladores

- **Status:** Aceita
- **Data:** 2026-11-06
- **Contexto delimitado:** UI (`shared`, `app`, `panels`) e empacotamento (`app-desktop`)

## Contexto

A barra superior acumulava botões soltos (⚙ ⌨ ◐ ✕ e um menu de painéis), sem lugar para novas ações
e pouco descobrível. Faltavam ajuda integrada e uma introdução ao primeiro uso. Os instaladores
usavam as telas padrão do NSIS/WiX/DMG, sem a identidade do Ebiki. Mockups aprovados pelo usuário
no canvas de design.

## Decisão

1. **Menu fixo no header** (padrão Photoshop/VS Code), em duas peças independentes:
   - `shared/MenuBar` — componente de design system **dirigido por dados** (`MenuDef`/`MenuEntry`),
     sem conhecer IPC nem domínio; WAI-ARIA `menubar` completo (Alt/F10, setas, submenus, Esc).
   - `app/menuModel.buildMenus(ctx)` — **função pura** do estado da sessão para o modelo do menu.
   Menu e teclado executam o **mesmo registro de comandos** do `Workspace` (um id por ação);
   os atalhos exibidos vêm do registro editável de atalhos. Assim nenhuma ação existe em dois lugares.
2. **Central de ajuda (F1)** com conteúdo estático em `helpTopics.ts` e busca local por palavras
   sem acentos (título > palavras-chave > texto; palavra inteira > prefixo). O tópico de atalhos é
   gerado do registro. Sem rede, coerente com a política offline (PRIVACY.md).
3. **Tutorial pulável** de 7 passos que destaca áreas reais por seletores `data-tour`, com preparo
   (trazer painel à frente). Estado em zustand; "não mostrar ao abrir" em `localStorage`. Não abre
   sozinho em automação (`navigator.webdriver`, modo de teste).
4. **Tela de carregamento** inline em `index.html` (aparece antes do JS), removida após o primeiro
   render com duração mínima de 700 ms e fade; respeita `prefers-reduced-motion`.
5. **Instaladores com a marca**: imagens próprias para NSIS (sidebar/header), WiX (banner/diálogo)
   e fundo do DMG em `crates/app-desktop/installer/`; NSIS e WiX em pt-BR + inglês; janela com
   `backgroundColor` da paleta para não piscar branco ao abrir.

## Alternativas

| Opção | Por que não |
|---|---|
| Menu nativo do Tauri (`Menu` do SO) | Visual e atalhos diferentes por SO, sem temas; macOS exige barra global; não testável com Vitest/Playwright |
| Biblioteca de menus (Radix etc.) | Dependência a mais para um componente pequeno; o padrão ARIA é simples de cumprir |
| Ajuda como página web/manual externo | Exige rede ou navegador; quebra o uso offline |
| Tour por biblioteca (Shepherd, Driver.js) | Dependência e CSS próprios para 7 passos; difícil integrar ao dockview |

## Consequências

- Novas ações entram em um único lugar (registro de comandos) e aparecem no menu e no teclado.
- Remoção do `PanelsMenu` (substituído por *Janela* e *Exibir › Painéis*).
- O conteúdo da ajuda precisa acompanhar o manual (checar na varredura).
- Imagens dos instaladores devem ser regeradas se a marca mudar (BMP 24 bits nos tamanhos do NSIS/WiX).
