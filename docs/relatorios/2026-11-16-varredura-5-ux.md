# Varredura 5 — UX (2026-11-16)

| Campo | Valor |
|---|---|
| Id | REL-2026-11-16-varredura-5-ux |
| Título | Varredura priorizando UX + diagnóstico de "áudio baixo" e "a vista não segue a reprodução" |
| Data | 2026-11-16 |
| Usuário responsável | TokumoH ("corrija bugs… áudio baixo… configs de microfone e fone… a linha sai da tela… faça uma varredura priorizando o UX") |
| Versão analisada | v0.3.0 (`f4681e3`) |
| Status | **implementado** (S1, S2 e S3 aprovados em 2026-11-16; ver seção 5) |
| Mockup | Mockup 11 — configurações de áudio (microfone × fone) e "Seguir reprodução" |

## 1. Pedidos do usuário — diagnóstico

### 1.1 "O áudio está baixo"

**Dupla verificação:**

1. **Código:** caminho clipe → faixa (balanço com ganho unitário no centro, `audio-engine/src/graph.rs`) →
   master → saída do dispositivo (`core.rs`: L/R copiados sem atenuação; só saída mono faz a média). Fader padrão
   0 dB (`ctx-mixing/src/domain.rs`), nenhum limitador no master por padrão. Gravação em float 32 (`recorder.rs`),
   sem escala errada de inteiros.
2. **Medição de ponta a ponta (outra abordagem):** tom de amplitude 0,5 importado e exportado sem normalizar →
   pico de saída **0,5000 (−6,02 dB) nos dois canais**, idêntico à entrada.

**Conclusão:** a reprodução **não perde volume**. As causas prováveis estão antes da mixagem, e o app não ajuda a
notar nem a corrigir:

| Causa provável | Evidência |
|---|---|
| Sinal do microfone chega baixo e não há **ganho de entrada** | Nenhum controle de ganho na entrada; microfones de notebook/USB costumam ficar entre −40 e −25 dBFS |
| Medidores sem **faixa-alvo**: o usuário não sabe que está baixo | `inputRoutes.ts` mostra −60…0 dB sem indicação de nível bom |
| **"Ouvir entrada" começa em −6 dB** | `InputSection.tsx:13` (`DEFAULT_GAIN_DB = -6`) |
| Volume do sistema/fone baixo e **sem teste de saída** no app | O diálogo de áudio não tem "Testar som" |

### 1.2 "A linha de reprodução sai da tela e a vista não acompanha"

`Timeline.tsx` desenha o cursor (`Playhead`), mas **não existe código que role a vista**. Não há ADR, especificação
nem commit dizendo que isso foi proposital (busca por "seguir/acompanhar/follow/auto-scroll" no código e em `docs/`).
É uma funcionalidade que falta (Reaper, Audacity e Ableton seguem por padrão).

### 1.3 "Deixar claro o que é microfone e o que é fone"

`AudioSettingsDialog.tsx` usa "Saída", "Entrada (gravação)" e "Driver", sem dizer o que é fone/alto-falante ou
microfone/interface. O dispositivo de entrada é escolhido em **dois lugares** (diálogo e painel Gravação), e a latência
é calculada sempre como 48 kHz (`AudioSettingsDialog.tsx:46`).

## 2. Sugestões (formato §5)

### S1 — Configurações de áudio em "Fone / alto-falantes" × "Microfone / interface" (mockup 11, artboard 1)

- **O que:** o diálogo não diz qual escolha é do fone e qual é do microfone.
- **Solução:** dois blocos com ícone e subtítulo ("o que você ouve" / "o que você grava"):
  - **Fone:** dispositivo + **Testar som** (tom curto a −12 dBFS com medidor).
  - **Microfone:** dispositivo + medidor por canal com **faixa verde −18 a −6 dB** + **Ganho de entrada 0 a +24 dB** +
    "Ouvir o microfone no fone" (0 dB por padrão).
  - **Desempenho** (buffer) recolhido.

  Os mesmos nomes valem no app inteiro. O dispositivo do microfone passa a ser escolhido num lugar só (o diálogo); o painel
  Gravação mostra o microfone e tem um atalho para abrir o diálogo. A latência é calculada pela taxa real.
- **Por que:** resolve a dúvida do usuário e dá ferramentas para achar e corrigir o volume baixo: testar a saída, ver o
  nível, subir o ganho.
- **Por que não outras:**

  | Opção | Motivo de descarte |
  |---|---|
  | Só trocar os rótulos | Não resolve o volume baixo nem o teste de saída |
  | Controlar o volume do sistema operacional | Fora do escopo (sem drivers próprios); varia por sistema e exige permissões |
  | Normalizar toda gravação automaticamente | Muda o áudio sem o usuário pedir; "Normalizar" já existe no clipe |
  | AGC (ganho automático) | Bombeia o som e piora a gravação musical |

- **Onde:**
  - `ui/src/app/AudioSettingsDialog.tsx`, `ui/src/contexts/recording/InputSection.tsx`, glossário.
  - Ganho de entrada no motor: `audio-engine/src/core.rs`, aplicado antes do tap, dos medidores e do monitor, sem alocação
    e com rampa.
  - Configuração do dispositivo persistida junto com a escolha do dispositivo; IPC em `app-desktop`.
  - "Testar som": tom gerado no motor, fora do master.
- **Exemplo:** microfone a −35 dBFS → medidor laranja e "Sinal baixo: suba o ganho" → com +18 dB fica em −17 dB, dentro da faixa verde.
- **Riscos:** o ganho digital sobe também o ruído (é dito na legenda). Testes no motor: ganho aplicado, rampa, sem alocação.

### S2 — Seguir reprodução (mockup 11, artboard 2)

- **O que:** a vista não acompanha a linha de reprodução.
- **Solução:** modo **"vira a página"**:
  - Quando a linha passa de 90% da largura visível, a vista pula e a deixa em 10%.
  - Ao dar ▶ com a linha fora da tela, a vista vai até ela.
  - No fim do loop, a vista volta ao início dele.
  - Rolar à mão durante a reprodução pausa o seguir até a linha reaparecer ou até o próximo ▶.
  - Botão **Seguir** na barra de transporte, ligado por padrão, mais **Exibir › Seguir reprodução** e a tecla **F**.
- **Por que:** é o padrão dos DAWs consolidados. Virar a página (em vez de rolar suave) evita redesenhar a vista a cada
  quadro, o que mantém a otimização de renders da varredura 4.
- **Por que não outras:**

  | Opção | Motivo de descarte |
  |---|---|
  | Rolagem contínua (linha fixa no centro) | Redesenha a vista inteira ~30×/s; pode virar opção depois |
  | Sempre seguir, sem botão | Atrapalha quem edita outra parte durante a reprodução |

- **Onde:** função pura `followScroll()` em `ui/src/contexts/project/timelineView.ts` (com testes);
  `Timeline.tsx` assina a posição com `useLive.subscribe`, sem re-render; estado `follow` no `core/` (persistido);
  `TransportBar`, menu Exibir, `shortcuts.ts`, glossário, ajuda e manual.
- **Exemplo:** zoom de 60 px por tempo, vista de 1200 px. A linha chega a 1080 px (90%) e a vista avança 960 px: a linha
  fica a 120 px (10%).

### S3 — Correções da varredura UX (lista completa no Anexo)

Prioridade sugerida:

| Ordem | Itens | Por quê |
|---|---|---|
| 1 | **UX-01** (Espaço reativa o último botão clicado em vez de tocar) | Quebra o atalho mais usado; P |
| 2 | **UX-02** (barra superior quebra em janela estreita / zoom 200%), **UX-03** (dock fixo em 330 px), **UX-07** (rodapé dos diálogos rola junto) | Telas inutilizáveis em zoom alto/janela pequena |
| 3 | **UX-09** (gravar sem faixa armada só dá erro), **UX-12** (metrônomo mostra "Ligado" desligado), **UX-05** (sem indicação de ocupado) | Primeiro uso e feedback |
| 4 | **UX-06** (foco escapa dos diálogos), **UX-10** (contraste do tema claro), **UX-11** (teclas do tutorial), **UX-21** (erros anunciados como alerta) | Acessibilidade |
| 5 | **UX-04**, **UX-08**, **UX-13…UX-20**, **UX-22**, **UX-23** | Conforto e consistência |

## 3. Verificação desta varredura

- **Verificação 1:** navegação real no modo simulado com Playwright/Chromium em 1280×720, 1024×640, 800×600,
  853×480 (zoom 150%) e 640×360 (zoom 200%), temas claro e escuro, mais leitura de código.
- **Verificação 2 (outra abordagem):** conferência pontual no código dos achados de severidade alta e de uma amostra dos
  demais:
  - UX-01: `shortcuts.ts` `isActivationKey` sempre cede Espaço/Enter a `BUTTON`;
  - UX-03: `dockControl.ts:88` `height: 330`;
  - UX-11: `Tutorial.tsx` só trata as setas e Esc;
  - UX-12: `RecordingPanel.tsx` com o rótulo fixo "Ligado";
  - monitor: `InputSection.tsx:13` em −6 dB.

  Todos confirmados.
- **Áudio:** medição de ponta a ponta (seção 1.1).
- **Não verificado:** dispositivos reais (sem hardware de áudio no ambiente), leitor de tela real, macOS/Windows.

## 4. Riscos e pendências

- O diagnóstico do volume baixo depende do uso real. Perguntamos ao usuário se o baixo é a gravação, tudo o que toca ou o
  retorno do microfone. S1 cobre os três casos.
- Itens "a confirmar" do anexo (Afinação sem dispositivo, mensagens do sistema em inglês, seletor de host) ficam para a
  implementação.

---

## 5. Fechamento (2026-11-17)

Todas as sugestões foram implementadas, com dupla verificação e registro:

| Sugestão / itens | Registro |
|---|---|
| S1 — microfone × fone, ganho de entrada, Testar som | VER-2026-11-16-02 |
| S2 — seguir reprodução; UX-01 | VER-2026-11-16-03 |
| UX-02, UX-03, UX-07 | VER-2026-11-17-01 |
| UX-05, UX-09, UX-12, UX-17, UX-21 | VER-2026-11-17-02 |
| UX-06, UX-10, UX-11 | VER-2026-11-17-03 |
| UX-04, UX-08 | VER-2026-11-17-04 |
| UX-13, UX-14, UX-15, UX-16 | VER-2026-11-17-05 |
| UX-19, UX-20, UX-22, UX-23 | VER-2026-11-17-06 |
| Integração do S1, UX-18, corrida do seguir | VER-2026-11-17-07 |

**Fica para decisão do usuário:**

- "Mostrar na pasta" após exportar: exige ampliar a permissão do opener, hoje restrita à página de releases (ADR-0034).
- Progresso real por arquivo na importação: exige evento novo do núcleo.

**Não verificado neste ambiente:** som real (ganho, Testar som e latência) num Windows/macOS com placa de som.

**Estado final:**

- Rust: fmt/clippy limpos; 67 suítes, 666 testes;
- UI: Vitest 52 arquivos / 341 testes; Playwright 78/78.

## Anexo — achados detalhados da varredura UX


Data: 2026-11-16 · Método: leitura de código (ui/src) + navegação real com Playwright/Chromium no modo simulado
(`npm run dev:mock -- --port 5199`), deviceScaleFactor 1. Zoom do navegador simulado reduzindo o viewport
(150 % de 1280×720 ≈ 853×480; 200 % ≈ 640×360). Scripts e capturas ficaram no ambiente da sessão (não versionados).
Nada foi editado no repositório.


Esforço: P = pequeno (< 1 h), M = médio (meio dia), G = grande (> 1 dia).

---

### ALTA

### UX-01 — Espaço/Enter reativam o último botão clicado em vez de tocar/pausar
- **Onde:** `ui/src/app/shortcuts.ts:47-52` (`isActivationKey`) + `ui/src/app/Workspace.tsx:171-183`; todas as telas.
- **O que acontece:** depois de clicar com o mouse em qualquer botão (Metrônomo, M/S do mixer, Exportar, Importar, Loop…),
  ele mantém o foco; o atalho global ignora Espaço/Enter quando o foco está em um botão, então Espaço **aciona o botão de novo**.
  Medido: Metrônomo clicado (on) → Espaço → off e transporte parado; Mute voz (on) → Espaço → off; fechar Exportar com Esc →
  Espaço → o diálogo **reabre**. Em Reaper/Ableton/Logic, Espaço é sempre o transporte (botões não pegam foco no clique).
- **Evidência:** `s6.cjs` — saída `met after click true / met after space false transport stopped`, `mute after space false`,
  `dialog reopened by space? 1`.
- **Sugestão:** em `isActivationKey`, só ceder ao controle quando o foco veio do teclado:
  `return t.matches(":focus-visible") && (...)`. Alternativa complementar: `onMouseDown={e => e.preventDefault()}` nos botões
  de barra de ferramentas/transporte/mixer (não roubam foco). Cobrir com E2E “clicar Metrônomo, Espaço toca”.
- **Esforço:** P.

### UX-02 — Barra superior quebra em janelas estreitas / zoom 200 %
- **Onde:** `ui/src/app/Workspace.tsx:239-262` (`.topbar`), `ui/src/contexts/transport/transport.css`; tela principal.
- **O que acontece:** a 640 px de largura (1280 px com zoom 200 %, ou 800 px com 125 %+ do SO) os botões ▶ ⏹ ⏺ ficam
  espremidos (fatias de ~6 px), o grupo Loop **sobrepõe o botão Importar** (Playwright não consegue clicá-lo: “Loop … intercepts
  pointer events”), “Ajuda” do menu é cortado e o nome do projeto some. Não há quebra de linha, overflow nem menu “mais”.
- **Evidência:** `ws-zoom200-640x360.png`; erro de clique em `s2.cjs`.
- **Sugestão:** `flex-shrink:0` nos botões de transporte; esconder rótulos/ícones secundários por media query
  (Metrônomo → ícone, tempo/compasso compactos) e mover Importar/Exportar para um menu “⋯” abaixo de ~900 px;
  `overflow:hidden` + `min-width:0` nos grupos. Acrescentar 640×360 ao `e2e/overflow.spec.ts`.
- **Esforço:** M.

---

### MÉDIA

### UX-03 — Dock inferior com altura fixa de 330 px esmaga o Arranjo em telas baixas
- **Onde:** `ui/src/panels/dockControl.ts:88` (`setSize({ height: 330 })`).
- **O que acontece:** a altura do grupo de baixo não acompanha a janela. 1280×720: Arranjo ~250 px; 1024×640: 2 faixas mal
  cabem; 853×480 (zoom 150 %): o Arranjo mostra ~40 px (meia faixa) e o Mixer ocupa o resto.
- **Evidência:** `ws-1024x640.png`, `ws-800x600.png`, `ws-zoom150-853x480.png`.
- **Sugestão:** altura proporcional (`Math.min(330, Math.round(innerHeight*0.4))`) e mínimo para o grupo principal; ao
  redimensionar a janela, manter a proporção. Consolidadas (Reaper/Ableton) priorizam a área de arranjo.
- **Esforço:** P.

### UX-04 — Mute/Solo das faixas no Mixer ficam escondidos no tamanho padrão
- **Onde:** `ui/src/contexts/mixing/mixer.css:4` (`.strip { overflow-y:auto }`), `MixerPanel.tsx:143-150`.
- **O que acontece:** em 1280×720 os botões M/S das faixas ficam abaixo do fim do painel (bottom = 718 px; painel termina em
  ~696) e só aparecem rolando **cada canal** separadamente (rolagem sem barra visível). O Master mostra o M porque não tem pan/saída.
- **Evidência:** `mixer-1280.png` (canais voz/violão sem M/S), `s3.cjs` (`Mute voz@718`).
- **Sugestão:** layout do canal em coluna com o fader flexível (`flex:1; min-height:60px`) e M/S/valor fixos no rodapé do
  canal; ou M/S no topo, junto ao nome, como no Reaper.
- **Esforço:** M.

### UX-05 — Nenhum indicador visual de “ocupado” (importar, abrir, criar projeto)
- **Onde:** `ui/src/app/Workspace.tsx:235` só põe `aria-busy`; nenhum CSS usa `[aria-busy]`; `core/session.ts:37-49`.
- **O que acontece:** importações grandes, abrir/criar projeto e recuperar autosave não mostram spinner/progresso nem
  desabilitam botões; o usuário pode clicar de novo (dois seletores de arquivo) e não sabe se travou.
- **Evidência:** `grep aria-busy` em `*.css` = 0 resultados; `busy` só é consumido em Bases/Harmonia.
- **Sugestão:** barra de progresso indeterminada fina sob a appbar + `cursor: progress` quando `busy`; desabilitar
  Importar/Exportar/Salvar enquanto `busy`. Para importações longas, progresso por arquivo (evento do núcleo).
- **Esforço:** P (indicador) / M (progresso real).

### UX-06 — Diálogos não prendem o foco (Tab sai para a tela de trás)
- **Onde:** `ui/src/shared/components.tsx:56-97` (`Dialog`).
- **O que acontece:** com `aria-modal="true"`, Tab a partir de “Aplicar” vai para “Fechar aviso”, menu principal, renomear
  projeto… (atrás do fundo escuro). Atalhos ficam bloqueados, mas o foco some visualmente.
- **Evidência:** `s7.cjs` — sequência `IN:Cancelar | IN:Aplicar | OUT:Fechar aviso | OUT:menubar | OUT:Menu do Ebiki`.
- **Sugestão:** focus trap simples no `Dialog` (Tab/Shift+Tab circulam entre focáveis) ou `inert` no `#root` irmão enquanto
  aberto; botão ✕ no cabeçalho (convenção desktop) seria bem-vindo.
- **Esforço:** P.

### UX-07 — Rodapé do diálogo rola junto com o conteúdo (“Fechar” fora da tela)
- **Onde:** `ui/src/shared/ui.css:50-62` (`.dialog { overflow:auto }`, corpo sem rolagem própria).
- **O que acontece:** em Atalhos de teclado (20 linhas) o botão “Fechar” fica em y=745 com janela de 720/640 px; só
  rolando o diálogo inteiro. Afeta qualquer diálogo alto (Exportar com MP3, futuros).
- **Evidência:** `s8.cjs` — `{"dlg":704,"fechar":745}` (720) e `{"dlg":624,"fechar":745}` (640); `shortcuts.png`.
- **Sugestão:** `.dialog { overflow:hidden }` + `.dialog-body { overflow:auto; min-height:0 }` (cabeçalho e rodapé fixos).
- **Esforço:** P.

### UX-08 — Clicar na faixa/clipe não posiciona o cursor; só a régua
- **Onde:** `ui/src/contexts/project/Timeline.tsx:443-469` (lanes/clips só selecionam) vs `:361-380` (régua faz `seek`).
- **O que acontece:** clicar no vazio da faixa ou em um clipe mantém `001.1.000`; só a régua move o cursor. “S divide no
  cursor” e importação “no cursor” exigem ir à régua. Reaper/Audacity/Logic posicionam o cursor ao clicar na área de faixas.
- **Evidência:** `s12.cjs` — `after lane click 001.1.000`, `after clip click 001.1.000`, `after ruler click 003.3.000`.
- **Sugestão:** clique simples no vazio da faixa → `seek(snap(x))` (sem arrastar); em clipe, clique sem arrastar move
  cursor e seleciona. Opção nas preferências se houver conflito com seleção por retângulo futura.
- **Esforço:** M.

### UX-09 — Gravar sem faixa armada: só um toast de erro, sem caminho para resolver
- **Onde:** `Workspace.tsx:94-97` (ação `record`), mensagem do núcleo “Arme ao menos uma faixa com entrada definida para gravar.”
- **O que acontece:** R ou ⏺ num projeto novo mostra o erro (bom, PT-BR), mas não oferece ação; repetir empilha toasts
  idênticos. Entrada padrão é “Nenhuma” (tanto no diálogo quanto no painel Gravação), então o primeiro uso sempre cai aqui.
- **Evidência:** `s9.cjs` (dois toasts iguais); `record-noarm.png`.
- **Sugestão:** toast com botão “Abrir Gravação” (abre o painel e foca o seletor de entrada) ou, se houver 1 faixa
  selecionada e entrada configurada, armá-la automaticamente (Ableton/Logic). Deduplicar toasts com mesma mensagem.
- **Esforço:** P/M.

### UX-10 — Contraste insuficiente no tema claro (Agridoce)
- **Onde:** `ui/src/shared/tokens.css:69,77-78` (`--fg-faint #7d8796`, `--ok #2f9e6c`, `--warn #c27c16`).
- **O que acontece:** WCAG calculado: `--warn` sobre `--bg` 2,96:1 (barra de status “Sem dispositivo de áudio”, aviso de
  falhas de buffer); `--ok` 2,94:1; `--fg-faint` 3,17:1 sobre bg / 3,58:1 sobre panel (atalhos nos menus, itens desabilitados,
  rodapé da ajuda). Mínimo AA para texto pequeno = 4,5:1. Tema escuro passa (warn 10,0; faint 5,2).
- **Evidência:** `ws-1280-light.png` (status em laranja claro); cálculo em Python na sessão.
- **Sugestão:** escurecer no tema claro: `--warn ≈ #8f5a00`, `--ok ≈ #1f7a52`, `--fg-faint ≈ #5f6878`; manter cores vivas
  só para preenchimentos/ícones.
- **Esforço:** P.

### UX-11 — Tutorial: Enter não avança e os atalhos globais disparam por baixo
- **Onde:** `ui/src/app/Tutorial.tsx:2` (comentário promete “→ / Enter avançam”) e `:161-174` (só trata Esc/←/→);
  `Workspace.tsx:173` só bloqueia atalhos com `.dialog-backdrop`.
- **O que acontece:** no passo 1, Enter dispara “Parar e voltar ao início” e o passo continua 1; Espaço começa a tocar o
  projeto com o tour aberto (R gravaria, Delete apagaria clipe).
- **Evidência:** `s13.cjs` — `after Enter Passo 1 de 7`, `after Space: playing? 1`.
- **Sugestão:** tratar Enter como avançar e engolir as demais teclas enquanto o tour está aberto (ou marcar o tour para o
  filtro global, ex. checar `[data-testid=tour]`).
- **Esforço:** P.

---

### BAIXA

### UX-12 — Botão do metrônomo no painel Gravação diz “Ligado” mesmo desligado
- **Onde:** `ui/src/contexts/recording/RecordingPanel.tsx:100-102`. Ao lado, “Acentuar 1º tempo” aparece realçado (ligado),
  o que reforça a leitura errada. **Evidência:** `recording.png`.
- **Sugestão:** rótulo fixo “Metrônomo” com estado (`aria-pressed`) ou checkbox “Ligado”. **Esforço:** P.

### UX-13 — Editor de atalhos: sem detecção de conflito, sem restaurar padrão, rótulos inconsistentes
- **Onde:** `ui/src/app/ShortcutsDialog.tsx:19-31`, `shortcuts.ts:57-77`.
- **O que acontece:** dá para atribuir a mesma tecla a duas ações (vence a primeira da lista, em silêncio); não há
  “Restaurar padrões”; no macOS ⌘ é mostrado como “Ctrl” (`eventKeys` funde meta e ctrl); a tecla aparece como “Space”
  enquanto legendas/tutorial dizem “Espaço”. Faltam convenções: End (ir ao fim), Ctrl+N/Ctrl+O.
- **Sugestão:** checar conflito ao salvar (aviso “já usado por X — substituir?”), botão “Restaurar padrões”, exibir
  teclas localizadas (Espaço, ⌘ no mac), adicionar End/Ctrl+N/Ctrl+O. **Esforço:** M.

### UX-14 — Menu mostra “Delete” como atalho global, mas só funciona com foco no Arranjo
- **Onde:** `menuModel.ts:102,159` vs `Timeline.tsx:333` (handler local).
- **Evidência:** `s11.cjs` — clipe selecionado, clique no Mixer, Delete → clipe continua (`clips … 1`).
- **Sugestão:** registrar `deleteClip` em `DEFAULT_SHORTCUTS` (Delete/Backspace) para o atalho global. **Esforço:** P.

### UX-15 — Unidades e separador decimal inconsistentes
- **Onde:** `ui/src/shared/ui.css:32` (`.field > span { text-transform: uppercase }`) e formatações com `toFixed`.
- **O que acontece:** rótulos viram “BUFFER — ~10.7 MS DE IDA E VOLTA A 48 KHZ”, “LÁ4 (HZ)” (unidades SI erradas em
  maiúsculas). Decimal mistura vírgula (“44,1 kHz”, tela inicial) e ponto (“10.7 ms”, “♩ 120.0”, “0.0 dB”, “Qualidade Vorbis (0.6)”).
- **Sugestão:** tirar o uppercase dos rótulos (ou envolver unidades em `<span style="text-transform:none">`); formatar
  números com `toLocaleString("pt-BR")` num helper único em `core/format.ts`. **Esforço:** P/M.

### UX-16 — Textos do dockview em inglês (leitor de tela e botão fechar aba)
- **Onde:** `ui/src/panels/DockLayout.tsx:60` (sem opção `messages`/`announcements`).
- **Evidência:** `s15.cjs` — `aria-label="Close Mixer"`, região viva “Clipe opened”.
- **Sugestão:** passar `messages` do dockview com traduções PT-BR (“Fechar Mixer”, “Clipe aberto”). **Esforço:** P.

### UX-17 — Tela inicial: “Criar projeto” desabilitado sem explicar; ícone do tema quebrado
- **Onde:** `ui/src/app/StartScreen.tsx:96-98` (disabled sem `!dir` explicado), `:58` (“◐ Tema”).
- **O que acontece:** sem pasta escolhida o botão fica apagado sem dica; o glifo ◐ renderiza como meia-lua minúscula.
  Segmentados não ocupam a largura (espaço vazio à direita). **Evidência:** `01-start.png`.
- **Sugestão:** sugerir pasta padrão (Documentos/Ebiki) ou mensagem “Escolha uma pasta” sob o campo; usar ícone SVG.
  **Esforço:** P.

### UX-18 — Barra de status: status de áudio não é clicável e mostra dados contraditórios
- **Onde:** `Workspace.tsx:268-273`.
- **O que acontece:** “Sem dispositivo de áudio (modo silencioso)” não leva às configurações; buffer/latência aparecem
  mesmo no modo silencioso; só a saída é mostrada (nunca a entrada).
- **Sugestão:** transformar em botão que abre Configurações de áudio; mostrar “Saída: X · Entrada: Y”; ocultar latência
  sem dispositivo. **Esforço:** P.

### UX-19 — Exportar: sem intervalo (loop/seleção) e sem retorno de onde salvou
- **Onde:** `ui/src/app/ExportDialog.tsx:1` (comentário cita “intervalo”), `:34` (`exportMix(path, s, null)`), `:35`.
- **O que acontece:** sempre exporta o projeto inteiro; toast “Mixagem exportada.” sem caminho/“Abrir pasta”; só
  “Renderizando…” no botão como progresso. Toggles Normalizar/Stems parecem botões comuns.
- **Sugestão:** opção “Trecho do loop” quando houver loop; toast com caminho e ação “Mostrar na pasta” (plugin-opener já
  instalado). **Esforço:** M.

### UX-20 — BPM/compasso no transporte não são editáveis
- **Onde:** `TransportBar.tsx:57-62`. Duplo clique no ♩ 120.0 não faz nada (`s12.cjs`, `tempo edit input? 0`); só pelo painel
  Mapa de tempo. Reaper/Ableton/Logic editam no próprio display. **Sugestão:** duplo clique abre campo numérico que grava o
  tempo inicial (compasso 1). **Esforço:** M.

### UX-21 — Erros anunciados como `status` educado
- **Onde:** `ui/src/app/Toasts.tsx:7` (`role="status" aria-live="polite"` para todos). **Sugestão:** região separada
  `role="alert"` para `kind === "error"`. **Esforço:** P.

### UX-22 — Remover faixa (✕) colado nos botões ▲▼, sem toast de desfazer
- **Onde:** `Timeline.tsx:95`; na ordem de Tab vem logo após “Mover voz para baixo”. É desfazível (Ctrl+Z, `s11.cjs`), mas
  sem confirmação/aviso. **Sugestão:** toast “Faixa removida — Desfazer”; afastar ou mover para menu de contexto. **Esforço:** P.

### UX-23 — Nomenclatura (nit)
- “Arranjo” (aba) × “Linha do tempo” (aria-label do painel e texto do painel Clipe); “Mute/Solo/Pan” em inglês nos rótulos
  acessíveis enquanto o resto é PT-BR; tutorial diz “L faz loop de 4 compassos”, mas L religa o último trecho
  (`core/loop.ts:32-43`). **Esforço:** P.

---

### Observações sobre o diálogo “Configurações de áudio” (tema tratado separadamente)
`ui/src/app/AudioSettingsDialog.tsx` — captura `audio-settings.png`.
1. Rótulos “Saída” e “Entrada (gravação)” sem ícone/hint de “fones/caixas” × “microfone/interface”; “Driver: Simulado” é jargão.
2. Não há **teste de som** (tom de teste na saída) nem **medidor de entrada** no diálogo; os medidores ficam no painel Gravação.
3. Mesma configuração em dois lugares: dispositivo de entrada no diálogo e em Gravação › Entrada de áudio (`InputSection.tsx`),
   e a ajuda (“Primeiros passos”, passo 3) manda ao diálogo.
4. Latência mostrada é calculada fixando 48 kHz (`latency = buffer*2*1000/48000`, linha 46) mesmo em projetos 44,1/96 kHz;
   a barra de status usa o valor estimado do núcleo — podem divergir.
5. Rótulo em maiúsculas deforma unidades (“MS”, “KHZ”) — ver UX-15. Aplicar fecha o diálogo sem feedback do resultado além do toast.
6. Entrada padrão “Nenhuma”: no primeiro uso o usuário sempre cai no erro de UX-09.

### Adjacências aos problemas tratados à parte (não investigados a fundo)
- **Áudio baixo:** monitor de entrada inicia em −6 dB (`InputSection.tsx:13`, `DEFAULT_GAIN_DB`); faders de faixa/master em 0 dB
  sem indicação de headroom; nada mais conclusivo pela UI.
- **Seguir o cursor:** não há botão/opção “seguir cursor” no Arranjo nem no menu Exibir/Transporte; “Voltar ao início”
  (Home) faz `seek(0)` — *a confirmar* se a vista rola até o início quando o usuário está com zoom e rolado à direita.

### A confirmar (palpites não reproduzidos)
- Painel Afinação lista “Entrada 1/Entrada 2” e botão “Ligar” mesmo sem dispositivo de entrada configurado — testar o que
  acontece ao ligar sem entrada.
- Mensagens do núcleo são PT-BR, mas várias interpolam `{0}` de erros de E/S do SO (podem chegar em inglês).
- No Windows/macOS reais, falta seletor de host/driver (WASAPI/ASIO/CoreAudio) no diálogo — só “Driver: X” informativo.

### Pontos positivos verificados
Sem erros de console; Esc fecha diálogos e o popover de loop; foco visível na maioria dos controles; legendas (data-tip)
ricas; estados vazios claros (“Projeto vazio”, Partitura, Clipe, Afinação); recuperação de autosave; diálogo de fechar com
“Salvar e fechar”; overflow sem rolagem horizontal da página em 1280/1024/800.
