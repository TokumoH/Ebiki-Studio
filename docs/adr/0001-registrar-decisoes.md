# ADR-0001: Registrar decisões de arquitetura com ADRs

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
O projeto é extenso (10 fases) e as decisões precisam ser rastreáveis entre sessões.

## Decisão
Toda decisão arquitetural relevante será registrada em `docs/adr/NNNN-slug.md` (Contexto, Decisão, Consequências, Alternativas).

## Consequências
- Histórico auditável das decisões.
- Uma ADR substituída é marcada como *Substituída por ADR-XXXX*, nunca apagada.

## Alternativas consideradas
- Registrar apenas no CLAUDE.md (perde o raciocínio).
