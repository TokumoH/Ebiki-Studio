# contexts

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-contexts |
| Título | contexts |
| Tipo | UI por bounded context |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/contexts/README.md`) |
| Arquivos | 86 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Uma pasta por contexto de domínio (projeto/arranjo, mixagem, gravação, transporte, harmonia, biblioteca, notação, letra, afinação…), com painéis e regras de apresentação. Um contexto nunca importa outro (teste de arquitetura).

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Subpasta por contexto, com componentes, CSS e testes; o que é compartilhado entre contextos vai para `core/` ou `shared/`.

## Por que esta organização

Camadas com direção única de dependência (app → panels → contexts → core/ipc/shared) mantêm componentes independentes; o teste `architecture.test.ts` impede violações.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Organizar por tipo de arquivo (components/, hooks/…) | Espalharia cada contexto por várias pastas e esconderia as dependências |
| Tudo em `src/` | Sem fronteiras, o acoplamento cresce sem ser notado |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [harmony/](harmony/) | Subdiretório (coberto por este README) |
| [harmony/HarmonyPanel.test.tsx](harmony/HarmonyPanel.test.tsx) | Testes: HarmonyPanel; mostra o campo, alterna para graus/tétrades e troca o modo |
| [harmony/HarmonyPanel.tsx](harmony/HarmonyPanel.tsx) | Painel "Harmonia": campo harmônico, progressões por gênero, próximo acorde e progressões do usuário. */ |
| [harmony/NextChordHelper.tsx](harmony/NextChordHelper.tsx) | Sequência em construção (sobrevive à troca de abas do painel). */ |
| [harmony/ProgressionCard.tsx](harmony/ProgressionCard.tsx) | Cartão de progressão: nome, gêneros, selo universal, acordes no tom, ouvir e inserir. */ |
| [harmony/ProgressionList.tsx](harmony/ProgressionList.tsx) | Progressões ranqueadas para o tom e o gênero escolhidos (RF-5.3). */ |
| [harmony/UserProgressions.tsx](harmony/UserProgressions.tsx) | Progressões do usuário: cadastrar, editar, remover, com desfazer/refazer da biblioteca. */ |
| [harmony/actions.ts](harmony/actions.ts) | Ações do contexto Harmonia que tocam o núcleo (audição e inserção no arranjo). |
| [harmony/harmony.css](harmony/harmony.css) | Estilos |
| [harmony/sequence.test.ts](harmony/sequence.test.ts) | Testes: construtor de sequência; acrescenta sem mutar e remove o último |
| [harmony/sequence.ts](harmony/sequence.ts) | Regras puras do construtor de sequência (aba "Próximo acorde") e de notação. |
| [library/](library/) | Subdiretório (coberto por este README) |
| [library/AddUserBaseDialog.tsx](library/AddUserBaseDialog.tsx) | Adiciona um áudio do usuário como base (o arquivo é copiado para a biblioteca). */ |
| [library/BasesPanel.test.tsx](library/BasesPanel.test.tsx) | Testes: BasesPanel; busca por texto e tipo |
| [library/BasesPanel.tsx](library/BasesPanel.tsx) | Painel "Bases": busca, audição, geração, áudio do usuário, SoundFont e inserção com divergência. */ |
| [library/GenerateBaseDialog.tsx](library/GenerateBaseDialog.tsx) | Gera uma base (progressão + estilo + bateria + baixo), ouve e salva a receita na biblioteca. */ |
| [library/InsertBaseDialog.tsx](library/InsertBaseDialog.tsx) | posição do cursor no momento em que o diálogo abre |
| [library/SoundFontDialog.tsx](library/SoundFontDialog.tsx) | Escolha do SoundFont (.sf2) usado para ouvir e renderizar progressões e bases geradas. */ |
| [library/filters.ts](library/filters.ts) | Regras puras: filtros do formulário de busca → 'BaseQuery' do núcleo. |
| [library/insertFlow.ts](library/insertFlow.ts) | Regras puras do fluxo "inserir base" com divergência de BPM/tom (RF-4.5: sempre perguntar). |
| [library/labels.ts](library/labels.ts) | Rótulos de exibição do contexto Biblioteca (sem regra de negócio além da formatação). |
| [library/library.css](library/library.css) | Estilos |
| [library/library.test.ts](library/library.test.ts) | Testes: filtros → BaseQuery; vazio vira consulta vazia |
| [mixing/](mixing/) | Subdiretório (coberto por este README) |
| [mixing/AdvisorPanel.tsx](mixing/AdvisorPanel.tsx) | Auxílio de mixagem por regras: mede a mixagem offline e lista conselhos. */ |
| [mixing/EffectEditor.tsx](mixing/EffectEditor.tsx) | Editor genérico de parâmetros; cada arrasto de slider é um único desfazer (gesto). */ |
| [mixing/MixerPanel.test.tsx](mixing/MixerPanel.test.tsx) | Testes: MixerPanel — bus; renomeia o bus clicando no nome |
| [mixing/MixerPanel.tsx](mixing/MixerPanel.tsx) | Presente: o nome vira editável (clique/F2). */ |
| [mixing/effectsMeta.test.ts](mixing/effectsMeta.test.ts) | Testes: rótulos visíveis em PT-BR (Mistura/Saturação; Reverb/Delay como no manual); todo efeito padrão é válido pelo contrato e tem metadados coerentes |
| [mixing/effectsMeta.ts](mixing/effectsMeta.ts) | Metadados de exibição dos efeitos (rótulos, faixas e unidades) — espelham os limites do domínio. |
| [mixing/mixer.css](mixing/mixer.css) | Estilos |
| [mixing/pitchCorrector.test.ts](mixing/pitchCorrector.test.ts) | Testes: PitchCorrector no mixer; rótulo PT-BR e efeito padrão válido pelo contrato |
| [notation/](notation/) | Subdiretório (coberto por este README) |
| [notation/NoteTextEntry.tsx](notation/NoteTextEntry.tsx) | "Digitar notas" (mockup 8): texto corrido → prévia ao vivo → inserção em lote a partir do cursor. |
| [notation/ScorePanel.tsx](notation/ScorePanel.tsx) | foco pendente (partitura recém-criada): aplicado quando o editor estiver montado, sem depender de atraso fixo |
| [notation/ScoreView.tsx](notation/ScoreView.tsx) | a cada desenho, guarda as linhas de pauta (o SVG só muda quando o alphaTab redesenha) |
| [notation/alphaTab.ts](notation/alphaTab.ts) | Adaptador do alphaTab (renderização de partitura/tablatura e conversão GP3–GP6 → GP7, ADR-0020). |
| [notation/notation.css](notation/notation.css) | Impressão / PDF: só a partitura (window.print; "Salvar como PDF" no diálogo do sistema) |
| [notation/noteText.test.ts](notation/noteText.test.ts) | em Sol maior o Fá é sustenido pela armadura; "Fn" ou "F♮" força o natural |
| [notation/noteText.ts](notation/noteText.ts) | Entrada de notas por texto corrido (mockup 8): "C, C, C, x, x, x, D, D, D, E5:8, F#:8, C+E+G:2". |
| [notation/scoreEditing.test.ts](notation/scoreEditing.test.ts) | Testes: mapa de teclado do editor de partitura; A–G, 0, dígitos, ponto, setas, Delete e ligadura |
| [notation/scoreEditing.ts](notation/scoreEditing.ts) | Regras de edição da partitura no editor (puras, testáveis): teclado → ação, cursor de eventos, |
| [project/](project/) | Subdiretório (coberto por este README) |
| [project/ClipContextMenu.tsx](project/ClipContextMenu.tsx) | Item de alternância (Inverter fase, Reverter). */ |
| [project/ClipEditing.test.tsx](project/ClipEditing.test.tsx) | Testes: menu de contexto do clipe; itens com estado e desabilitados conforme o contexto |
| [project/ClipInspector.tsx](project/ClipInspector.tsx) | Campo numérico em ms que só envia ao confirmar (Enter/perder o foco). */ |
| [project/ClipView.tsx](project/ClipView.tsx) | Bordas e alças que o usuário arrasta (ou ajusta pelo teclado) num clipe. */ |
| [project/TempoPanel.test.tsx](project/TempoPanel.test.tsx) | troca para 3/4 no meio do compasso 2 (tempo 3): o compasso 2 fica cortado e o 3/4 começa o compasso 3 |
| [project/TempoPanel.tsx](project/TempoPanel.tsx) | mesma numeração da régua e do loop: cada mudança de fórmula inicia um compasso novo; |
| [project/Timeline.tsx](project/Timeline.tsx) | zoom pela roda: o tique sob o mouse e sua distância à borda, para manter o ponto parado |
| [project/TrackOrderButtons.test.tsx](project/TrackOrderButtons.test.tsx) | Testes: TrackOrderButtons; primeira não sobe e última não desce |
| [project/TrackOrderButtons.tsx](project/TrackOrderButtons.tsx) | Sobe/desce a faixa na ordem do arranjo (e do mixer); acessível por teclado (Tab + Enter/Espaço). */ |
| [project/clipActions.ts](project/clipActions.ts) | Ações de edição de clipe (Fase 3) compartilhadas por menu de contexto, inspetor e atalhos. |
| [project/clipEdit.test.ts](project/clipEdit.test.ts) | 120 BPM @ 48 kHz: 1 tick = 25 amostras |
| [project/clipEdit.ts](project/clipEdit.ts) | Regras de apresentação da edição de clipes (Fase 3): conversões e limites calculados na UI para |
| [project/grid.test.ts](project/grid.test.ts) | Testes: gera compassos e tempos em 4/4; respeita mudança de fórmula de compasso |
| [project/grid.ts](project/grid.ts) | Cálculos de grade musical a partir do mapa de tempo (puros, testáveis). |
| [project/loopRanges.test.ts](project/loopRanges.test.ts) | 4/4 nos compassos 1–2, 3/4 a partir do compasso 3; e uma troca no meio de um compasso (compasso cortado) |
| [project/popupPosition.test.ts](project/popupPosition.test.ts) | Testes: posição de menus flutuantes; menu do clipe abre no ponteiro e vira quando não cabe |
| [project/spectrogramCache.ts](project/spectrogramCache.ts) | Cache de espectrogramas por fonte (o núcleo também guarda em disco; aqui evita refazer o IPC). |
| [project/timeline.css](project/timeline.css) | uma única linha: se não couber, rola na horizontal em vez de quebrar e esmagar o nome |
| [project/timelineView.test.ts](project/timelineView.test.ts) | mouse a 300 px da borda, no tique 9600 com 0,1 px/tique → depois do zoom 0,2 px/tique |
| [project/timelineView.ts](project/timelineView.ts) | Estado de visualização da timeline (zoom no tempo e altura das faixas), compartilhado entre a |
| [project/waveformCache.ts](project/waveformCache.ts) | Cache de picos por fonte (evita pedir ao núcleo a cada redesenho). |
| [recording/](recording/) | Subdiretório (coberto por este README) |
| [recording/InputSection.tsx](recording/InputSection.tsx) | "Entrada de áudio" do painel Gravação (mockup 7): dispositivo de entrada, medidores por canal e |
| [recording/RecordingPanel.tsx](recording/RecordingPanel.tsx) | Configuração de gravação: entradas, monitoração, takes, metrônomo, punch e latência. */ |
| [recording/inputRoutes.test.ts](recording/inputRoutes.test.ts) | Testes: rotas de entrada; mono por canal e pares estéreo |
| [recording/inputRoutes.ts](recording/inputRoutes.ts) | Rotas de entrada (mono por canal e pares estéreo) — usadas pela entrada da faixa e pelo |
| [recording/recording.css](recording/recording.css) | recolher/abrir a entrada |
| [sync/](sync/) | Subdiretório (coberto por este README) |
| [sync/LyricsPanel.test.tsx](sync/LyricsPanel.test.tsx) | Testes: LyricsPanel; aplica o texto, lista as unidades e define o tempo da selecionada |
| [sync/LyricsPanel.tsx](sync/LyricsPanel.tsx) | o texto acompanha a letra do núcleo (desfazer, importação) enquanto não houver edição pendente |
| [sync/lyrics.css](sync/lyrics.css) | Estilos |
| [sync/lyricsModel.test.ts](sync/lyricsModel.test.ts) | Testes: unidades da letra; por sílaba, palavra e linha com intervalos derivados |
| [sync/lyricsModel.ts](sync/lyricsModel.ts) | Regras puras do painel de letra: unidades por granularidade, texto, posições e a máquina de |
| [transport/](transport/) | Subdiretório (coberto por este README) |
| [transport/LoopPopover.test.ts](transport/LoopPopover.test.ts) | Testes: posição das opções de loop; abaixo do botão, alinhada à direita dele e presa à janela |
| [transport/LoopPopover.tsx](transport/LoopPopover.tsx) | Opções de loop (mockup 10): 4 compassos a partir do cursor, projeto inteiro ou compassos X até Y. |
| [transport/TransportBar.tsx](transport/TransportBar.tsx) | Barra de transporte: tocar/parar/gravar, posição, andamento, metrônomo e loop (com as opções). |
| [transport/transport.css](transport/transport.css) | loop: botão + opções |
| [tuning/](tuning/) | Subdiretório (coberto por este README) |
| [tuning/TuningPanel.tsx](tuning/TuningPanel.tsx) | ------------------------------------------------------------------------------- afinador |
| [tuning/tuning.css](tuning/tuning.css) | Estilos |
| [tuning/tuningMath.test.ts](tuning/tuningMath.test.ts) | Testes: afinador; ponteiro: ±50 cents → ±45°, satura e ignora valores inválidos |
| [tuning/tuningMath.ts](tuning/tuningMath.ts) | Matemática pura do afinador e do editor de afinação (ponteiro, alturas, arraste → edição). |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/contexts`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
