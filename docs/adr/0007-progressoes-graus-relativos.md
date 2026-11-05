# ADR-0007: Progressões harmônicas em graus relativos

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Requisito de normalização máxima. Armazenar progressões por tonalidade gera 12+ cópias por progressão.

## Decisão
Progressões são armazenadas como sequências de **graus relativos** (I, vi, IV, V) ligadas a um modo/escala; acordes do campo harmônico são **derivados** de intervalos da escala + grau. A tonalidade é aplicada na leitura (domínio).

## Consequências
- Sem redundância; transposição trivial.
- Cálculo em tempo de leitura (barato, feito em memória).

## Alternativas consideradas
- Tabela de acordes absolutos por tom (redundante, viola normalização).
