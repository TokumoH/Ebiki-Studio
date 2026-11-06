# ADR-0035: "Ouvir entrada" somado fora do master, com estado de dispositivo na aplicação

- **Status:** Aceita
- **Data:** 2026-11-06
- **Contexto delimitado:** `audio-engine` (Pure Fabrication), `app-core` (aplicação), UI `recording`

## Contexto

O usuário pediu a escolha da entrada no próprio painel Gravação e a possibilidade de **monitorar o
dispositivo** — ouvir o microfone sem precisar criar/armar uma faixa (testar sinal, ajustar ganho).
Já existia monitoração **por faixa** (desligada / quando armada / sempre), que passa pela mixagem.

## Decisão

1. **Engine:** mensagem `SetDeviceMonitor(Option<DeviceMonitor>)` com rota (canal mono ou par) e
   ganho linear. O sinal é somado **na saída do dispositivo, depois do master**: não aparece no
   medidor do master, na mixagem, na gravação nem na exportação (o renderizador offline não tem entrada).
   Ganho em rampa por amostra (~5 ms) para ligar/desligar/mudar sem estalo; rota inválida = silêncio.
2. **Medidores de entrada:** pico por canal (até 8) calculado no callback e publicado por atômicos na
   telemetria (`input_peaks`), lido e zerado pela thread de controle como os demais medidores.
   Sem alocação nem bloqueio (teste `no_alloc` cobre monitor + medidores).
3. **Aplicação:** `device_monitor` é **estado do dispositivo**, não do projeto (não vai ao banco):
   começa desligado a cada abertura (evita microfonia), é validado contra os canais do dispositivo e
   reaplicado ao trocar a engine — ou desligado se a rota deixou de existir.
4. **UI:** estado do dispositivo num store único (`core/audio`), usado pela barra de status, pelo
   diálogo de Configurações de áudio e pelo painel Gravação; trocar a entrada no painel mantém saída
   e buffer atuais.

## Alternativas

| Opção | Por que não |
|---|---|
| Faixa oculta de "monitor" | Entraria na mixagem/exportação e no histórico do projeto |
| Monitorar pelo master (antes dos efeitos de master) | Apareceria no medidor e na exportação; efeitos do master atrasariam o sinal |
| Persistir no projeto | Projeto pode abrir em outro computador/dispositivo; risco de microfonia ao abrir |

## Consequências

- O monitor soma latência de ida e volta do buffer; para latência mínima, buffer 64–128 ou o
  monitoramento direto da própria interface.
- `MetersView.inputs` entra no contrato IPC (fixture `meters.json` atualizada).
