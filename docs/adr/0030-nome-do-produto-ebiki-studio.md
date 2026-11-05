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
