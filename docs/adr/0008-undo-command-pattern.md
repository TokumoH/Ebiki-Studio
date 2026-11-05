# ADR-0008: Undo/Redo com Command Pattern em memória

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Requisito: desfazer/refazer ilimitado durante a sessão.

## Decisão
Toda mutação passa pelo `CommandBus`; cada `Command` implementa `execute`/`undo`. Histórico em memória; gestos contínuos agrupados em comando composto. Ações sem mutação ficam fora do histórico.

## Consequências
- Um único ponto de mutação: consistência, eventos e auditoria.
- Todo caso de uso mutável precisa de inverso testado.

## Alternativas consideradas
- Snapshots completos do projeto (memória alta).
- Histórico persistido (não requisitado).
