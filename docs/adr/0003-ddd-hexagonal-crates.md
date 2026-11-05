# ADR-0003: DDD + Hexagonal + um crate por Bounded Context

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Requisito CORE: DDD, GRASP Pure Fabrication e arquitetura de componentes, com independência entre módulos.

## Decisão
Camadas Interface → Application → Domain ← Infrastructure. Cada Bounded Context é um crate Rust (domain + application). Infraestrutura técnica em crates Pure Fabrication (`audio-engine`, `infra-persistence`, `infra-codecs`). Composition root em `app-desktop`. Contextos se comunicam apenas por eventos de domínio e IDs.

## Consequências
- Independência garantida pelo compilador.
- Mais crates e mais cerimônia de wiring.

## Alternativas consideradas
- Crate único com módulos (independência só por convenção).
