# ADR-0032: Compensação de latência de efeitos por leitura antecipada dos clipes

- **Status:** Aceita
- **Data:** 2026-11-06
- **Contexto delimitado:** `audio-engine` (Pure Fabrication)

## Contexto

Efeitos com look-ahead ou janela de análise (Limiter, correção de pitch PSOLA) declaram
`AudioProcessor::latency_samples()`, mas a engine ignorava o valor: uma faixa com esses inserts
soava atrasada em relação às demais, ao vivo e na exportação (achado da varredura de 2026-11-06).

## Decisão

Cada faixa lê os seus clipes **adiantada** pela soma das latências dos inserts ativos
(não em bypass). A saída, atrasada pelos efeitos, volta a cair na posição correta da timeline.

- Custo zero no tempo real: soma sobre os inserts por bloco, sem alocação nem linhas de atraso.
- Vale para reprodução e exportação (o mesmo núcleo renderiza offline).

## Alternativas

| Opção | Por que não |
|---|---|
| Atrasar todos os caminhos até a maior latência (PDC clássico) | Exige linhas de atraso por canal e aumenta a latência de monitoração (RNF-01) |
| Ignorar | Desalinha faixas com limiter/afinação em dezenas de ms |

## Consequências / limitações

- Entrada monitorada ao vivo não pode ser adiantada: a latência do efeito soma-se à de monitoração
  (padrão em DAWs; o usuário pode pôr o efeito em bypass ao gravar).
- Inserts de **buses** não são compensados (um envio e o caminho direto teriam latências diferentes);
  mantenha efeitos com latência nas faixas ou no master. Ligar/desligar bypass desloca a leitura
  instantaneamente (um pequeno salto no áudio durante a reprodução).
- Teste: `audio-engine/tests/engine.rs::insert_latency_is_compensated_on_playback` (validado por mutação).
