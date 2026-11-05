# ADR-0030: Nome do produto — Ebiki Studio

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
O projeto nasceu com o nome provisório "AppMusica". O usuário escolheu o nome definitivo
**Ebiki Studio** (inspiração no rock japonês).

## Decisão
- Nome exibido: **Ebiki Studio** (janela, instalador, tela inicial, documentação).
- Identificador do pacote: `com.ebikistudio.app`; binário: `ebiki-studio`.
- Extensão da pasta de projeto: **`.ebiki`** (antes `.amproj`); projetos `.amproj` continuam abrindo
  (a abertura aceita qualquer pasta que contenha `project.db`).
- Nomes internos (repositório `AppMusica`, crates `ctx-*`, `app-core`…) não mudam: são identificadores
  técnicos e renomeá-los só geraria ruído no histórico.

## Consequências
- Troca feita antes da primeira release, sem impacto para usuários.
- Preferências antigas no navegador interno (`appmusica.*`) são ignoradas e recriadas.

## Adendo: identidade de cor
- **Tema escuro (padrão): "Odoriko"**: noite índigo-violeta (`#14121d`), texto `#eee7f2`, destaque rosa
  de lanterna `#f4a3b6`, clipes azul-elétrico (`#2f3466`/`#b9c4ff`).
- **Tema claro: "Agridoce — manhã"**: papel quente (`#f3efe9`), tinta azul (`#1d2430`), destaque damasco
  `#ab4e25`.
- Elemento de marca: gradiente do horizonte (`--horizon`, azul do rio → lilás → rosa → damasco).
- Contrastes WCAG ≥ 4,5:1 para todo texto; prancha em `docs/diagramas/paletas-ebiki.html`.
