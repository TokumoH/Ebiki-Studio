# ADR-0006: Persistência: SQLite por projeto + SQLite de biblioteca

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
Projetos devem ser portáteis (pasta com tudo); conhecimento musical e preferências são globais.

## Decisão
`project.db` dentro da pasta `.amproj` (WAL, áudios copiados com nome por hash) e `library.db` na pasta de dados do usuário. `rusqlite` + migrações `refinery`. Repository por agregado. Autosave com `VACUUM INTO` para backups rotativos.

## Consequências
- Projeto portátil e recuperável.
- Duas rotinas de migração.

## Alternativas consideradas
- Banco global único (projeto não portátil).
- Arquivo único compactado (salvamento mais lento, recuperação difícil).
