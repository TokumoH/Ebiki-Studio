# Architecture Decision Records

Gerado por `scripts/adr_index.py`.

| ADR | Decisão | Status |
|---|---|---|
| [0001](0001-registrar-decisoes.md) | Registrar decisões de arquitetura com ADRs | Aceita |
| [0002](0002-stack-tauri-rust-react.md) | Stack: Tauri + Rust + React/TypeScript | Aceita |
| [0003](0003-ddd-hexagonal-crates.md) | DDD + Hexagonal + um crate por Bounded Context | Aceita |
| [0004](0004-backend-audio-cpal.md) | Backend de áudio: cpal atrás da porta AudioBackend | Aceita |
| [0005](0005-grafo-audio-lock-free.md) | Grafo de áudio compilado com troca lock-free | Aceita |
| [0006](0006-persistencia-dois-sqlite.md) | Persistência: SQLite por projeto + SQLite de biblioteca | Aceita (migrações: ver ADR-0013) |
| [0007](0007-progressoes-graus-relativos.md) | Progressões harmônicas em graus relativos | Aceita |
| [0008](0008-undo-command-pattern.md) | Undo/Redo com Command Pattern em memória | Aceita |
| [0009](0009-ipc-tipado.md) | IPC tipado com tauri-specta e channels binários | Substituída em parte pela ADR-0013 (tauri-specta → fixtures + zod) |
| [0010](0010-alphatab-notacao.md) | alphaTab para partitura e tablatura | Aceita |
| [0011](0011-portas-extensao.md) | Portas de extensão para IA, plugins e automação | Aceita |
| [0012](0012-exclusoes-escopo.md) | Exclusões: Spotify e drivers próprios | Aceita |
| [0013](0013-ajustes-de-implementacao-fase-0.md) | Ajustes de implementação da Fundação | Aceita (substitui partes das ADR-0006 e ADR-0009) |
| [0014](0014-crate-dsp-independente.md) | Crate `dsp` independente do domínio, com contrato `Processor` de tempo real | Aceita |
| [0016](0016-dsp-altura-yin-psola-wsola.md) | DSP de altura — YIN, PSOLA de latência constante e WSOLA | Aceita (modo autônomo) |
| [0017](0017-letra-silabica-em-ticks.md) | Letra sincronizada por sílaba em ticks, LRC/SRT e mapa de warp | Aceita (modo autônomo) |
| [0018](0018-graus-romanos-relativos-ao-modo.md) | Graus romanos relativos ao modo, qualidade diatônica derivada e sintaxe de baixo/secundária | Aceita (modo autônomo) |
| [0019](0019-gerador-de-bases-sem-dependencia-de-harmony.md) | Gerador de bases recebe acordes realizados; vocabulário compartilhado por código | Aceita (modo autônomo) |
| [0020](0020-guitar-pro-via-alphatab-gp7.md) | Importação de Guitar Pro via alphaTab (GP3–GP6 → GP7) + leitor GPIF em Rust | Aceita |
| [0021](0021-modelo-notacao-e-io.md) | Modelo de notação próprio (pauta + tablatura sobre as mesmas notas) e I/O isolado | Aceita |
| [0022](0022-codificadores-exportacao-licencas.md) | Codificadores de exportação e licenças (WAV, AIFF, FLAC, MP3, OGG) | Aceita (modo autônomo) |
| [0023](0023-exportacao-aac-adiada.md) | Exportação AAC/M4A adiada (importação suportada) | Aceita (modo autônomo) |
| [0025](0025-soundfont-rustysynth-e-audicao-wav-em-memoria.md) | SoundFont opcional via `rustysynth`; audição como WAV em memória; arraste vira áudio | Aceita (modo autônomo) |
| [0026](0026-biblioteca-de-composicao-persistencia-e-decisao-de-bases.md) | Biblioteca de composição — persistência, histórico próprio e decisão explícita ao inserir bases | Aceita (modo autônomo) |
| [0027](0027-partitura-documento-versionado-com-cabecalho-normalizado.md) | Partitura no `project.db` como documento versionado com cabeçalho normalizado | Aceita |
| [0028](0028-afinador-por-derivacao-lock-free-da-entrada.md) | Afinador por derivação lock-free da entrada na engine | Aceita |
| [0029](0029-processamento-de-altura-e-tempo-gera-fonte-derivada.md) | Correção de afinação, time-stretch e partitura renderizada geram fontes derivadas; corretor em tempo real como insert | Aceita |
| [0030](0030-nome-do-produto-ebiki-studio.md) | Nome do produto — Ebiki Studio | Aceita |
| [0031](0031-edicao-nao-destrutiva-de-clipes.md) | Edição de clipes não destrutiva (janela + processamento na engine) e espectrograma como fabricação pura | Aceita |
| [0032](0032-compensacao-de-latencia-por-leitura-antecipada.md) | Compensação de latência de efeitos por leitura antecipada dos clipes | Aceita |
| [0033](0033-menu-fixo-ajuda-tutorial-e-identidade-dos-instaladores.md) | Menu fixo, central de ajuda, tutorial e identidade visual dos instaladores | Aceita |

Números não usados: 0015, 0024 (reservados durante o desenvolvimento em paralelo e nunca publicados).
