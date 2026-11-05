# ADR-0012: Exclusões: Spotify e drivers próprios

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Solicitado estudo de integração com Spotify e criação de drivers.

## Decisão
**Spotify descartado:** termos de uso proíbem captura/download de áudio e o endpoint de audio features foi restrito para apps novos. **Drivers próprios descartados:** inviáveis em 3 SOs; substituídos por APIs nativas de baixa latência (ADR-0004).

## Consequências
- Escopo menor e juridicamente seguro.

## Alternativas consideradas
- Integração apenas de metadados (descartada pelo usuário).
