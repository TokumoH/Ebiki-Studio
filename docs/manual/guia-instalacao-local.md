# Ebiki Studio — Guia de instalação e teste local

Há **dois jeitos** de testar:

| Modo | Para quê | Tempo |
|---|---|---|
| **A. Só a interface no navegador** (núcleo simulado) | Ver telas, fluxos e atalhos; sem áudio real | ~5 min |
| **B. App desktop completo** (Tauri + Rust) | Tudo de verdade: áudio, gravação, arquivos | 20–40 min na 1ª vez (compilação) |

Requisitos de máquina para o modo B: ~10 GB livres em disco (compilação) e 8 GB de RAM.

---

## 1. Pré-requisitos

### Todos os sistemas
1. **Git** — <https://git-scm.com/downloads>
2. **Node.js 22 LTS** — <https://nodejs.org> (confira com `node -v`)
3. **Rust (rustup)** — só para o modo B — <https://rustup.rs>.
   A versão exata (1.97) é baixada sozinha na primeira compilação (`rust-toolchain.toml`).

### Windows (modo B)
- **Microsoft C++ Build Tools**: instale o "Visual Studio Build Tools" e marque
  **"Desenvolvimento para desktop com C++"**.
- **WebView2**: já vem no Windows 10/11 atualizado.
- No instalador do rustup, aceite o padrão (`x86_64-pc-windows-msvc`).

### macOS (modo B)

```bash
xcode-select --install
```

### Linux — Ubuntu/Debian (modo B)

```bash
sudo apt update
```

```bash
sudo apt install -y build-essential curl libwebkit2gtk-4.1-dev libgtk-3-dev libasound2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev librsvg2-dev libayatana-appindicator3-dev
```

(Fedora: `webkit2gtk4.1-devel gtk3-devel alsa-lib-devel libsoup3-devel librsvg2-devel`.)

---

## 2. Baixar o código

```bash
git clone --branch branchTrabalho https://github.com/TokumoH/Ebiki-Studio.git
```

```bash
cd Ebiki-Studio
```

```bash
npm --prefix ui ci
```

---

## 3A. Testar só a interface (navegador)

```bash
npm run ui
```

Abra <http://localhost:5173>. Tudo funciona com dados simulados: criar projeto ("Escolher…"
preenche uma pasta fictícia), importar (gera duas faixas de exemplo), mixer, edição, harmonia,
partitura, letra, afinação. **Não há som real nem arquivos gravados no disco.**
Para parar: `Ctrl+C` no terminal.

## 3B. Rodar o app desktop completo

```bash
npm run dev
```

- A primeira vez compila ~600 dependências (pode levar de 10 a 30 min); depois, segundos.
- A janela "Ebiki Studio" abre sozinha. Alterações no código da interface recarregam ao vivo.
- Sem placa de som disponível o app continua em **modo silencioso** (aviso na barra de status).

## 4. Gerar o instalador (opcional)

```bash
npm run build
```

Os instaladores ficam em `target/release/bundle/`:

| Sistema | Arquivos |
|---|---|
| Windows | `msi/…msi` e `nsis/…-setup.exe` |
| macOS | `dmg/…dmg` e `macos/Ebiki Studio.app` |
| Linux | `deb/…deb`, `appimage/…AppImage`, `rpm/…rpm` |

Os instaladores **não são assinados**:
- Windows: SmartScreen → "Mais informações" → "Executar assim mesmo".
- macOS: clique direito no app → **Abrir** → Abrir.

---

## 5. Roteiro de teste (15 minutos)

1. **Novo projeto** → nome, pasta, 48 kHz / 24 bits → *Criar projeto*.
2. **⚙ Áudio** (barra superior): escolha saída, entrada e buffer (128 para gravar).
3. **Importar** (Ctrl+I ou arrastar arquivos) um WAV/MP3 → aparece uma faixa com a forma de onda.
4. **Espaço** toca/para. Clique na régua para posicionar. **≋** na faixa alterna para espectrograma.
5. **Mixer**: mova o fader e o pan; *+ efeito* → Compressor/EQ/Reverb; crie um **Bus** e um envio.
6. **Edição**: selecione o clipe → **S** divide no cursor; arraste as bordas (aparar) e os cantos
   superiores (fades); clique direito → Normalizar / Inverter fase / Reverter. **Ctrl+Z** desfaz tudo.
7. **Gravação**: arme a faixa (⏺ no cabeçalho), escolha a entrada no painel *Gravação*, ligue o
   metrônomo e aperte **R** (ou ⏺ no transporte). Pare com Espaço → nasce um take/clipe.
   (macOS pede permissão de microfone na primeira vez.)
8. **Harmonia**: escolha tom e gênero → *Ouvir* → *Inserir no arranjo*.
9. **Bases**: *Gerar base…* ou *Adicionar áudio…*; ao inserir com BPM/tom diferente, o app pergunta.
10. **Partitura**: *+ Nova partitura*, digite notas pelo teclado, ouça; exporte MusicXML/MIDI ou *Imprimir / PDF*.
11. **Letra**: cole o texto, *Aplicar texto*, sincronize por toque (Espaço marca) e exporte LRC/SRT.
12. **Afinação**: afinador em tempo real; no editor, analise um clipe de voz e corrija notas.
13. **Análise da mixagem**: meta −14 LUFS → *Analisar mixagem* → conselhos.
14. **Exportar** (barra superior): WAV/FLAC/MP3/OGG, ou stems.
15. Feche e reabra o projeto (*Recentes*): tudo deve voltar igual.

---

## 6. Testes automatizados

Tudo o que a CI do GitHub verifica, rodando no seu computador (útil sem minutos do Actions):

```bash
scripts/ci-local.sh
```

(`scripts/ci-local.sh rust` ou `scripts/ci-local.sh ui` para só uma parte; no Windows use o Git Bash.)
Ou, mais rápido:

```bash
npm test
```

(Rust + testes unitários da interface.) Testes de ponta a ponta no navegador:

```bash
npm --prefix ui exec playwright install chromium
```

```bash
npm --prefix ui run e2e
```

---

## 7. Problemas comuns

| Sintoma | Solução |
|---|---|
| `error: linker 'link.exe' not found` (Windows) | Instale o C++ Build Tools (seção 1) e abra um terminal novo |
| `webkit2gtk-4.1 not found` (Linux) | Instale os pacotes da seção 1 |
| Porta 5173 ocupada | Feche o outro `npm run dev/ui` ou rode `npx kill-port 5173` |
| "Sem dispositivo de áudio" | Conecte e selecione a interface em ⚙ Áudio; no Linux verifique PipeWire/ALSA (`aplay -l`) |
| Gravação muda/silenciosa no macOS | Ajustes do Sistema → Privacidade → **Microfone** → ative o Ebiki Studio |
| Estalos no áudio | Aumente o buffer em ⚙ Áudio (256–512) |
| Compilação lenta/sem espaço | Normal na 1ª vez; `cargo clean` libera espaço depois |

## 8. Onde ficam os dados

- **Projeto**: a pasta `Nome.ebiki` que você escolheu (banco `project.db`, `audio/`, `backups/`) — portátil.
- **Biblioteca** (recentes, bases do usuário, progressões): pasta de dados do app `com.ebikistudio.app`
  - Windows: `%APPDATA%\com.ebikistudio.app`
  - macOS: `~/Library/Application Support/com.ebikistudio.app`
  - Linux: `~/.local/share/com.ebikistudio.app`
