# ADR-0010: alphaTab para partitura e tablatura

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Requisitos: editor completo de partitura + tablatura, importação MusicXML e Guitar Pro, reprodução por soundfont.

## Decisão
Adotar **alphaTab** como motor de renderização/reprodução de notação na UI, encapsulado num adaptador do contexto Notation. O modelo de domínio `Score` permanece próprio (Rust); alphaTab é infraestrutura de apresentação.

## Consequências
- Cobre boa parte das Fases 7 e 9.
- Editor completo exigirá camada de edição própria sobre alphaTab.

## Alternativas consideradas
- VexFlow (sem tablatura completa nem import Guitar Pro).
- OSMD (sem tablatura completa).
