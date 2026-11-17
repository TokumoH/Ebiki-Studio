# VER-2026-11-17-07 — Integração do S1, barra de status (UX-18) e corrida do seguir reprodução

| Campo | Valor |
|---|---|
| Id | VER-2026-11-17-07 |
| Título | S1 sobre a `branchTrabalho`; barra de status com fone e microfone clicável; seguir não puxa a vista durante a rolagem manual |
| Gravidade | média (corrida do seguir) · baixa (UX-18, ajustes de integração) |
| Tipo | integração + correção |
| Data | 2026-11-17 |
| Usuário responsável | TokumoH (S1 e S2 aprovados; "corrija tudo do S3 na ordem") |
| Status | implementado |
| Commits | este commit (depois do commit do S1, VER-2026-11-16-02) |
| Arquivos | `ui/src/app/{Workspace.tsx,app.css,AudioSettingsDialog.tsx,AudioSettingsDialog.test.tsx}`, `ui/src/contexts/recording/InputSection.tsx`, `ui/src/contexts/project/Timeline.tsx`, `ui/src/core/glossary.ts`, `ui/e2e/{first-record,follow,audio-settings}.spec.ts`, `docs/manual/README.md`, `docs/verificacoes/README.md` |

## Problema

1. **Integração:** o S1 foi feito num espaço isolado a partir de `fab6d20`. Depois disso a `branchTrabalho` recebeu
   UX-13 a UX-23.
   - O rebase só conflitou no índice `docs/verificacoes/README.md`.
   - A verificação do conjunto achou dois ajustes:
     - o E2E do UX-09 escolhia o microfone no painel Gravação, e o S1 levou essa escolha para o diálogo;
     - as latências do S1 usavam ponto decimal (UX-15).
2. **Seguir reprodução:** sob carga, se a vista era rolada à mão e uma leitura da telemetria chegava antes do evento de
   rolagem, o seguir puxava a vista de volta. Isso apareceu como instabilidade do E2E com 4 a 6 execuções em paralelo.
3. **UX-18:**
   - "Sem dispositivo de áudio" na barra de status não levava às configurações;
   - buffer e latência apareciam até no modo silencioso;
   - o microfone não aparecia.

## Era proposital?

- O conflito de índice é da paralelização (a linha do S1 e as dos UX-13 a UX-23 entraram no mesmo ponto).
- A corrida do seguir não era proposital. O desenho (VER-2026-11-16-03) contava com o evento de rolagem para detectar a
  rolagem manual.
- UX-18 dependia do S1 (abrir o diálogo pelo `core/audio`), por isso ficou para depois da integração.

## Solução

- **Integração:**
  - índice com as duas linhas, em ordem de id;
  - o E2E do UX-09 escolhe o microfone em *Arquivo › Configurações de áudio*;
  - `formatDecimal` na latência do diálogo e do painel; testes com vírgula.
- **Seguir:**
  - `setScrollLeft` passa a guardar a posição aplicada em `lastLeft`;
  - a cada leitura da telemetria, se a vista está mais de 2 px longe dessa posição, houve rolagem manual ainda não
    notificada, e o seguir pausa em vez de rolar.
- **E2E do seguir:** espera o estado "Tocar" depois de pausar. O estado vem da telemetria (~33 ms), e dois Espaços colados
  pausariam duas vezes, o que uma pessoa não faz.
- **UX-18:**
  - a entrada de áudio da barra de status é um botão que abre as Configurações de áudio;
  - mostra "Fone: X · Microfone: Y" (ou "Sem dispositivo de áudio — configurar");
  - buffer e latência só aparecem com dispositivo;
  - kHz com vírgula.
- **Manual:** nota do armar automático (UX-09) na seção Gravação, que o S1 reescreveu.

## Por que esta solução

- Comparar com a última posição aplicada detecta a rolagem manual sem depender da ordem dos eventos.
- Rebase (e não merge) mantém o histórico linear, como nas varreduras anteriores.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Aumentar a espera no E2E | Esconderia a corrida real em vez de corrigir |
| Merge com commit de merge | Histórico da `branchTrabalho` sempre linear |
| Status com só o microfone | O usuário pediu clareza entre fone e microfone; os dois cabem na barra |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [ui/src/contexts/project/Timeline.tsx](../../ui/src/contexts/project/Timeline.tsx) | `setScrollLeft`, efeito do seguir | Rolagem manual pendente pausa o seguir |
| [ui/src/app/Workspace.tsx](../../ui/src/app/Workspace.tsx) | barra de status | Botão de áudio |
| [ui/src/app/AudioSettingsDialog.tsx](../../ui/src/app/AudioSettingsDialog.tsx), [InputSection.tsx](../../ui/src/contexts/recording/InputSection.tsx) | latência | Vírgula |
| [ui/e2e/first-record.spec.ts](../../ui/e2e/first-record.spec.ts) | UX-09 | Microfone pelo diálogo |
| [ui/e2e/follow.spec.ts](../../ui/e2e/follow.spec.ts) | rolagem manual | Espera de estado |
| [ui/e2e/audio-settings.spec.ts](../../ui/e2e/audio-settings.spec.ts) | fim | E2E do UX-18 |

## Lógica

- **Antes:** a telemetria chegava, o seguir via a vista em 6000 (rolada à mão, evento pendente) e rolava para a linha,
  desfazendo a rolagem.
- **Depois:** a telemetria chega, o seguir compara com a última posição que ele mesmo aplicou, vê a diferença, pausa e
  não mexe.

## Exemplo

Tocando com a linha em 2000 px: a pessoa arrasta a barra para 6000. Na próxima leitura, |6000 − 1945| > 2, então o
seguir pausa e a vista fica em 6000 até a linha voltar a aparecer ou até o próximo ▶.

## Verificação

- **Integração:** rebase com conflito resolvido no índice; `check_readmes.py` ok.
- **Estresse do seguir:**
  - antes: 1 a 4 falhas em 16 a 24 execuções paralelas;
  - depois: `--repeat-each 6 --workers 6` com 24/24.
- **Verificação 1:**
  - `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`: 67 suítes, 666 testes, 0 falhas
    (as mensagens de ALSA vêm da falta de placa de som no ambiente);
  - `tsc`; Vitest 52 arquivos / 341 testes; Playwright completo, 78/78.
- **Verificação 2 (outra abordagem):** `vite build`; estresse do seguir; E2E de UX-18 e de áudio isolados.

## Riscos

- Sem hardware de áudio no ambiente: ganho e Testar som do S1 foram verificados por simulação offline do mesmo núcleo e
  pelo simulado. Falta ouvir num Windows/macOS real.
- O processamento do callback em blocos (S1) é a parte mais sensível. Todos os testes do motor passam, incluindo o
  `no_alloc`.
