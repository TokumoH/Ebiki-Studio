# ADR-0011: Portas de extensão para IA, plugins e automação

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Sem IA na versão inicial, mas o sistema deve permitir IA, plugins VST3/CLAP e automação no futuro.

## Decisão
Definir desde já as traits `SuggestionProvider`, `MixAdvisor`, `EffectProcessor`, `ParameterSource`, `PitchDetector` e `AudioBackend`, com implementações iniciais baseadas em regras/nativas.

## Consequências
- Extensões futuras entram como novos adaptadores, sem alterar o domínio.

## Alternativas consideradas
- Adicionar abstrações só quando necessário (refatoração maior depois).
