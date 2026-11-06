# Varredura — Identidade Ebiki (menu fixo, ajuda, tutorial, splash, instaladores) — v0.2.0

Data: 2026-11-06 · Checklist: [`docs/processo/varredura.md`](../processo/varredura.md) ·
Decisões: [ADR-0033](../adr/0033-menu-fixo-ajuda-tutorial-e-identidade-dos-instaladores.md)

## Decisões tomadas

- Menu fixo **dirigido por dados**: `shared/MenuBar` (design system, sem domínio) + `app/menuModel`
  (função pura). Menu e teclado usam o **mesmo registro de comandos** — nenhuma ação duplicada.
- Ajuda e tutorial **offline**, com conteúdo estático e busca local sem acentos.
- Splash inline em `index.html` (aparece antes do JS, sem script inline: CSP `script-src 'self'` mantida).
- Instaladores NSIS/WiX/DMG com imagens próprias e idiomas pt-BR + inglês.
- Versão **0.2.0**.

## Commits principais

| Commit | Conteúdo |
|---|---|
| `e258b3e` | Imagens e configuração dos instaladores com a marca |
| `310f0ed` | Menu fixo, central de ajuda, tutorial, splash, Sobre, zoom/tela cheia/layout |
| `1c6ae31` | ADR-0033, manual e guia, CHANGELOG 0.2.0, versão 0.2.0 |
| (este) | Correções da varredura |

## Erros encontrados e corrigidos

**Destaques**

| Problema | Causa | Correção |
|---|---|---|
| Em janelas estreitas (≈ 800 px) o nome do projeto **cobria o menu Ajuda** | A barra de menus tinha `min-width: 0` e encolhia; os botões transbordavam sob o nome | A barra não encolhe no header; o nome é que trunca. Teste E2E que falha sem a correção |
| Busca da ajuda colocava "fader" à frente de "fade" | Pontuação por substring | Pontuação por palavra (inteira > prefixo; título > palavras-chave > texto) |
| Esc após abrir um menu com o mouse tirava o foco do botão | Esc tratado como "sair da barra" | Esc fecha o menu e mantém o foco; Alt dentro da barra sai dela |

**Demais**

- Indicador de submenu "▸" aparecia como ponto na fonte mono → "›" na fonte da interface.
- Ajuda citava "Janela › Painéis" (não existe; os painéis estão direto em *Janela* e em *Exibir › Painéis*).
- Manual e guia citavam os botões antigos ⚙ e ⌨ → caminhos do menu e novos atalhos.
- PRIVACY.md passa a dizer onde ficam as preferências de interface (armazenamento local da janela).
- CHANGELOG: links relativos quebravam na página da release → links absolutos.
- Toolchain Rust 1.97.0 local estava corrompida (sem `cargo`) → reinstalada; `Cargo.lock` atualizado para 0.2.0.

## Checklist

| # | Item | Resultado |
|---|---|---|
| 1 | Arquitetura | `architecture.test.ts` verde; MenuBar em `shared` sem IPC/domínio; modelo puro em `app` |
| 2 | Reutilização | Registro único de comandos; `Dialog` ganhou `size="lg"` em vez de diálogo novo |
| 3 | UX/UI | Testado em 1280, 1024, 800 e 640 px sem rolagem horizontal; ARIA menubar; contraste dos temas |
| 4 | SQL injection | Sem mudança no banco |
| 5 | Carregamento | Splash some mesmo se o render demorar (mínimo 700 ms, fade, remoção do DOM) |
| 6 | Desempenho | Nada novo no áudio; simulação de 32 faixas verde |
| 7 | Funcionalidade | E2E: splash, menu (tema, Alt), F1 + busca + "Mostrar no app", tutorial, Sobre, Janela, Ferramentas |
| 8 | Cálculo | Zoom com limites testados; posição do cartão do tutorial testada (`cardPosition`) |
| 9 | IPC | Comandos existentes reutilizados; nenhum comando novo |
| 10 | Segurança | CSP inalterada; sem script inline; sem rede na ajuda |
| 11 | Navegação | "Mostrar no app" e Ferramentas abrem/focam os painéis certos |
| 12 | Dependências | `npm audit --omit=dev`: 0 vulnerabilidades |
| 13 | Documentação | ADR-0033, manual, guia, CHANGELOG e PRIVACY atualizados |
| 14 | Testes | 164 unitários + 31 E2E; Rust: fmt, clippy, testes, desempenho e simulação (scripts/ci-local.sh rust) |
