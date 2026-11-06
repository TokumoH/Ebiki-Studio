# Ebiki Studio — Manual de Uso

Estúdio musical para computador (Windows, macOS e Linux): gravação, mixagem, edição, harmonia,
bases, partitura, letra sincronizada e afinação.

## 1. Instalação e execução

### Usuário final
Baixe o instalador da página de *Releases* do repositório (gerado pela CI em cada versão `v*`).

### A partir do código
Pré-requisitos: Rust 1.97 (fixado em `rust-toolchain.toml`), Node 22, e no Linux as bibliotecas do
WebKitGTK/ALSA (`libwebkit2gtk-4.1-dev libgtk-3-dev libasound2-dev libsoup-3.0-dev librsvg2-dev`).

```bash
npm --prefix ui ci
```

```bash
npm run dev
```

Para ver apenas a interface no navegador, com um núcleo simulado (sem áudio):

```bash
npm run ui
```

Testes completos (Rust + UI):

```bash
npm test
```

## 2. Primeiros passos

1. **Tela inicial** → *Novo projeto*: escolha nome, pasta, taxa de amostragem (padrão 48 kHz) e
   resolução (padrão 24 bits). O projeto é uma pasta `Nome.ebiki` com tudo dentro (portátil).
2. **Configurações de áudio** (⚙ na barra superior): escolha saída, entrada e buffer.
   - Buffers de 64–128 amostras dão a menor latência (gravação); 512+ para mixagens pesadas.
   - Sem dispositivo disponível, o app continua funcionando em modo silencioso (barra de status avisa).
3. **Importar áudio** (*Importar* ou Ctrl+I, ou arraste arquivos para a janela): WAV, AIFF, FLAC,
   MP3, OGG, AAC/M4A. O arquivo é copiado para o projeto e convertido para a taxa do projeto;
   o original nunca é alterado.

## 3. Arranjo (timeline)

- Arraste clipes na horizontal (encaixa nos tempos; segure **Alt** para mover livremente) e entre
  faixas. Cada arrasto é um único *desfazer*.
- **Delete** remove o clipe selecionado. Duplo clique no nome da faixa renomeia.
- Clique na régua para posicionar a reprodução. Zoom com **−/+** no canto da timeline.
- **M** (mute), **S** (solo) e **⏺** (armar para gravação) em cada faixa.
- **≋** em cada faixa alterna entre **forma de onda** e **espectrograma** (frequências graves embaixo,
  agudas em cima; cores claras = mais energia).

### 3.1 Edição de clipes (não destrutiva)

Nada do que você faz aqui altera o arquivo de áudio original: o projeto guarda só o trecho usado e
os ajustes, aplicados na reprodução e na exportação. Tudo pode ser desfeito (Ctrl+Z).

- **Cortar:** arraste a **borda esquerda ou direita** do clipe (encaixa nos tempos; **Alt** = livre).
  Puxar a borda para fora revela áudio que havia sido cortado, até o limite do arquivo.
- **Dividir:** posicione o cursor (clique na régua) dentro do clipe e pressione **S** (ou *Dividir no
  cursor* no menu/painel). O som não muda; você passa a ter dois clipes independentes.
- **Fades:** arraste as **alças quadradas nos cantos superiores** do clipe; a curva desenhada mostra a
  atenuação. No painel **Clipe** dá para digitar a duração em ms e escolher a curva
  (*Linear* ou *Equal power*, mais natural para música).
- **Crossfade:** sobreponha dois clipes da mesma faixa (estenda/arraste um sobre o outro), selecione um
  deles e use *Crossfade* (menu ou painel). A sobreposição vira fade de saída + fade de entrada
  (equal power).
- **Normalizar:** ajusta o **ganho do clipe** para o pico chegar a −1 dBFS.
- **Ganho do clipe:** slider no painel **Clipe** (−24 a +12 dB; duplo clique volta a 0 dB).
- **Inverter fase** (Ø) e **Reverter** (⇆, toca de trás para frente): botões do painel ou itens do menu.
- **Menu do clipe:** botão direito no clipe, ou tecla **Menu** / **Shift+F10** com o clipe
  selecionado. Navegue com as setas, **Enter** executa, **Esc** fecha.
- **Teclado nas alças:** com o clipe selecionado, **Tab** até uma borda/alça e use **←/→**
  (cortar: 1/16 de tempo, **Shift** = 1 tempo; fades: 10 ms, **Shift** = 100 ms).

## 4. Gravação

1. No painel **Gravação**, escolha a entrada de cada faixa (mono ou par estéreo) e a monitoração.
2. Arme a faixa (⏺) e pressione **R** (ou o botão de gravar).
3. **Metrônomo**: liga/desliga (C), volume, subdivisão, contagem (0, 1, 2 ou 4 compassos).
4. **Punch in/out**: defina os compassos; só o trecho entre eles é gravado.
5. **Takes**: cada passada vira um take; escolha o take ativo na lista da faixa.
6. **Compensação de latência**: use a estimativa (buffer de entrada + saída) ou meça e ajuste.

## 5. Mixagem

- **Mixer**: fader (arraste; **Shift** para ajuste fino; duplo clique = 0 dB; setas do teclado),
  pan (knob), mute/solo, saída (master ou bus) e envios para buses.
- **Efeitos** (*+ efeito*): EQ paramétrico (até 8 bandas), compressor, gate, limitador, reverb,
  delay, saturação, chorus, de-esser e correção de afinação. Clique no efeito para editar;
  *Bypass* desliga sem remover.
- **Presets por instrumento**: voz principal, backing vocal, rap, violão, guitarra limpa/distorcida,
  baixo, bateria, piano e master (substituem a cadeia do canal; desfazível).
- **Análise da mixagem**: renderiza e mede loudness integrado (LUFS), true peak, pico, RMS e
  correlação de fase; lista conselhos por regras (clipping, loudness acima/abaixo da meta,
  problemas de fase, compressão excessiva, canais muito baixos).

## 6. Exportação

*Exportar* (Ctrl+E): WAV/AIFF (16/24/32 float), FLAC (16/24), MP3 (CBR 128–320 ou V0), OGG Vorbis.
Opções: normalizar para −1 dBFS e **stems** (um arquivo por faixa). AAC ainda não é suportado na
exportação (licenciamento — ADR-0023). MP3 aceita até 48 kHz.

## 7. Harmonia e composição

- Escolha tom e modo (maior, menores, modos gregos) para ver o **campo harmônico** em cifra ou graus.
- Filtre por gênero para ver **progressões** com boa química; ouça e **insira no arranjo** na posição
  atual (vira áudio numa faixa nova; desfazível).
- **Próximo acorde**: monte uma sequência clicando nas sugestões (cada uma mostra o motivo).
- Cadastre suas próprias progressões.

## 8. Bases instrumentais

- Pesquise por texto, BPM, tom, gênero, instrumento e tags; ouça antes de inserir.
- **Gerar base**: progressão + tom + BPM + compasso + estilo (pop, balada, bossa, samba, rock,
  reggae, funk, valsa, guarânia) + bateria + baixo.
- Ao inserir uma base com BPM/tom diferente do projeto, o app **sempre pergunta**: *Manter* ou
  *Adaptar ao projeto* (time-stretch e transposição).
- **SoundFont**: escolha um arquivo `.sf2` para timbres realistas; sem ele, um sintetizador
  embutido é usado.

## 9. Partitura, letra e afinação

- **Partitura**: crie, importe (MusicXML, MXL, MIDI, Guitar Pro 7; GP3–6 são convertidos na
  interface) e exporte (MusicXML, MXL, MIDI); partitura e tablatura; ouça ou renderize para uma faixa.
- **Letra**: cole o texto (separação silábica automática), sincronize por sílaba/palavra/linha
  tocando a barra de espaço durante a reprodução, exporte LRC ou SRT.
- **Afinação**: afinador em tempo real na entrada; editor de notas por clipe (analisar, arrastar
  notas, corrigir para a escala, aplicar — o original é preservado); correção em tempo real como
  efeito no mixer; *time-stretch* de clipe para o BPM do projeto.

## 10. Salvamento e recuperação

- **Ctrl+S** salva. O ponto laranja ao lado do nome indica alterações não salvas.
- **Autosave** a cada 2 minutos em `backups/` (5 últimos). Se o app fechar sem salvar, ao abrir o
  projeto você pode **recuperar** o autosave mais novo ou abrir a versão salva.
- Desfazer/refazer ilimitado durante a sessão (Ctrl+Z / Ctrl+Shift+Z).

## 11. Atalhos (editáveis em ⌨)

| Ação | Atalho |
|---|---|
| Tocar / pausar | Espaço |
| Parar e voltar ao início | Enter |
| Gravar | R |
| Desfazer / Refazer | Ctrl+Z / Ctrl+Shift+Z ou Ctrl+Y |
| Salvar | Ctrl+S |
| Nova faixa | Ctrl+T |
| Importar áudio | Ctrl+I |
| Exportar | Ctrl+E |
| Metrônomo | C |
| Loop | L |
| Início | Home |
| Dividir clipe no cursor | S |
| Menu do clipe selecionado | Menu ou Shift+F10 |

## 12. Limitações conhecidas

- Áudio dos clipes fica decodificado em memória (≈ 1 h de estéreo por 8 GB de RAM).
- Sem compensação automática de atraso de plugins: a correção de afinação em tempo real atrasa a
  faixa em ≈ 58 ms (use a correção offline para resultado alinhado).
- Arquivos gerados que deixaram de ser usados (após desfazer) permanecem em `audio/` até uma
  limpeza futura.
- Exportação AAC indisponível; MP3 até 48 kHz.
- Sem plugins VST3/CLAP e sem automação nesta versão (arquitetura preparada — ADR-0011).
- Efeitos **por clipe** ainda não existem (efeitos são por faixa/bus no mixer).
