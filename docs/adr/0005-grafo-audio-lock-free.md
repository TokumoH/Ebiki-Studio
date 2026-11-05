# ADR-0005: Grafo de áudio compilado com troca lock-free

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
A thread de áudio não pode alocar, bloquear nem fazer I/O, mas a mixagem muda durante o uso.

## Decisão
A thread de controle compila o modelo de mixagem do domínio num `RenderGraph` plano (ordem topológica) e o envia por fila SPSC lock-free. A audio thread troca o grafo atomicamente e devolve o antigo para desalocação fora do tempo real. Parâmetros via atômicos com suavização. Disco via ring buffers e threads dedicadas.

## Consequências
- Áudio sem estalos durante edições.
- Complexidade maior na engine; exige testes de não-alocação (`assert_no_alloc`) e modo offline.

## Alternativas consideradas
- Mutex compartilhado (risco de inversão de prioridade e xruns).
