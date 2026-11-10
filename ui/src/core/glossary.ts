// Glossário das legendas ao passar o mouse (mockup 9): um texto curto por setor, efeito, parâmetro e
// controle, em português. Funções puras: `tipByKey` (chave explícita em `data-tip`) e `tipByName`
// (nome acessível do controle — aria-label, title ou texto — por nome exato ou padrão).

export interface TipEntry {
  title: string;
  body: string;
  /** Faixa de valores, valor típico ou atalho. */
  meta?: string;
  /** Padrões do nome acessível que também usam esta legenda. */
  match?: RegExp[];
}

const e = (title: string, body: string, meta?: string, match?: RegExp[]): TipEntry => ({ title, body, meta, match });

export const GLOSSARY: Record<string, TipEntry> = {
  // ------------------------------------------------------------- setores (abas)
  "panel.timeline": e("Arranjo", "As faixas e os clipes no tempo. Arraste clipes para mover, as bordas para aparar e os cantos para fazer fades.", "Ctrl+roda: zoom no tempo · Alt+roda: altura das faixas"),
  "panel.mixer": e("Mixer", "Um canal por faixa, buses e o master: volume, pan, mute/solo, efeitos (inserts) e envios para buses."),
  "panel.recording": e("Gravação", "Entrada de áudio, faixas armadas, monitoração, metrônomo, contagem, punch in/out e compensação de latência."),
  "panel.tempo": e("Mapa de tempo", "Mudanças de andamento (BPM) e de fórmula de compasso ao longo do projeto."),
  "panel.advisor": e("Análise da mixagem", "Mede volume (LUFS), picos e equilíbrio da mixagem e sugere ajustes por regras, sem IA."),
  "panel.score": e("Partitura", "Editor de partitura e tablatura: notas por clique, por texto ou pelo teclado; ouvir, validar e exportar."),
  "panel.lyrics": e("Letra", "Letra sincronizada por linha, palavra ou sílaba; exporta LRC ou SRT."),
  "panel.tuning": e("Afinação", "Afinador em tempo real e editor de afinação dos clipes de voz (nota a nota ou pela escala)."),
  "panel.harmony": e("Harmonia", "Campo harmônico do tom, progressões por gênero e sugestão do próximo acorde."),
  "panel.bases": e("Bases", "Bases instrumentais geradas pelo app ou suas, com BPM, tom e gênero; insira no arranjo."),
  "panel.clip": e("Clipe", "Edição do clipe selecionado: ganho, fades, normalizar, inverter fase, reverter e time-stretch."),

  // -------------------------------------------------------------------- efeitos
  "effect.eq": e("Equalizador", "Realça ou corta faixas de frequência: tira embolado nos graves, dá brilho nos agudos, limpa ruídos.", "até 8 bandas"),
  "effect.compressor": e("Compressor", "Diminui a diferença entre partes fortes e fracas, deixando o som mais estável e presente."),
  "effect.gate": e("Gate", "Silencia o sinal quando ele fica abaixo do limiar — corta ruído e vazamento entre as frases."),
  "effect.limiter": e("Limitador", "Impede que o sinal passe do teto. Usado no master para subir o volume sem distorcer (clipar)."),
  "effect.reverb": e("Reverb", "Simula o som de um ambiente (sala, salão), dando espaço e profundidade."),
  "effect.delay": e("Delay", "Repete o som depois de um tempo (eco). Com realimentação, as repetições continuam e somem aos poucos."),
  "effect.saturation": e("Saturação", "Distorção suave que adiciona harmônicos, calor e corpo, como equipamento analógico."),
  "effect.chorus": e("Chorus", "Duplica o som com pequenas variações de tempo e afinação, deixando-o mais largo e cheio."),
  "effect.deEsser": e("De-esser", "Reduz o chiado dos \"s\" e \"x\" na voz, atuando só na região aguda quando ela passa do limiar."),
  "effect.pitchCorrector": e("Correção de afinação", "Puxa a voz para as notas da escala escolhida em tempo real. Soma ~58 ms de atraso: ponha em bypass ao gravar."),

  // ------------------------------------------------------------------ parâmetros
  "param.compressor.thresholdDb": e("Limiar", "Nível a partir do qual o compressor começa a agir. Quanto mais baixo, mais do sinal é comprimido.", "−60 a 0 dB · voz: −24 a −12 dB"),
  "param.compressor.ratio": e("Razão", "Quanto o sinal acima do limiar é reduzido. 4:1 = cada 4 dB acima viram 1 dB.", "1:1 a 20:1 · voz: 2:1 a 4:1"),
  "param.compressor.attackMs": e("Ataque", "Tempo para o compressor agir depois que o som passa do limiar. Curto segura picos; longo deixa passar o \"golpe\" inicial.", "0,1–200 ms · voz: 5–15 ms"),
  "param.compressor.releaseMs": e("Liberação", "Tempo para o compressor parar de agir depois que o som volta abaixo do limiar.", "5–2000 ms · voz: 80–200 ms"),
  "param.compressor.kneeDb": e("Joelho", "Suaviza a entrada da compressão em volta do limiar (joelho macio soa mais natural).", "0–24 dB"),
  "param.compressor.makeupDb": e("Ganho", "Ganho de compensação depois da compressão, para recuperar o volume perdido.", "0–24 dB"),
  "param.gate.thresholdDb": e("Limiar", "Abaixo deste nível o gate fecha (silencia).", "−90 a 0 dB"),
  "param.gate.attackMs": e("Ataque", "Tempo para o gate abrir quando o sinal passa do limiar.", "0,05–50 ms"),
  "param.gate.holdMs": e("Retenção", "Tempo que o gate fica aberto depois que o sinal cai, evitando cortes no meio das notas.", "0–500 ms"),
  "param.gate.releaseMs": e("Liberação", "Tempo para o gate fechar por completo.", "5–2000 ms"),
  "param.gate.rangeDb": e("Atenuação", "Quanto o sinal é abaixado com o gate fechado (−90 dB ≈ silêncio).", "−90 a 0 dB"),
  "param.limiter.ceilingDb": e("Teto", "Nível máximo de saída: nada passa daqui.", "−24 a 0 dB · master: −1 dB"),
  "param.limiter.inputGainDb": e("Ganho de entrada", "Empurra o sinal contra o teto para subir o volume percebido.", "0–24 dB"),
  "param.limiter.releaseMs": e("Liberação", "Tempo para o limitador soltar depois de um pico.", "1–1000 ms"),
  "param.reverb.roomSize": e("Tamanho", "Tamanho do ambiente simulado: maior = cauda mais longa.", "0–1"),
  "param.reverb.damping": e("Amortecimento", "Quanto os agudos da cauda são absorvidos (ambiente mais \"abafado\").", "0–1"),
  "param.reverb.width": e("Largura", "Abertura estéreo do reverb.", "0–1"),
  "param.reverb.predelayMs": e("Pré-atraso", "Tempo antes do reverb começar; separa a voz do ambiente e mantém a clareza.", "0–200 ms · voz: 20–60 ms"),
  "param.reverb.mix": e("Mistura", "Proporção entre som original (seco) e reverb (molhado). Em bus, use 100%.", "0–1"),
  "param.delay.timeMs": e("Tempo", "Intervalo entre o som e cada repetição.", "1–2000 ms · 500 ms = semínima a 120 BPM"),
  "param.delay.feedback": e("Realimentação", "Quanto de cada repetição volta para o delay (mais = mais repetições).", "0–0,95"),
  "param.delay.highCutHz": e("Corte de agudos", "Escurece as repetições, para não competirem com o som original.", "500–20000 Hz"),
  "param.delay.mix": e("Mistura", "Proporção entre som original e repetições.", "0–1"),
  "param.saturation.driveDb": e("Saturação", "Quanto o sinal é empurrado para a distorção.", "0–36 dB"),
  "param.saturation.tone": e("Tom", "Equilíbrio entre graves e agudos da distorção.", "0–1"),
  "param.saturation.mix": e("Mistura", "Proporção entre som original e saturado (compressão paralela).", "0–1"),
  "param.chorus.rateHz": e("Velocidade", "Velocidade da oscilação do chorus.", "0,05–10 Hz"),
  "param.chorus.depthMs": e("Profundidade", "Quanto o tempo da cópia varia (mais = efeito mais evidente).", "0,1–15 ms"),
  "param.chorus.mix": e("Mistura", "Proporção entre som original e com chorus.", "0–1"),
  "param.deEsser.freqHz": e("Frequência", "Onde ficam os \"s\" da voz que serão controlados.", "2–12 kHz · típico: 5–8 kHz"),
  "param.deEsser.thresholdDb": e("Limiar", "A partir deste nível na região aguda, o de-esser reduz.", "−60 a 0 dB"),
  "param.deEsser.reductionDb": e("Redução máx.", "Quanto o de-esser pode abaixar os chiados, no máximo.", "−24 a 0 dB"),
  "param.pitchCorrector.retuneMs": e("Velocidade (retune)", "Quão rápido a nota é puxada para a afinada. 0 ms = efeito robótico; 50–200 ms = natural.", "0–1000 ms"),
  "param.pitchCorrector.humanize": e("Humanizar", "Deixa notas longas com variação natural em vez de retas.", "0–1"),
  "param.pitchCorrector.a4Hz": e("Lá4", "Frequência de referência da afinação.", "415–466 Hz · padrão 440 Hz"),
  "param.eq.kind": e("Tipo da banda", "Passa-altas/baixas cortam abaixo/acima da frequência; shelf realça ou corta tudo além dela; pico age só em volta."),
  "param.eq.freqHz": e("Frequência", "Centro (ou corte) da banda.", "20 Hz – 20 kHz"),
  "param.eq.gainDb": e("Ganho", "Quanto a banda é realçada (+) ou cortada (−).", "−24 a +24 dB"),
  "param.eq.q": e("Q", "Largura da banda: Q alto = estreita e cirúrgica; Q baixo = larga e suave.", "0,1–18"),

  // ------------------------------------------------------------------- mixer
  "mixer.volume": e("Volume (fader)", "Nível do canal na mixagem. 0 dB = sem alteração.", "duplo clique: 0 dB · setas com foco", [/^Volume (?!de monitoração)/]),
  "mixer.pan": e("Pan", "Posição no estéreo: esquerda, centro (C) ou direita.", "−100 (E) a +100 (D)", [/^Pan /]),
  "mixer.mute": e("Mute", "Silencia o canal.", undefined, [/^Mute /]),
  "mixer.solo": e("Solo", "Ouve só os canais em solo (os outros ficam mudos).", undefined, [/^Solo /]),
  "mixer.insert": e("Efeito (insert)", "Efeito no caminho do canal; o som passa por ele inteiro. Clique para editar.", undefined, [/^Editar (Equalizador|Compressor|Gate|Limitador|Reverb|Delay|Saturação|Chorus|De-esser|Correção de afinação)$/]),
  "mixer.addEffect": e("Adicionar efeito", "Coloca um efeito (insert) neste canal.", undefined, [/^Adicionar efeito em /]),
  "mixer.send": e("Envio", "Manda uma cópia do som para um bus (ex.: um reverb compartilhado), sem tirar do canal.", undefined, [/^Envio de /]),
  "mixer.output": e("Saída", "Para onde o canal vai: master ou um bus.", undefined, [/^Saída de /]),
  "mixer.bypass": e("Bypass", "Desliga o efeito temporariamente, sem perder os ajustes.", undefined, [/^Bypass$/]),
  "mixer.bus": e("Bus", "Canal que recebe envios de várias faixas — ideal para reverb e delay compartilhados.", undefined, [/^\+ Bus$/]),
  "mixer.channel": e("Canal", "Uma faixa (ou bus) no mixer. Clique para selecionar.", undefined, [/^Canal /]),

  // ---------------------------------------------------------------- transporte
  "transport.play": e("Tocar / pausar", "Inicia ou pausa a reprodução a partir do cursor.", "Espaço", [/^Tocar( \(Espaço\))?$/, /^Pausar( \(Espaço\))?$/]),
  "transport.stop": e("Parar", "Para a reprodução ou a gravação.", "Enter volta ao início", [/^Parar \(Enter/]),
  "transport.record": e("Gravar", "Grava nas faixas armadas (⏺). Cada gravação vira um take novo.", "R", [/^Gravar/]),
  "transport.home": e("Voltar ao início", "Leva o cursor para o começo do projeto.", "Home", [/^Voltar ao início/]),
  "transport.metronome": e("Metrônomo", "Clique no tempo durante a reprodução e a gravação. Volume, subdivisão e contagem no painel Gravação.", "C", [/^Metrônomo$/]),
  "transport.loop": e("Loop", "Repete um trecho na reprodução. ▾ escolhe o trecho: 4 compassos, projeto inteiro ou compassos X–Y. Na gravação o loop é ignorado.", "L", [/^Loop$/]),
  "transport.loopMenu": e("Opções de loop", "Escolha o trecho do loop. Também dá para arrastar na faixa de cima da régua.", undefined, [/^Opções de loop$/]),
  "transport.position": e("Posição", "Compasso.tempo.tique e tempo decorrido do cursor."),
  "transport.tempo": e("Andamento e compasso", "BPM e fórmula de compasso no cursor. Mude no painel Mapa de tempo."),
  "transport.undo": e("Desfazer", "Desfaz a última ação.", "Ctrl+Z", [/^Desfazer/]),
  "transport.redo": e("Refazer", "Refaz o que foi desfeito.", "Ctrl+Shift+Z", [/^Refazer/]),

  // ------------------------------------------------------------------ arranjo
  "timeline.ruler": e("Régua", "Clique para mover o cursor. Na faixa de cima, arraste para desenhar o loop e arraste as bordas para ajustá-lo.", undefined, [/^Régua/]),
  "timeline.newTrack": e("Nova faixa", "Cria uma faixa de áudio vazia.", "Ctrl+T", [/^Nova faixa/, /^\+ Faixa$/]),
  "track.arm": e("Armar", "Prepara a faixa para gravar. Só faixas armadas recebem a gravação.", undefined, [/^Armar /]),
  "track.spectrum": e("Espectrograma", "Mostra as frequências do áudio em vez da forma de onda.", undefined, [/^Espectrograma /]),
  "track.remove": e("Remover faixa", "Apaga a faixa e seus clipes (dá para desfazer).", undefined, [/^Remover faixa$/]),
  "track.move": e("Reordenar", "Move a faixa para cima ou para baixo.", undefined, [/^Mover .* para (cima|baixo)$/]),
  "clip.item": e("Clipe", "Trecho de áudio na faixa. Arraste para mover; bordas aparam; cantos fazem fade; botão direito: mais opções.", "S divide no cursor", [/^Clipe /]),
  "clip.handle": e("Alça do clipe", "Arraste para aparar o início/fim ou ajustar o fade.", undefined, [/^(Início|Fim|Fade de entrada|Fade de saída) — /]),

  // ------------------------------------------------------------------ gravação
  "rec.inputDevice": e("Dispositivo de entrada", "Interface ou microfone usado para gravar. A saída e o buffer ficam em Arquivo › Configurações de áudio.", undefined, [/^Dispositivo de entrada da gravação$/]),
  "rec.inputMeter": e("Nível da entrada", "Sinal que chega do dispositivo. Mire em −18 a −6 dB nos trechos fortes; perto de 0 dB vai distorcer.", undefined, [/^Nível da entrada /]),
  "rec.listen": e("Ouvir entrada", "Toca o dispositivo direto na saída, sem armar faixa. Não entra na gravação nem na exportação. Use fones.", undefined, [/^Ouvir entrada$/]),
  "rec.listenChannels": e("Canais para ouvir", "Qual entrada (mono) ou par (estéreo) ouvir.", undefined, [/^Canais para ouvir$/]),
  "rec.listenVolume": e("Volume de monitoração", "Volume do \"Ouvir entrada\". Não muda o nível gravado.", "−60 a +6 dB", [/^Volume de monitoração$/]),
  "rec.trackInput": e("Entrada da faixa", "Qual canal do dispositivo esta faixa grava: mono (um canal) ou estéreo (par).", undefined, [/^Entrada de /]),
  "rec.monitor": e("Monitoração da faixa", "Ouvir a entrada pela faixa (com os efeitos dela): desligada, quando armada ou sempre.", undefined, [/^Monitoração de /]),
  "rec.take": e("Take ativo", "Cada gravação vira um take; escolha qual toca.", undefined, [/^Take ativo de /]),
  "rec.subdivision": e("Subdivisão", "Cliques entre os tempos: colcheias, tercinas ou semicolcheias."),
  "rec.countIn": e("Contagem", "Compassos de metrônomo antes de começar a gravar."),
  "rec.punch": e("Punch in/out", "Grava só entre dois compassos — para regravar um trecho sem mexer no resto."),
  "rec.latency": e("Compensação de latência", "Desloca o que foi gravado para alinhar com o que você ouviu. Use a estimativa ou meça.", "amostras"),

  // ------------------------------------------------------------------ outros
  "audio.buffer": e("Buffer", "Tamanho do bloco de áudio. Menor = menos atraso (bom para gravar); maior = mais estável (mixagens pesadas).", "64–128 gravar · 512+ mixar", [/^Tamanho do buffer$/]),
  "audio.output": e("Dispositivo de saída", "Onde o som toca (fones, caixas, interface).", undefined, [/^Dispositivo de saída$/]),
  "audio.input": e("Dispositivo de entrada", "Interface ou microfone usado para gravar.", undefined, [/^Dispositivo de entrada$/]),
  "status.audio": e("Áudio", "Dispositivo de saída em uso. \"Modo silencioso\" = nenhum dispositivo disponível; o app funciona sem som."),
  "status.format": e("Formato do projeto", "Taxa de amostragem e resolução em que o projeto grava e mixa."),
  "status.buffer": e("Buffer e latência", "Tamanho do buffer e atraso estimado de ida e volta."),
  "score.mode": e("Modo do mouse", "Selecionar: clique para editar notas. Inserir notas: passe o mouse na pauta e clique para inserir.", undefined, [/^Modo do mouse na pauta$/]),
  "score.accidental": e("Acidente do clique", "Força bemol, bequadro ou sustenido na nota inserida por clique. Clique de novo para voltar à armadura.", undefined, [/^(Bemol|Bequadro|Sustenido)$/]),
  "score.text": e("Digitar notas", "C, C, C, x, x, x, D — x = pausa, C5 = oitava, :8 = duração, C+E+G = acorde. Enter insere tudo.", undefined, [/^Notas em texto$/]),
  "score.dot": e("Ponto de aumento", "Aumenta a duração da nota em metade.", ".", [/^Ponto de aumento/]),
  "score.tie": e("Ligadura de valor", "Junta duas notas iguais numa só duração.", "+", [/^Ligadura de valor/]),
  "score.dynamic": e("Dinâmica", "Intensidade a partir da nota (pp, p, mf, f, ff…).", undefined, [/^Dinâmica$/]),
  "tuning.a4": e("Referência do Lá4", "Frequência usada como Lá para medir a afinação.", "padrão 440 Hz", [/^Referência do Lá4$/]),
  "tuning.scale": e("Escala", "Notas permitidas na correção: a afinação puxa para as notas desta escala.", undefined, [/^Tipo de escala$/, /^Tônica da escala$/]),
  "export.format": e("Formato", "WAV/AIFF/FLAC: sem perda. MP3/OGG: arquivos menores, com perda.", undefined, [/^Formato de exportação$/]),
  "export.normalize": e("Normalizar", "Ajusta o volume para o pico final chegar ao nível escolhido."),
  "export.stems": e("Stems", "Exporta cada faixa em um arquivo separado, para mixar ou masterizar fora."),
  "rec.metronomeVolume": e("Volume do metrônomo", "Volume do clique (não entra na gravação nem na exportação)."),
  "export.bitDepth": e("Resolução", "Bits por amostra: 16 para CD/streaming; 24 para guardar ou masterizar; 32 float sem limite de pico.", "padrão 24 bits"),
  "export.quality": e("Qualidade", "Taxa do MP3: maior = melhor som e arquivo maior.", "320 kbps ≈ transparente"),
  "advisor.peak": e("Pico (dBFS)", "Maior amostra da mixagem. 0 dBFS é o limite digital."),
  "advisor.rms": e("RMS", "Média de energia do sinal — uma ideia do volume médio, sem a curva da audição (para isso, LUFS)."),
  "advisor.target": e("Meta de loudness", "Volume alvo da plataforma onde a música será publicada; a análise compara a mixagem com ele."),
  // gravação / tempo (complementos)
  "rec.accent": e("Acentuar 1º tempo", "Clique mais forte no primeiro tempo de cada compasso, para não se perder."),
  "rec.punchFrom": e("Do compasso", "Onde o punch in começa a gravar."),
  "rec.punchTo": e("Até o compasso", "Onde o punch out para de gravar."),
  "rec.punchSet": e("Definir punch", "Ativa o punch in/out nos compassos escolhidos: só esse trecho será gravado."),
  "rec.punchClear": e("Limpar punch", "Desliga o punch: volta a gravar a partir do cursor."),
  "tempo.bar": e("Compasso", "Compasso onde a mudança começa (1 = início)."),
  "tempo.bpm": e("BPM", "Batidas por minuto: o andamento a partir do compasso escolhido.", "20–999"),
  "tempo.setBpm": e("Definir andamento", "Cria ou altera a mudança de BPM no compasso escolhido."),
  "tempo.numerator": e("Tempos", "Quantos tempos tem o compasso (o número de cima: 3 em 3/4)."),
  "tempo.denominator": e("Figura", "Que figura vale um tempo (o número de baixo: 4 = semínima, 8 = colcheia)."),
  "tempo.setMeter": e("Definir compasso", "Cria ou altera a fórmula de compasso a partir do compasso escolhido."),
  // menus principais (resolvidos por data-menu-id)
  "menu.arquivo": e("Arquivo", "Projeto: novo, abrir, salvar, importar, exportar e configurações de áudio."),
  "menu.editar": e("Editar", "Desfazer/refazer, dividir e excluir clipes, atalhos de teclado."),
  "menu.exibir": e("Exibir", "Tema, zoom, painéis, espectrograma, legendas, barra de status, tela cheia e layout."),
  "menu.transporte": e("Transporte", "Tocar, parar, gravar, metrônomo e loop."),
  "menu.faixa": e("Faixa", "Criar, armar e excluir faixas."),
  "menu.clipe": e("Clipe", "Dividir, normalizar, inverter fase, reverter e excluir o clipe selecionado."),
  "menu.ferramentas": e("Ferramentas", "Abre os painéis de análise, afinação, harmonia, bases, partitura, letra e tempo."),
  "menu.janela": e("Janela", "Mostra ou esconde cada painel e restaura o layout padrão."),
  "menu.ajuda": e("Ajuda", "Central de ajuda (F1), tutorial, atalhos, atualizações e sobre."),
  "menu.app": e("Menu do Ebiki", "Atalhos para projeto, exportar, configurações, legendas, atualizações e ajuda."),
  // zoom
  "view.zoomOut": e("Afastar", "Mostra mais tempo na tela.", "Ctrl+- · Ctrl+roda", [/^Diminuir zoom$/, /^Afastar$/]),
  "view.zoomIn": e("Aproximar", "Mostra mais detalhe do tempo.", "Ctrl+= · Ctrl+roda", [/^Aumentar zoom$/, /^Aproximar$/]),
  // análise
  "advisor.run": e("Analisar mixagem", "Renderiza a mixagem atual e mede volume, picos e estéreo, com conselhos.", undefined, [/^Analisar mixagem$/]),
  // partitura (complementos)
  "score.new": e("Nova partitura", "Cria uma partitura em branco com título, tom, compasso e instrumentos.", undefined, [/^\+ Nova partitura$/]),
  "score.import": e("Importar", "Abre MusicXML, MXL, MIDI ou Guitar Pro como nova partitura.", undefined, [/^Importar…$/]),
  // letra
  "lyrics.apply": e("Aplicar texto", "Usa o texto colado como letra; hífens separam as sílabas.", undefined, [/^Aplicar texto$/]),
  "lyrics.granularity": e("Unidade de sincronização", "Sincronizar por sílaba, palavra ou linha inteira.", undefined, [/^(Sílaba|Palavra|Linha)$/, /^Unidades da letra$/]),
  "lyrics.tap": e("Sincronizar por toque", "Toca o projeto e você pressiona Espaço a cada unidade; Enter conclui.", undefined, [/^Sincronizar por toque$/, /^Sincronização por toque$/]),
  "lyrics.export": e("Exportar letra", "Salva a letra sincronizada em LRC (players) ou SRT (legendas de vídeo).", undefined, [/^Exportar…$/, /^Formato de exportação da letra$/]),
  "lyrics.import": e("Importar letra", "Abre uma letra já sincronizada em LRC ou SRT.", undefined, [/^Importar LRC\/SRT…$/]),
  // afinador
  "tuning.tuner": e("Afinador", "Mostra a nota e o desvio em cents do que chega na entrada. 0 cents = afinado.", undefined, [/^(Ligar|Desligar) afinador$/]),
  // harmonia
  "harmony.key": e("Tom", "Tônica e modo (maior, menor, modos gregos) do campo harmônico.", undefined, [/^Tônica$/, /^Modo$/]),
  "harmony.notation": e("Cifra ou graus", "Mostra os acordes como cifra (C, Am…) ou graus romanos (I, vi…), que valem em qualquer tom.", undefined, [/^(Cifra|Graus)$/]),
  "harmony.chordSize": e("Tríades ou tétrades", "Acordes de 3 notas ou de 4 notas (com sétima).", undefined, [/^(Tríades|Tétrades)$/]),
  "harmony.target": e("Faixa de destino", "Faixa onde a progressão é inserida.", undefined, [/^Faixa: /]),
  "harmony.views": e("Modo da harmonia", "Progressões prontas por gênero, sugestão do próximo acorde ou suas progressões.", undefined, [/^(Progressões|Próximo acorde|Minhas progressões)$/]),
  "harmony.genre": e("Gênero", "Filtra as progressões típicas do estilo.", undefined, [/^Gênero$/]),
  "common.preview": e("Ouvir", "Toca uma prévia (sem mexer no projeto).", undefined, [/^Ouvir (?!entrada)/, /^Ouvir$/]),
  "common.stopPreview": e("Parar audição", "Para a prévia que está tocando.", undefined, [/^Parar (audição|sequência)$/, /^Parar (?!\()/]),
  "common.insert": e("Inserir no arranjo", "Coloca no arranjo, na faixa de destino, a partir do cursor.", undefined, [/^Inserir .+ no arranjo$/, /^Inserir (?!notas)/]),
  // bases
  "bases.search": e("Buscar bases", "Procura por nome, gênero, tom ou tags.", undefined, [/^Buscar bases$/]),
  "bases.bpm": e("Faixa de BPM", "Mostra só bases entre os andamentos mínimo e máximo.", undefined, [/^BPM (mínimo|máximo)$/]),
  "bases.filter": e("Filtro", "Restringe a lista de bases por este critério.", undefined, [/^Filtrar por /]),
  "bases.generate": e("Gerar base", "Cria uma base a partir de progressão, ritmo, andamento e compassos (SoundFont).", undefined, [/^Gerar base…$/]),
  "bases.addAudio": e("Adicionar áudio", "Cadastra um loop ou base sua, com BPM, tom e gênero.", undefined, [/^Adicionar áudio…$/]),
  "bases.soundfont": e("SoundFont", "Banco de timbres usado para gerar bases e tocar partituras (General MIDI).", undefined, [/^SoundFont…$/]),
  "bases.clear": e("Limpar filtros", "Remove todos os filtros e mostra todas as bases."),
  "harmony.clearSeq": e("Limpar sequência", "Apaga a sequência de acordes montada."),
  "lyrics.clearSpan": e("Limpar tempo", "Remove o tempo sincronizado da unidade selecionada."),
  "tuning.clear": e("Limpar análise", "Descarta as notas detectadas e as correções deste clipe (o áudio original não muda)."),
  "advisor.lufs": e("LUFS", "Volume percebido médio. Streaming costuma usar −14 LUFS integrados."),
  "advisor.truePeak": e("Pico real (dBTP)", "Pico entre amostras. Fique abaixo de −1 dBTP para evitar distorção na conversão."),
  "advisor.correlation": e("Correlação", "Compatibilidade estéreo/mono: +1 = mono; perto de 0 = muito largo; negativo = cancela em mono."),
};

const norm = (s: string) => s.replace(/\s+/g, " ").trim();

export function tipByKey(key: string): TipEntry | undefined {
  return GLOSSARY[key];
}

/** Legenda pelo nome acessível do controle (padrões das entradas, depois título exato). */
export function tipByName(name: string): TipEntry | undefined {
  const n = norm(name);
  if (!n) return undefined;
  for (const entry of Object.values(GLOSSARY)) if (entry.match?.some((re) => re.test(n))) return entry;
  // nome exato só para efeitos (títulos únicos); parâmetros repetem nomes ("Limiar") e usam chave
  const lower = n.toLowerCase();
  const hit = Object.entries(GLOSSARY).find(([k, x]) => k.startsWith("effect.") && x.title.toLowerCase() === lower);
  return hit?.[1];
}

/** Legenda do setor pelo título da aba. */
export function tipForPanelTitle(title: string, catalog: readonly { id: string; title: string }[]): TipEntry | undefined {
  const p = catalog.find((c) => c.title === norm(title));
  return p ? GLOSSARY[`panel.${p.id}`] : undefined;
}
