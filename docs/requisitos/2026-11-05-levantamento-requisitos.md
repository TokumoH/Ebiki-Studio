# Ebiki Studio — Levantamento de Requisitos

- **Data:** 2026-11-05
- **Status:** Aprovado pelo usuário (questionário de 12 rodadas)
- **Branch de trabalho:** `branchTrabalho`

## 1. Visão

Estúdio musical **desktop multiplataforma** (Windows, macOS, Linux), de **uso pessoal**, que integra gravação, mixagem, edição, teoria musical, notação e composição. **Sem IA** na versão inicial, porém com **arquitetura preparada para integração futura de IA** em todas as áreas.

## 2. Stack Tecnológica

| Camada | Tecnologia |
|---|---|
| Shell desktop | Tauri |
| Domínio, aplicação, engine de áudio | Rust |
| Interface | React + TypeScript |
| Persistência | SQLite (por projeto) |
| CI | GitHub Actions (Windows, macOS, Linux) |
| Distribuição | Instaladores via GitHub Releases (manual) |

## 3. Arquitetura (obrigatória)

- DDD (Bounded Contexts por área funcional)
- GRASP Pure Fabrication (serviços técnicos isolados do domínio)
- Arquitetura de Componentes (componentes independentes e reutilizáveis)
- Portas/adaptadores para extensões futuras: **IA**, **plugins VST3/CLAP**, **automação de parâmetros**

## 4. Processo de Desenvolvimento

| Item | Decisão |
|---|---|
| Commits | Livres na `branchTrabalho`; **push somente com autorização** |
| Testes | TDD completo: unitários no domínio + integração na infraestrutura |
| CI | Build + testes nos 3 sistemas a cada push |
| Documentação por fase | Especificação; diagramas (casos de uso, máquina de estados, componentes, classes, sequência, serviços, processos — quando necessários); documentação de arquitetura; modelo de dados (DER + dicionário); ADRs; manual de uso |
| Idioma | Interface PT-BR; código (identificadores) em inglês |
| Componentes visuais | Sempre perguntar e oferecer mockup antes de implementar |

## 5. Requisitos Não Funcionais

| ID | Requisito |
|---|---|
| RNF-01 | Latência de monitoração: alvo **< 10 ms**; aceitável **≤ 15 ms** |
| RNF-02 | Baixa latência via APIs nativas: ASIO e WASAPI exclusivo (Windows), CoreAudio (macOS), JACK/PipeWire (Linux) |
| RNF-03 | Buffer adaptativo e thread de áudio em tempo real sem alocação/bloqueio |
| RNF-04 | Taxa de amostragem configurável por projeto; padrão **48 kHz / 24-bit** |
| RNF-05 | Mínimo de **32 faixas** simultâneas, arquitetura expansível |
| RNF-06 | Funcionar com qualquer hardware de áudio (não acoplado à máquina do usuário) |
| RNF-07 | Edição **não destrutiva** (arquivos originais nunca alterados) |
| RNF-08 | Banco de dados na maior forma normal possível, com leitura eficiente |

## 6. Requisitos Funcionais por Fase

### Fase 0 — Fundação
- RF-0.1 Criar, abrir e salvar projeto (pasta + SQLite + subpasta de áudios)
- RF-0.2 Áudios importados são **copiados** para o projeto
- RF-0.3 Autosave periódico + salvamento manual
- RF-0.4 Desfazer/refazer **ilimitado** durante a sessão
- RF-0.5 Mapa de tempo com **BPM e fórmula de compasso variáveis**
- RF-0.6 Timeline multitrilha com reprodução sincronizada, volume e pan
- RF-0.7 Importar WAV e MP3 (demais formatos na Fase 9)

### Fase 1 — Captura
- RF-1.1 Gravação mono e estéreo
- RF-1.2 Gravação de múltiplas entradas simultâneas
- RF-1.3 Punch-in / punch-out
- RF-1.4 Múltiplos takes por faixa
- RF-1.5 Metrônomo configurável (sons, acentos, subdivisões, contagem prévia)
- RF-1.6 Seleção de dispositivo de entrada/saída e tamanho de buffer

### Fase 2 — Mixagem
- RF-2.1 Efeitos nativos com foco em **voz, violão e guitarra**: EQ paramétrico, compressor, reverb, delay, limiter/maximizer, gate, saturação, chorus (e demais viáveis)
- RF-2.2 Roteamento: faixas → buses/grupos → master, com **sends** auxiliares
- RF-2.3 Auxílio por regras: medidores pico/RMS, alerta de clipping, LUFS com meta (−14 LUFS), presets por instrumento, checagem mono/correlação de fase
- RF-2.4 (Futuro) Automação de parâmetros — arquitetura preparada

### Fase 3 — Edição / Pós-produção
- RF-3.1 Cortar, dividir, mover clipes
- RF-3.2 Fades e crossfades
- RF-3.3 Normalizar, ganho de clipe, inverter fase, reverter
- RF-3.4 Efeitos e características por clipe/faixa
- RF-3.5 Waveform com zoom/rolagem (visão principal) e **espectrograma selecionável**

### Fase 4 — Biblioteca de Bases
- RF-4.1 Pacote de bases livres (CC0) incluído
- RF-4.2 Bases **geradas pelo app** a partir de padrões de acordes/ritmos + soundfont
- RF-4.3 Bases do usuário
- RF-4.4 Metadados: BPM, tom, gênero, instrumentos, progressão harmônica, tags livres
- RF-4.5 Ao inserir base com BPM/tom diferente do projeto: **perguntar** (ajustar ou manter)

### Fase 5 — Harmonia & Composição
- RF-5.1 Campos harmônicos: maior, menores (natural, harmônica, melódica) e modos gregos
- RF-5.2 Progressões por gênero (pop, MPB, samba, rock, jazz…) + progressões cadastradas pelo usuário
- RF-5.3 Sugestão por tom + gênero → progressões
- RF-5.4 Sugestão de próximo acorde
- RF-5.5 Ouvir progressão sugerida
- RF-5.6 Arrastar progressão para a timeline
- RF-5.7 Notação alternável: cifra popular ↔ graus romanos

### Fase 6 — Sincronização
- RF-6.1 Sincronização de letra **por palavra/sílaba**
- RF-6.2 Exportar letra em **LRC e SRT**
- RF-6.3 Grade de BPM / sincronização de instrumentos
- RF-6.4 **Time-stretch** (alterar tempo sem alterar tom)

### Fase 7 — Notação
- RF-7.1 **Partitura e tablatura**
- RF-7.2 **Editor completo** (articulações, dinâmicas, múltiplas vozes, layout)
- RF-7.3 Auxílios: validar compasso, transposição, destacar notas fora da escala, gerar partitura a partir de progressão
- RF-7.4 Reprodução via soundfont General MIDI + soundfonts do usuário

### Fase 8 — Afinação
- RF-8.1 Afinador em tempo real
- RF-8.2 Correção de pitch em tempo real
- RF-8.3 Correção de pitch em pós-produção
- RF-8.4 Edição manual nota a nota

### Fase 9 — Formatos
- RF-9.1 Importar: WAV, AIFF, FLAC, MP3, OGG, AAC/M4A, MIDI, MusicXML, Guitar Pro
- RF-9.2 Exportar: WAV, FLAC, AIFF, MP3, OGG, AAC, stems, MIDI, MusicXML, PDF

## 7. Interface

| Item | Decisão |
|---|---|
| Tema | Alternável; **escuro padrão** |
| Layout | Janela única com **painéis encaixáveis** |
| Atalhos | Padrão DAW + editáveis |
| MIDI | Apenas arquivos (sem controlador) |

## 8. Fora do Escopo

- IA na versão inicial (arquitetura preparada)
- Integração com Spotify (descartada: termos de uso proíbem captura de áudio)
- Drivers de áudio próprios (substituídos por APIs nativas de baixa latência)
- Entrada de teclado/controlador MIDI
- Auto-update

## 9. Riscos Identificados

| Risco | Mitigação |
|---|---|
| Latência < 10 ms em todos os sistemas | APIs nativas + buffer adaptativo; meta formal ≤ 15 ms |
| Editor completo de partitura/tablatura é grande | Fase própria, decomposta em sub-entregas |
| Correção de pitch em tempo real (DSP complexo) | Fase 8 após engine estabilizada |
| Importação Guitar Pro (formato proprietário) | Avaliar bibliotecas existentes na Fase 9 |
