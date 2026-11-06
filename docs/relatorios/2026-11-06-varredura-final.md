# Relatório final — varredura e entrega do Ebiki Studio

- **Data:** 2026-11-06 · **Branch:** `branchTrabalho`
- **Escopo:** varredura completa (`docs/processo/varredura.md`, 15 itens) após a entrega das 10 fases,
  identidade visual e correções.

## 1. Estado do produto

| Fase | Situação |
|---|---|
| 0 Fundação | Projeto `.ebiki` (pasta + SQLite), autosave, desfazer ilimitado, mapa de tempo, timeline |
| 1 Captura | Mono/estéreo, multi-entrada, punch, takes, metrônomo, dispositivo/buffer |
| 2 Mixagem | EQ, compressor, reverb, delay, limiter, gate, saturação, chorus; buses/sends; LUFS/correlação |
| 3 Edição | **Entregue nesta rodada** — dividir, aparar, fades/crossfades, normalizar, polaridade, reverso, ganho, espectrograma |
| 4 Bases | CC0, geradas, do usuário; metadados; decisão BPM/tom |
| 5 Harmonia | Campos e modos, progressões por gênero, próximo acorde, ouvir, arrastar |
| 6 Sincronização | Letra por sílaba, LRC/SRT, time-stretch |
| 7 Notação | Partitura + tablatura, editor, MusicXML/MXL/MIDI, playback GM/SF2 |
| 8 Afinação | Afinador, correção em tempo real e offline, edição nota a nota |
| 9 Formatos | Importa WAV/AIFF/FLAC/MP3/OGG/(M4A), MIDI, MusicXML/MXL, Guitar Pro; exporta áudio, stems, MIDI, MusicXML, PDF (impressão da partitura) |

**Testes:** 630 Rust · 141 unitários de UI · 24 E2E (Playwright, 3 execuções seguidas estáveis) ·
clippy `-D warnings` limpo · `cargo fmt` ok · contrato IPC (fixtures Rust → zod) sem divergência ·
smoke test do app desktop sob Xvfb ok (`img/2026-11-06-smoke-desktop.png`).

## 2. Decisões tomadas

| Decisão | Registro |
|---|---|
| Nome "Ebiki Studio", tema escuro Odoriko e claro Agridoce | ADR-0030 |
| Logo do autor preservada: só bordas suavizadas (conjuntos de nível do alfa); cor neutra (papel no escuro, sumi no claro) — escolha do autor | `assets/brand/README.md` |
| Edição de clipes não destrutiva (janela + processamento na engine); espectrograma como fabricação pura | ADR-0031 |
| Compensação de latência por leitura antecipada dos clipes | ADR-0032 |
| Coleta de mídia órfã **ao abrir** o projeto (histórico vazio), respeitando autosaves | spec Fase 1 §5 |
| Renders longos (análise de mix, espectrograma) fora do lock do estúdio | código + teste IPC |

## 3. Erros encontrados, explicados e corrigidos

### Principais

| # | Erro | Causa | Correção | Prova |
|---|---|---|---|---|
| 1 | **Fase 3 inexistente** (requisitos RF-3.1…3.5 sem código) | Fase não implementada na primeira passagem | Implementada de ponta a ponta (domínio → engine → SQLite → IPC → UI → docs) | testes de domínio, render por amostra, E2E dividir+fade+desfazer |
| 2 | **Take perdido** ao trocar dispositivo/projeto durante a gravação | `stop_recording` com engine nova sem início registrado apagava os WAVs | Gravação é finalizada antes da troca | teste app-core (falha sem a correção) |
| 3 | **Threads e arquivos parciais** após falha ao iniciar gravação | `RecordingSession` sem `Drop` | `Drop` encerra threads e apaga parciais; `finish` por faixa | 2 testes de engine |
| 4 | **Clique do metrônomo perdido** na borda do bloco | pertença decidida pelo tick, quadro arredondado caía no bloco seguinte | pertença decidida pelo quadro | teste falha no código antigo (bloco 571) |
| 5 | **L/R trocados** permanentemente após transbordo na entrada | escrita parcial no meio de um quadro | só quadros inteiros na fila | — (callback do dispositivo) |
| 6 | **Faixas com limiter/afinação atrasadas** | `latency_samples()` nunca lido | leitura antecipada (ADR-0032) | teste falha sem a correção |
| 7 | **Disco crescendo sem limite** | renders, takes e importações nunca coletados | coleta ao abrir; lote de importação com falha apaga só as cópias novas | 2 testes |
| 8 | **Fração de partitura perdia 50%** perto do limite i64 | redução por divisões sucessivas por 2 | melhor aproximação com denominador que cabe | proptest (erro relativo < 1e-9) |
| 9 | **UI congelada** durante a análise de mixagem | renderização de todas as faixas segurando o mutex | `RenderSnapshot` + `spawn_blocking` | teste IPC |

### Encontrados pela CI nos 3 sistemas (após a varredura local)

- **Rampa de ganho recuava no fim** (Linux, proptest): o erro acumulado de ponto flutuante passava
  do alvo e o último passo voltava — clique inaudível mas não monotônico. Passo limitado ao alvo;
  regressão com os bits exatos do caso (falha no código antigo).
- **Testes não linkavam no macOS**: `generate_context!()` expandido duas vezes duplicava
  `_EMBED_INFO_PLIST`; uma única `app_context()` serve app e testes.
- **Teste de gravação instável** (macOS lento): `sleep` fixo trocado por espera pela posição real
  do transporte.

### Demais (resumo)

- Exportações só gravam arquivos com a extensão do formato (defesa contra webview comprometida).
- Bases do usuário: limite de 2 GB, assinatura conferida, cópia atômica; arquivos sem base coletados.
- Fila de lixo da thread de áudio: transbordo pré-alocado em vez de vazamento.
- Importação usa a duração decodificada (MP3 com cabeçalho incorreto não some mais).
- `discard_recovery` remove `-wal/-shm`.
- UI: painel fechado não reabria (menu "Painéis"); "Salvar e fechar" fechava mesmo com erro; loop fixo
  em 16 tempos (agora 4 compassos pela fórmula); Enter/Espaço globais bloqueavam botões pelo teclado;
  Esc do menu dependia do foco; rótulos em inglês.
- Recursos sem UI ganharam controles: renomear/remover bus, reordenar faixas, renomear projeto, ganho de clipe.
- Testes fracos substituídos (12): asserções exatas no contrato IPC, MusicXML malformado, afinador,
  campo harmônico, mix quente, roundtrip com clipes em faixas distintas, overflow derivado do catálogo.
- Docs: arquitetura com os 19 crates e grafo real; spec da Fase 1; manual (edição, PDC, limpeza);
  referências quebradas; `rust-version` alinhado; índice de ADRs com números não usados.

## 4. Itens da varredura

| # | Item | Resultado |
|---|---|---|
| 1 | Arquitetura (DDD, PF, componentes) | `ctx-*` só dependem do kernel; portas no app-core; camadas de UI testadas |
| 2 | Reuso | duplicações removidas (`ipc_args`, `EventTable`, catálogo de painéis) |
| 3 | UX/overflow | E2E de overflow em 11 painéis × 2 resoluções (validado por mutação) |
| 4 | SQL injection | só nomes de tabela constantes em `format!`; nomes maliciosos em teste de roundtrip |
| 5 | Disco/render | coleta de órfãos, bases, sidecars, renders fora do lock |
| 6 | Desempenho/alocação | `assert_no_alloc` em engine, DSP, pitch e edição; PDC sem alocação |
| 7 | Funcionalidade | smoke desktop + E2E + cadeia completa |
| 8 | Cálculos | dB, LUFS/true peak, tempo variável, Hz↔MIDI revisados; Frac corrigido; metrônomo corrigido |
| 9 | API/dados | NaN/inf, ids, caminhos absolutos, extensões de saída |
| 10 | Ambiente/caminhos | CSP estrita, capacidades mínimas, `safe_join`, sem traversal |
| 11 | Páginas/links | todo comando IPC tem chamada na UI ou é utilitário; painéis no catálogo; links de docs válidos |
| 12 | Bibliotecas | `npm audit` 0; `cargo audit` sem vulnerabilidades diretas (2 avisos transitivos de Tauri/GTK) |
| 13 | Docs | atualizados (ver §3) |
| 14 | Testes | 12 fracos substituídos; regressões verificadas por mutação |
| 15 | Correções | todas acima |

## 5. Limitações conhecidas (documentadas no manual §12)

- Efeitos com latência em **buses** não são compensados; monitoração ao vivo soma a latência do efeito.
- Áudio decodificado em memória (≈ 1 h estéreo por 8 GB).
- AAC indisponível na exportação; sem VST3/CLAP e automação (arquitetura preparada).
- Logo: a fonte é um PNG de 98×210 px; uma exportação vetorial do autor daria ícones mais nítidos.
