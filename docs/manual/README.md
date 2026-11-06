# Ebiki Studio — Manual de Uso

Estúdio musical para computador (Windows, macOS e Linux): gravação, mixagem, edição, harmonia,
bases, partitura, letra sincronizada e afinação.

## 1. Instalação e execução

### Usuário final
Baixe o instalador da página de *Releases* do repositório (gerado pela CI em cada versão `v*`).

### A partir do código
Passo a passo completo (Windows, macOS, Linux, roteiro de teste e problemas comuns):
[`guia-instalacao-local.md`](guia-instalacao-local.md).

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

Ao abrir, a **tela de carregamento** mostra a marca Ebiki e o progresso. No primeiro projeto, um
**tutorial de início** (7 passos, pulável com Esc) destaca as áreas principais; refaça-o em
*Ajuda › Tutorial de início*.

### Menu principal e ajuda

O menu fixo no topo reúne todas as ações: **Arquivo, Editar, Exibir, Transporte, Faixa, Clipe,
Ferramentas, Janela e Ajuda**. Os atalhos aparecem ao lado de cada item.

- **Alt** ou **F10** levam o teclado ao menu; ←/→ trocam de menu, ↑/↓ percorrem, Enter ativa, Esc fecha.
- **Exibir**: tema escuro/claro, zoom da linha do tempo (Ctrl+= / Ctrl+- / Ctrl+0), painéis,
  espectrograma da faixa, barra de status, tela cheia (F11) e *Restaurar layout padrão*.
- **Janela**: abre, fecha ou traz qualquer painel para a frente. **Ferramentas**: atalhos para os painéis
  de análise, afinação, harmonia, bases, partitura, letra e tempo.
- **F1** abre a **Central de ajuda**: busca sem acentos por tópicos, lista de atalhos e o botão
  *Mostrar no app*, que leva o tutorial até a área do tópico.


1. **Tela inicial** → *Novo projeto*: escolha nome, pasta, taxa de amostragem (padrão 48 kHz) e
   resolução (padrão 24 bits). O projeto é uma pasta `Nome.ebiki` com tudo dentro (portátil).
2. **Configurações de áudio** (*Arquivo › Configurações de áudio…*): escolha saída, entrada e buffer.
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

1. No painel **Gravação**, bloco **Entrada de áudio**: escolha o dispositivo de entrada (a saída e o
   buffer continuam em *Arquivo › Configurações de áudio*). Os medidores mostram o nível de cada canal.
   - **🎧 Ouvir entrada** toca o dispositivo direto na saída, com volume próprio e sem armar faixa —
     útil para testar o microfone e ajustar o ganho da interface. Não entra na gravação nem na
     exportação. Começa desligado a cada abertura do app; use fones para evitar microfonia.
2. Escolha a entrada de cada faixa (mono ou par estéreo) e a monitoração da faixa
   (desligada, quando armada ou sempre).
3. Arme a faixa (⏺) e pressione **R** (ou o botão de gravar).
4. **Metrônomo**: liga/desliga (C), volume, subdivisão, contagem (0, 1, 2 ou 4 compassos).
5. **Punch in/out**: defina os compassos; só o trecho entre eles é gravado.
6. **Takes**: cada passada vira um take; escolha o take ativo na lista da faixa.
7. **Compensação de latência**: use a estimativa (buffer de entrada + saída) ou meça e ajuste.

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

### 10.1 Atualizações do app

- **Ajuda › Verificar atualizações…** consulta a página de releases. Se houver versão nova, o
  diálogo mostra as novidades e oferece **Instalar e reiniciar**: o app baixa o pacote, confere a
  **assinatura digital** (arquivo adulterado é recusado) e reinicia. Antes de instalar, o transporte
  para (uma gravação em andamento vira take) e as alterações não salvas são salvas; se salvar falhar,
  nada é instalado.
- **Verificar ao abrir o app** (no mesmo diálogo, desligado por padrão): com ela ligada, uma versão
  nova aparece só como aviso na barra de status — nunca interrompe uma gravação.
- **Pular esta versão** silencia o aviso automático daquela versão (a verificação manual ainda a mostra).
- Windows, macOS e Linux **AppImage** atualizam dentro do app. Instalações **.deb/.rpm** pertencem
  ao gerenciador do sistema: o diálogo abre a página de download para instalar a nova versão por cima.
- Sem internet, o app avisa e continua funcionando normalmente.

## 11. Atalhos (editáveis em *Editar › Atalhos de teclado…*, Ctrl+/)

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
| Central de ajuda | F1 |
| Atalhos de teclado | Ctrl+/ |
| Aproximar / afastar / zoom padrão | Ctrl+= / Ctrl+- / Ctrl+0 |
| Tela cheia | F11 |
| Menu principal | Alt ou F10 |

## 12. Limitações conhecidas

- Áudio dos clipes fica decodificado em memória (≈ 1 h de estéreo por 8 GB de RAM).
- Compensação de atraso de efeitos (limiter, correção de afinação) é feita **nas faixas**, na
  reprodução e na exportação. Efeitos com atraso em **buses** não são compensados, e a entrada
  monitorada ao vivo soma o atraso do efeito (≈ 58 ms na correção de afinação) — ponha-o em bypass
  enquanto grava (ADR-0032).
- Arquivos gerados que deixaram de ser usados (renders desfeitos, takes substituídos) ficam em
  `audio/` durante a sessão (o desfazer precisa deles) e são apagados ao reabrir o projeto.
- Exportação AAC indisponível; MP3 até 48 kHz.
- Sem plugins VST3/CLAP e sem automação nesta versão (arquitetura preparada — ADR-0011).
- Nenhuma base de áudio CC0 vem empacotada ainda: use as bases geradas ou adicione as suas.
- Efeitos **por clipe** ainda não existem (efeitos são por faixa/bus no mixer).
