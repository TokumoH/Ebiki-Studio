# ADR-0022: Codificadores de exportação e licenças (WAV, AIFF, FLAC, MP3, OGG)

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Relacionados:** ADR-0003 (crates hexagonais), ADR-0023 (AAC), spec `docs/specs/2026-11-05-fase-9-formatos.md`

## Contexto
A Fase 9 (RF-9.2) exige exportar WAV, FLAC, AIFF, MP3, OGG, AAC e stems. O projeto é desktop
multiplataforma (Windows/macOS/Linux), licença MIT, sem dependências de sistema além do toolchain
Rust e de um compilador C. Os codificadores precisam: (a) compilar nos três SOs sem `cmake`,
`pkg-config` ou bibliotecas instaladas; (b) ter licença compatível com distribuição de um app MIT;
(c) aceitar escrita em fluxo (blocos), para exportações longas sem carregar tudo em memória.

## Decisão
Crate `infra-codecs` (Pure Fabrication) com um `Encoder` único (`new` → `write` → `finish`,
temporário + rename atômico) e um backend por formato:

| Formato | Biblioteca | Licença | Build | Observações |
|---|---|---|---|---|
| WAV 16/24 int, 32 float | `hound` 3.5 | Apache-2.0 | Rust puro | dither TPDF próprio para 16/24 |
| AIFF 16/24, AIFF-C `fl32` | escritor próprio (~150 linhas) | MIT (projeto) | Rust puro | cabeçalho corrigido por seek no `finish` |
| FLAC 16/24 | `flacenc` 0.5 | Apache-2.0 | Rust puro | STREAMINFO regravado com MD5 e total |
| MP3 CBR 128–320 / VBR V0 | `mp3lame-encoder` 0.3 (`mp3lame-sys` com LAME vendorizado) | crate LGPL-3.0; LAME LGPL-2.0 | `cc` (compilador C) | tag LAME/Xing gravada ⇒ gapless exato |
| OGG Vorbis (qualidade −0,2…1,0) | `vorbis_rs` 0.5 (libvorbis aoTuV + libogg vendorizados) | BSD-3-Clause | `cc` | série Ogg determinística (semente) |
| Decodificação (todos) | `symphonia` 0.6 | MPL-2.0 | Rust puro | WAV, AIFF, FLAC, MP3, OGG, AAC-LC, ALAC, CAF, MKV |
| Conversão de taxa | `rubato` 5 (sinc assíncrono) | MIT/Apache-2.0 | Rust puro | 256 pontos, Blackman-Harris² |

MP3 e Vorbis ficam atrás das *features* `mp3` e `vorbis` (ligadas por padrão), permitindo um build
100% Rust (`--no-default-features`) em que esses formatos devolvem `codec.unsupported_format`.

### Obrigações de licença
- **LGPL (LAME):** o LAME é ligado estaticamente. A LGPL permite isso desde que o usuário possa
  religar a aplicação com uma versão modificada da biblioteca. Como o Ebiki Studio é MIT com código-fonte
  público e o LAME é compilado do fonte pelo `cargo`, a obrigação é cumprida publicando o código e as
  instruções de build; o instalador deve incluir o texto da LGPL e o aviso de uso do LAME
  (tela "Sobre"/arquivo `THIRD-PARTY-NOTICES`). Patentes do MP3 expiraram (2017).
- **MPL-2.0 (symphonia):** licença fraca por arquivo; usamos sem modificar — só aviso de terceiros.
- **BSD-3 / Apache-2.0 / MIT:** avisos de copyright em `THIRD-PARTY-NOTICES`.

## Consequências
- Nenhuma dependência de sistema além de um compilador C (já exigido pelo toolchain Rust no Windows
  MSVC/macOS Xcode CLT/Linux build-essential). `cmake` não é necessário.
- Exportação sempre atômica: arquivo parcial nunca aparece com o nome final.
- WAV/AIFF/FLAC verificados por ida e volta exata (sem dither); MP3/OGG por duração exata, RMS e
  correlação.
- A distribuição precisa do arquivo de avisos de terceiros (tarefa de empacotamento, Fase de release).

## Alternativas consideradas
- **`libflac`/`flac-bound` (C):** exigiria `cmake`/libs do sistema; `flacenc` é Rust puro e suficiente.
- **`shine` (MP3, LGPL, ponto fixo):** qualidade inferior ao LAME; sem VBR.
- **`lame` via `pkg-config` do sistema:** quebra builds no Windows/macOS sem dependências manuais.
- **`ogg`/`lewton` (Rust puro):** só decodificam Vorbis; não há codificador Vorbis em Rust puro maduro.
- **Opus em vez de Vorbis:** não pedido nos requisitos; pode ser adicionado depois com `audiopus`.
