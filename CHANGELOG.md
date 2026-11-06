# Changelog

Todas as mudanças relevantes do Ebiki Studio. Formato baseado em
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/); versões seguem [SemVer](https://semver.org/lang/pt-BR/).

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

Manual: [docs/manual/README.md](docs/manual/README.md) · Privacidade: [PRIVACY.md](PRIVACY.md) ·
Licença: GPL-3.0-or-later (nome e logo reservados).

[0.1.0]: https://github.com/TokumoH/Ebiki-Studio/releases/tag/v0.1.0
