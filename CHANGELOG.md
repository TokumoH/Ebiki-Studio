# Changelog

Todas as mudanças relevantes do Ebiki Studio. Formato baseado em
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/); versões seguem [SemVer](https://semver.org/lang/pt-BR/).

## [Não lançado]

### Novidades

- **Legendas ao passar o mouse** em todo o app: setores (abas), efeitos, cada parâmetro (limiar,
  ataque, liberação…), botões, medidores e campos, com faixa de valores e valor típico. Desligáveis
  em *Exibir › Legendas ao passar o mouse*.

## [0.2.1] - 2026-11-07

Entrada de áudio no painel Gravação, partitura mais direta e correções importantes.

### Novidades

- **Atualizando da 0.2.0? Salve o projeto antes** de clicar em *Instalar e reiniciar*: a 0.2.0 usa
  o fluxo antigo e, no Windows, fecha na hora. A partir da 0.2.1 o app salva sozinho antes de instalar.
- **Entrada de áudio no painel Gravação**: escolha do dispositivo de entrada, medidores de nível por
  canal e **🎧 Ouvir entrada** (ouve o dispositivo direto na saída, com volume próprio, sem armar
  faixa; não entra na gravação nem na exportação).
- **Partitura mais direta**: modo *✎ Inserir notas* (passe o mouse na pauta, veja a nota fantasma
  e clique) e campo **Digitar notas** para lotes em texto (`C, C, C, x, x, x, D, D, D`, com oitava,
  acidentes, duração e acordes), com prévia, erros apontados e um único passo de desfazer.

### Corrigido

- **Atualização no Windows**: o projeto agora é salvo (e a reprodução ou gravação é parada) **antes**
  de instalar. Antes, o salvamento vinha depois da instalação, que no Windows encerra o app na hora —
  alterações feitas desde o último autosave podiam se perder. Se salvar falhar, nada é instalado.
  *Quem atualizar a partir da 0.2.0 ainda usa o fluxo antigo: salve o projeto antes de instalar.*
- Mostrador de andamento não quebra mais em duas linhas em janelas estreitas.
- **Configurações de áudio**: não era possível escolher saída nem entrada — a cada atualização dos
  medidores (~30 por segundo) o diálogo devolvia o foco ao primeiro campo e fechava a lista aberta.
  Afetava todos os diálogos. O diálogo também passa a abrir com os dispositivos e o buffer atuais.

### Downloads

Mesmos formatos da 0.2.0 (Windows `.exe`/`.msi`, macOS `.dmg` universal, Linux `.deb`/`.rpm`/`.AppImage`),
com `SHA256SUMS.txt`. Quem tem a 0.2.0 recebe esta versão em *Ajuda › Verificar atualizações…*.

Manual: [docs/manual/README.md](https://github.com/TokumoH/Ebiki-Studio/blob/branchTrabalho/docs/manual/README.md) · Privacidade: [PRIVACY.md](https://github.com/TokumoH/Ebiki-Studio/blob/branchTrabalho/PRIVACY.md) ·
Licença: GPL-3.0-or-later (nome e logo reservados).

## [0.2.0] - 2026-11-06

Identidade Ebiki em todo o app e nos instaladores, com navegação e ajuda novas.

### Novidades

- **Menu fixo no topo** (estilo Photoshop / VS Code): Arquivo, Editar, Exibir, Transporte, Faixa,
  Clipe, Ferramentas, Janela e Ajuda, com atalhos visíveis e navegação completa por teclado (Alt ou F10).
- **Central de ajuda (F1)**: tópicos práticos com busca, lista de atalhos e *Mostrar no app*.
- **Tutorial de início** pulável em 7 passos, que destaca as áreas reais da interface; refaça em
  *Ajuda › Tutorial de início*.
- **Tela de carregamento** com a marca Ebiki e diálogo *Sobre*.
- **Exibir**: zoom da linha do tempo (Ctrl+= / Ctrl+- / Ctrl+0), tela cheia (F11), barra de status
  alternável, espectrograma da faixa e *Restaurar layout padrão*.
- **Atualizações**: *Ajuda › Verificar atualizações…* mostra as novidades e instala a nova versão
  dentro do app, com assinatura verificada (no Linux, só o AppImage; `.deb`/`.rpm` abrem a página de
  download). Verificar ao abrir é opcional e vem desligado — sem ela, o app não usa a rede.
- **Instaladores com a cara do Ebiki**: telas próprias no instalador do Windows (NSIS e MSI), em
  português e inglês, e fundo personalizado no DMG do macOS.

### Downloads

Mesmos formatos da 0.1.0 (Windows `.exe`/`.msi`, macOS `.dmg` universal, Linux `.deb`/`.rpm`/`.AppImage`),
com `SHA256SUMS.txt`. Instaladores não assinados: veja as instruções da
[0.1.0](https://github.com/TokumoH/Ebiki-Studio/releases/tag/v0.1.0).

Manual: [docs/manual/README.md](https://github.com/TokumoH/Ebiki-Studio/blob/branchTrabalho/docs/manual/README.md) · Privacidade: [PRIVACY.md](https://github.com/TokumoH/Ebiki-Studio/blob/branchTrabalho/PRIVACY.md) ·
Licença: GPL-3.0-or-later (nome e logo reservados).

## [0.1.0] - 2026-11-06

Primeira versão pública. Estúdio musical completo para Windows, macOS e Linux — **offline, sem
contas e sem coleta de dados**.

### Downloads

| Sistema | Arquivo |
|---|---|
| Windows 10/11 (64 bits) | `…_x64-setup.exe` (recomendado) ou `…_x64_en-US.msi` |
| macOS (Apple Silicon e Intel) | `…_universal.dmg` |
| Linux Debian/Ubuntu | `…_amd64.deb` |
| Linux Fedora/openSUSE | `…x86_64.rpm` |
| Linux, qualquer distribuição | `…_amd64.AppImage` |

Confira a integridade com `SHA256SUMS.txt` (ex.: `sha256sum -c SHA256SUMS.txt --ignore-missing`).

> **Instaladores não assinados.** Windows: SmartScreen → *Mais informações* → *Executar assim mesmo*.
> macOS: na primeira execução, se aparecer "não pode ser aberto", vá em *Ajustes do Sistema →
> Privacidade e Segurança → Abrir Mesmo Assim* (macOS 15+), ou clique direito no app → *Abrir*
> (macOS 14 e anteriores). Se o macOS disser que o app "está danificado", rode
> `xattr -dr com.apple.quarantine "/Applications/Ebiki Studio.app"` no Terminal.

### Recursos

- **Gravação**: mono/estéreo, várias entradas simultâneas, punch-in/out, takes, metrônomo com
  acentos, subdivisões e contagem prévia; seleção de dispositivo e buffer; baixa latência.
- **Mixagem**: EQ paramétrico, compressor, gate, saturação, chorus, delay, reverb e limiter; buses e
  envios; medidores pico/RMS/LUFS/correlação; análise da mixagem com conselhos por regras;
  compensação de atraso dos efeitos nas faixas.
- **Edição não destrutiva**: dividir, aparar, fades e crossfades, normalizar, inverter fase,
  reverter, ganho de clipe, espectrograma por faixa; desfazer ilimitado.
- **Composição**: campos harmônicos (maior, menores e modos gregos), progressões por gênero,
  sugestão de próximo acorde, ouvir e inserir no arranjo; cifra ou graus romanos.
- **Bases**: bases geradas pelo app a partir de progressões e ritmos, e bases do usuário, com BPM,
  tom, gênero e adaptação ao projeto.
- **Partitura e tablatura**: editor completo, reprodução por SoundFont General MIDI,
  importação MusicXML/MXL/MIDI/Guitar Pro, exportação MusicXML/MIDI e impressão em PDF.
- **Letra**: sincronização por sílaba (toque ou manual), exportação LRC e SRT.
- **Afinação**: afinador em tempo real, correção de afinação ao vivo e offline, edição nota a nota,
  time-stretch.
- **Formatos**: importa WAV, AIFF, FLAC, MP3, OGG e M4A; exporta WAV, FLAC, AIFF, MP3, OGG e stems.
- Interface em português do Brasil, temas escuro (Odoriko) e claro (Agridoce), painéis encaixáveis,
  atalhos editáveis; projetos portáteis (`.ebiki`) com autosave e recuperação.

### Limitações conhecidas

- Efeitos com atraso em buses não são compensados; a monitoração ao vivo soma o atraso do efeito.
- Áudio dos clipes fica decodificado em memória (≈ 1 h de estéreo por 8 GB de RAM).
- Exportação AAC indisponível; sem plugins VST3/CLAP e sem automação (arquitetura preparada).
- Nenhuma base de áudio CC0 empacotada ainda.

Manual: [docs/manual/README.md](https://github.com/TokumoH/Ebiki-Studio/blob/branchTrabalho/docs/manual/README.md) · Privacidade: [PRIVACY.md](https://github.com/TokumoH/Ebiki-Studio/blob/branchTrabalho/PRIVACY.md) ·
Licença: GPL-3.0-or-later (nome e logo reservados).

[0.2.1]: https://github.com/TokumoH/Ebiki-Studio/releases/tag/v0.2.1
[0.2.0]: https://github.com/TokumoH/Ebiki-Studio/releases/tag/v0.2.0
[0.1.0]: https://github.com/TokumoH/Ebiki-Studio/releases/tag/v0.1.0
