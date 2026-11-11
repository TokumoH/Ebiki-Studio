# infra-persistence

| Campo | Valor |
|---|---|
| Id | DIR-crates-infra-persistence |
| Título | infra-persistence |
| Tipo | serviço técnico (GRASP Pure Fabrication) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `e1704ca`; README por módulo em 2026-11-11 (ver `git log -- crates/infra-persistence/README.md`) |
| Arquivos | 15 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Pure Fabrication: persistência SQLite (project.db por projeto e library.db do usuário), migrações e backups.

## Origem

Criado em `e1704ca 2026-11-05 feat: ctx-recording, app-core (sessão e portas) e infra-persistence (project.db/l`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

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
| [migrations/](migrations/) | Subdiretório (coberto por este README) |
| [migrations/project_v1.sql](migrations/project_v1.sql) | project.db — versão 1 (Fases 0–2). Um banco por projeto: sem project_id nas tabelas filhas (FNBC). |
| [migrations/project_v2.sql](migrations/project_v2.sql) | project.db — versão 2 (Fases 6, 7 e 8): letra sincronizada, afinação e partituras. |
| [migrations/project_v3.sql](migrations/project_v3.sql) | project.db — versão 3 (Fase 3): edição não destrutiva de clipes. |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/effects.rs](src/effects.rs) | Mapeamento de efeitos para tabelas normalizadas (effect_slot / effect_param / eq_band). |
| [src/harmony_repo.rs](src/harmony_repo.rs) | Repositórios do 'library.db' para Harmonia (Fase 5) e Bases (Fase 4) — Pure Fabrication. |
| [src/lib.rs](src/lib.rs) | Persistência SQLite (Pure Fabrication). Implementa 'app_core::ports::SessionRepository'. |
| [src/library.rs](src/library.rs) | 'library.db': dados globais do usuário (projetos recentes, preferências). Migração 2: esquema de |
| [src/music_tables.rs](src/music_tables.rs) | Mapeamento dos agregados das Fases 6–8 (letra, afinação, partituras) para o 'project.db' v2. |
| [src/project_repo.rs](src/project_repo.rs) | Repositório da sessão em 'project.db'. |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/music_roundtrip.rs](tests/music_roundtrip.rs) | Fases 6–8: letra, afinação e partituras sobrevivem a salvar → carregar (identidade). |
| [tests/roundtrip.rs](tests/roundtrip.rs) | R4: salvar → carregar devolve exatamente o mesmo estado. |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/infra-persistence`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
