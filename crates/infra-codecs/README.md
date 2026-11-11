# infra-codecs

| Campo | Valor |
|---|---|
| Id | DIR-crates-infra-codecs |
| Título | infra-codecs |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `a24aa8e`; README por módulo em 2026-11-11 (ver `git log -- crates/infra-codecs/README.md`) |
| Arquivos | 26 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Pure Fabrication de codecs: probe/decodificação, conversão de taxa, importação para o projeto, picos de waveform e exportação (WAV, AIFF, FLAC, MP3, OGG).

## Origem

Criado em `a24aa8e 2026-11-05 feat(infra-codecs): probe/decodificação, importação, picos e exportação de áudio`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um módulo por responsabilidade técnica; testes de integração em `tests/` com dados e auxiliares em `tests/common`/`tests/data` quando existem.

## Por que esta organização

Isolar o técnico mantém o domínio puro e permite trocar a biblioteca sem mexer nas regras de negócio.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Lógica técnica dentro dos contextos | O domínio deixaria de ser puro e testável sem I/O |
| Um crate técnico único | Dependências pesadas (codecs, SQLite, áudio) entrariam em todos os consumidores |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/decode.rs](src/decode.rs) | Probe e decodificação para 'f32' não intercalado (symphonia), com downmix e conversão de taxa. |
| [src/downmix.rs](src/downmix.rs) | Redução de canais (downmix) para estéreo. |
| [src/error.rs](src/error.rs) | Erros do serviço de codecs, com código estável (exposto à UI) e mensagem em PT-BR. |
| [src/format.rs](src/format.rs) | Formatos de origem reconhecidos e lista branca de extensões de importação. |
| [src/fsutil.rs](src/fsutil.rs) | Utilitários de arquivo: gravação atômica (temporário + rename) e confinamento de caminhos. |
| [src/import.rs](src/import.rs) | Importação de arquivos de áudio para a pasta do projeto. |
| [src/lib.rs](src/lib.rs) | 'infra-codecs' — serviço técnico (GRASP Pure Fabrication) de formatos de áudio. |
| [src/peaks.rs](src/peaks.rs) | Picos de waveform (mín./máx. por canal e por bloco) e cache binário regenerável. |
| [src/resample.rs](src/resample.rs) | Conversão de taxa de amostragem em fluxo (sinc de alta qualidade, 'rubato'). |
| [src/spectrogram.rs](src/spectrogram.rs) | Espectrograma (RF-3.5): STFT com janela de Hann, magnitude em dB quantizada para 'u8' e |
| [src/encode/](src/encode/) | Subdiretório (coberto por este README) |
| [src/encode/aiff.rs](src/encode/aiff.rs) | Escritor AIFF (PCM 16/24 bits big-endian) e AIFF-C 'fl32' (32 bits float), em fluxo. |
| [src/encode/dither.rs](src/encode/dither.rs) | Quantização para inteiros com dither TPDF (densidade triangular, ±1 LSB). |
| [src/encode/flac.rs](src/encode/flac.rs) | Escritor FLAC em fluxo sobre o 'flacenc' (Rust puro). |
| [src/encode/mod.rs](src/encode/mod.rs) | Exportação: escritor em fluxo para WAV, AIFF, FLAC, MP3 e OGG Vorbis. |
| [src/encode/mp3.rs](src/encode/mp3.rs) | Escritor MP3 sobre o LAME ('mp3lame-encoder', compilado do código-fonte; LGPL — ADR-0022). |
| [src/encode/vorbis.rs](src/encode/vorbis.rs) | Escritor Ogg Vorbis sobre 'vorbis_rs' (libvorbis aoTuV + libogg, BSD-3; compilado com 'cc'). |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/import.rs](tests/import.rs) | Importação para a pasta do projeto: hash, deduplicação, confinamento e limites. |
| [tests/peaks_cache.rs](tests/peaks_cache.rs) | Cache de picos: gravação, reutilização e regeneração quando corrompido. |
| [tests/robustness.rs](tests/robustness.rs) | Entrada malformada nunca causa pânico: bytes aleatórios e arquivos válidos truncados ou |
| [tests/roundtrip.rs](tests/roundtrip.rs) | Ida e volta codificar → decodificar com sinais sintéticos (seno e ruído). |
| [tests/common/](tests/common/) | Subdiretório (coberto por este README) |
| [tests/common/mod.rs](tests/common/mod.rs) | Geradores de sinal e utilitários para os testes de integração (fixtures só em 'tempfile'). |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/infra-codecs`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
