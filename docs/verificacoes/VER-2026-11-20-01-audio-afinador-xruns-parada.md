# VER-2026-11-20-01 — Afinador após troca de microfone, falhas de buffer recentes e parada sem travar (S-4, S-5, S-6)

| Campo | Valor |
|---|---|
| Id | VER-2026-11-20-01 |
| Título | Afinador acompanha o microfone atual; barra de status só mostra falhas de buffer recentes; parar gravação com dispositivo morto não trava 1 s |
| Gravidade | baixa/média |
| Tipo | correção (continuação da varredura 6) |
| Data | 2026-11-20 |
| Usuário responsável | TokumoH ("continue", após a varredura 6) |
| Status | implementado |
| Commits | este commit |
| Arquivos | `ui/src/contexts/tuning/TuningPanel.tsx`, `ui/src/core/{xruns.ts,xruns.test.ts}` (novos), `ui/src/core/README.md`, `ui/src/app/Workspace.tsx`, `crates/app-core/src/studio.rs`, `crates/app-core/tests/studio.rs`, `ui/e2e/{audio-devices,music}.spec.ts`, `ui/e2e/README.md` |

## Problema

- **S-4 / AU-08:** o afinador lia os canais do microfone uma vez só, com mínimo de 1. Depois de trocar ou desconectar
  o microfone, oferecia canais inexistentes, e com o afinador ligado aparecia o erro "O afinador não está ligado".
- **S-5 / AU-11:** "N falha(s) de buffer — aumente o buffer" acumulava desde a abertura. Um soluço único pedia buffer
  maior para sempre.
- **S-6 / AU-12:** com o dispositivo morto no meio da gravação, `stop_recording` esperava 1 s com a trava do estúdio
  presa. Com a reabertura automática, a interface congelava 1 s.

## Era proposital?

Não. A leitura única do afinador é anterior ao estado de áudio compartilhado (`core/audio`). A espera de 1 s supunha um
núcleo sempre vivo.

## Solução

- **Afinador:**
  - usa `useAudio`;
  - sem microfone, mostra "Sem microfone" e não deixa ligar;
  - na troca, limita o canal e religa sozinho se o canal ainda existe; senão desliga e avisa.
- **Falhas de buffer:** `recentXruns` (função pura) conta só as falhas dos últimos 10 s; com o núcleo reaberto, a
  contagem recomeça.
- **Parada:** se a posição do núcleo não andar em ~100 ms, ele não está rodando, e a espera termina.

## Por que esta solução

- O estado compartilhado já é atualizado pela reabertura automática (VER-2026-11-17-11).
- A janela de tempo reflete o problema atual.
- Detectar o núcleo parado pela posição não depende de sinal novo do driver.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Contadores separados de xrun no núcleo | Muda telemetria e IPC; a janela resolve a mensagem enganosa (pode vir depois) |
| Passar "dispositivo perdido" até o `stop_recording` | Acopla a camada de app ao driver; a posição parada é sinal suficiente |
| Reenviar o rascunho do ganho de entrada depois de um pânico (AU-14) | Exige alocar fora do tempo real e trocar por mensagem; o pânico conhecido (AU-01) já foi eliminado. Fica como risco |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/contexts/tuning/TuningPanel.tsx](../../ui/src/contexts/tuning/TuningPanel.tsx) | `Tuner` | Estado compartilhado, religar e desligar |
| [ui/src/core/xruns.ts](../../ui/src/core/xruns.ts) | todo | Janela de 10 s |
| [ui/src/app/Workspace.tsx](../../ui/src/app/Workspace.tsx) | barra de status | Mensagem com a janela |
| [crates/app-core/src/studio.rs](../../crates/app-core/src/studio.rs) | `stop_recording` | Espera curta com núcleo parado |
| [crates/app-core/tests/studio.rs](../../crates/app-core/tests/studio.rs) | fim | Teste de tempo |

## Lógica

- **Antes:** o microfone some, o afinador continua com o canal antigo e dá erro; um soluço deixa o aviso eterno; parar
  com o dispositivo morto trava 1 s.
- **Depois:** o afinador se ajusta ou desliga com explicação; o aviso some após 10 s sem falhas; a parada termina em
  ~100 ms.

## Exemplo

Gravando, o microfone USB é puxado: o app reabre o áudio, a gravação para em cerca de 0,1 s (antes, 1,05 s), o take
aparece e o afinador ligado desliga com "Microfone desconectado: afinador desligado.".

## Verificação

- **TDD:**
  - `stopping_a_recording_after_the_device_died_does_not_block_for_a_second` falhou com 1,05 s antes da correção e passa
    em menos de 400 ms depois;
  - `xruns.test.ts`, 3 testes;
  - E2E do afinador com o microfone desconectado.
- **Testes ajustados:** os E2E de afinação escolhem o microfone simulado antes (o afinador agora exige um, como no app
  real).
- **Sabotagem:** sem a saída antecipada, o teste de tempo volta a 1,05 s e falha.
- **Verificação 1:** `cargo fmt --check`; clippy (`app-core`, `app-desktop`, `audio-engine`) sem avisos; `cargo test
  -p app-core` ok; `tsc`; Vitest 53 arquivos / 345 testes; Playwright 81/81.
- **Verificação 2 (outra abordagem):** sabotagem acima; `vite build`; `check_readmes.py`.

## Riscos

- AU-13, AU-14 e AU-15 (defesas latentes) continuam como risco documentado no relatório da varredura 6: hoje são
  inalcançáveis ou protegidos por validação.
- Um núcleo que roda mas com a posição parada (transporte pausado durante a gravação) terminaria a espera mais cedo:
  `engine.stop` é aplicado no próximo callback, e o estado é conferido antes.
