# ADR-0026: Biblioteca de composição — persistência, histórico próprio e decisão explícita ao inserir bases

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Relacionadas:** ADR-0006 (dois SQLite), ADR-0008 (Command), ADR-0011 (portas), ADR-0016 (WSOLA),
  ADR-0019, ADR-0025

## Contexto
Progressões do usuário e bases são dados **globais** (não pertencem a um projeto) e vivem no
`library.db`. O catálogo embutido e as bases CC0 mudam com versões do app, sem poder apagar dados do
usuário. Ao inserir uma base com BPM/tom diferentes do projeto, o requisito RF-4.5 exige que o usuário
**sempre decida**. Não há áudios CC0 no repositório ainda.

## Decisão
1. **Esquema:** `ctx_harmony::LIBRARY_SCHEMA_V1` entra como **migração 2** do `library.db` (a 1 já
   tinha recentes/preferências). `LibraryDb::open` semeia vocabulário (sempre, *upsert* por código) e
   progressões embutidas (só quando `seed_meta['harmony_catalog_version'] < CATALOG_VERSION`; apaga e
   regrava **apenas** as embutidas). Bases CC0 vêm de `assets/bases/manifest.json` (embutido no binário,
   licença obrigatória `CC0-1.0`, versão em `seed_meta['bundled_bases_version']`); bases **iniciais**
   (receitas geradas, ids estáveis) são semeadas uma vez por `starter_bases_version` e depois pertencem
   ao usuário (podem ser removidas sem voltar).
2. **Porta `LibraryStore`** (app-core) com `load`/`save` do agregado `Library { harmony, bases }`;
   implementada por `LibraryDb` (salvar = transação que regrava progressões do usuário e bases não CC0,
   preservando a ordem). Fallback em memória se o banco não abrir.
3. **Histórico próprio:** o serviço `Composer` mantém um `History<Library>` separado do projeto
   (`library_undo`/`library_redo`). Cada comando é gravado logo após aplicado; se a gravação falhar, é
   desfeito e o erro volta à UI (estado em memória = estado em disco).
4. **Bases de áudio do usuário são copiadas** para `<dados do app>/bases/<id>.<ext>` (o original pode
   sumir; caminho relativo validado por `AudioPath`). CC0: `<recursos>/assets/bases/<file>`.
5. **Receita canônica:** a progressão de uma base gerada é gravada em graus no modo **maior/menor da
   base** (cada acorde analisado com `RomanChord::analyze`); a gravação só acontece se a ida e volta
   reproduzir exatamente os mesmos acordes (senão erro tipado).
6. **Inserir base:** `Studio::insert_base` calcula `MismatchReport` (BPM do projeto no tick de destino;
   tom do projeto informado pela UI, pois o projeto não guarda tom). Com divergência e **sem**
   `decision`, retorna `NeedsDecision { report }` **sem alterar nada**. `Keep` insere o original (áudio
   importado para `audio/imported`). `Adapt`: base de **áudio** → `dsp_pitch::time_stretch` (razão
   `bpm_base / bpm_projeto`, limite 0,25–4) + `pitch_shift_offline` (±6 semitons); base **gerada** →
   re-síntese no tempo do projeto e no tom transposto (sem artefatos de DSP).

## Consequências
- Atualizar o catálogo nunca toca dados do usuário (testado com troca de versão).
- O usuário tem dois "desfazer": o do projeto e o da biblioteca — a UI deve mostrá-los em contextos
  distintos.
- Arquivos copiados de bases removidas ficam em disco (para permitir desfazer); limpeza de órfãos fica
  para uma volta futura.

## Alternativas consideradas
- Referenciar o arquivo original da base do usuário: quebra quando o usuário move/apaga o arquivo.
- Decisão implícita (ajustar sempre): viola RF-4.5.
- Um único histórico global (projeto + biblioteca): desfazer no projeto apagaria progressões cadastradas.
