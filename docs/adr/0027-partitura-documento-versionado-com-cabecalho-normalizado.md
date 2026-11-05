# ADR-0027: Partitura no `project.db` como documento versionado com cabeçalho normalizado

- **Data:** 2026-11-05
- **Status:** Aceita

## Contexto
A volta 2 da integração (Fases 6–8) persiste no `project.db` (migração v2) a letra, a afinação e as
partituras. A regra do projeto (CLAUDE.md §4) manda aplicar a **maior forma normal possível** e
preservar a **leitura eficiente**; desnormalização só com justificativa registrada.

Letra e afinação são rasas e de forma fixa (linha → palavra → sílaba; mapa → escala/nota → edição):
foram totalmente normalizadas (tabelas `lyric_*`, `pitch_correction_map`, `correction_scale_pitch`,
`detected_note`, `note_edit`).

A partitura (`ctx_notation::Score`, ADR-0021) é diferente:

- **Profundidade 8** (partitura → parte → pauta/compasso → voz → evento → nota → técnica, mais spanners
  que referenciam eventos por id e afinação de tablatura por pauta).
- **Polimorfismo**: `EventKind` (pausa | acorde), `Technique` com parâmetros (`Bend{cents}`,
  `Harmonic{kind}`), quiálteras/pontos opcionais, apojatura, letra por evento, repetições opcionais.
- **Invariantes do agregado inteiro** (`check_invariants`: compassos iguais em todas as partes, ids
  únicos, spanners válidos, durações exatas por frações) — só verificáveis com a partitura completa.
- **Padrão de acesso**: o núcleo carrega e salva o agregado inteiro (o editor trabalha em memória e
  envia o `Score` completo ao alphaTab); nenhuma consulta SQL precisa de "todas as notas Dó#" de um
  projeto. A listagem precisa só de título, compositor e partes.
- O modelo de domínio ainda vai evoluir (casas de 1ª/2ª vez, D.C./D.S., cifras — spec da Fase 7 §9).

## Alternativas
1. **Normalização total** (~14 tabelas: `score`, `part`, `staff`, `tab_tuning`, `measure`,
   `measure_attr`, `voice`, `event`, `event_articulation`, `note`, `note_technique`, `lyric`,
   `spanner`, `tempo_event`). Prós: integridade referencial por linha, consultas analíticas. Contras:
   reconstrução com 10+ consultas e ordenação por posição em cada nível; polimorfismo vira colunas
   anuláveis ou tabelas por subtipo; *cada* evolução do domínio exige migração de schema; o salvamento
   (regravação total na transação, padrão do repositório) passa a inserir dezenas de milhares de linhas
   por partitura (uma partitura orquestral de 300 compassos tem >50 mil notas); invariantes continuam
   precisando do agregado inteiro em memória.
2. **Documento JSON inteiro** numa coluna. Prós: simples, rápido. Contras: título/partes só
   consultáveis desserializando; tudo opaco ao SQL.
3. **Híbrido (escolhida)**: cabeçalho normalizado + corpo como documento versionado **sem os dados do
   cabeçalho**.

## Decisão
- `score(id PK, position UNIQUE, title, composer, doc_version, body CHECK(json_valid(body)))` e
  `score_part(score_id FK, position, name, program 0–127, percussion)`, mais a projeção
  `v_score_summary` para a listagem (sem desserializar corpos).
- `body` = `Score` serializado (serde, camelCase) **menos** `title`, `composer` e, em cada parte,
  `name`, `program`, `percussion`. Na carga, as colunas são reinseridas no documento e o agregado é
  desserializado e validado (`check_invariants`); divergência entre nº de partes do cabeçalho e do
  corpo, versão desconhecida ou JSON inválido → `persistence.corrupt`.
- `doc_version` (atual = 1) permite migrar o corpo em código quando o modelo mudar, sem migração de
  schema (o migrador lê a versão e converte antes de desserializar).

### Por que isto respeita a regra de normalização
- **1FN**: do ponto de vista do SGBD, `body` é um valor atômico de um domínio próprio
  ("documento de corpo de partitura v1"): o banco nunca o decompõe, filtra ou indexa por dentro.
- **Sem redundância**: nenhum fato é armazenado duas vezes. Título, compositor e dados das partes vivem
  **só** nas colunas (teste `score_header_lives_in_columns_not_in_the_document` altera o nome da parte
  só na coluna e verifica que a carga reflete a mudança). Não há anomalia de atualização entre
  cabeçalho e corpo.
- **2FN/3FN/FNBC** nas tabelas de cabeçalho: chaves `id` e `(score_id, position)`; todos os atributos
  dependem da chave inteira e de nada mais.
- A decomposição do corpo em tabelas não traria consulta útil (padrão de acesso = agregado inteiro) e
  pioraria a leitura eficiente, que é a outra metade da regra.

## Consequências
- Leitura: 2 consultas por partitura (cabeçalho + partes) e 1 `serde_json::from_value`.
- Integridade interna do corpo é garantida pelo domínio na carga (não por FKs).
- Consultas analíticas dentro da partitura (se um dia forem necessárias) devem ser feitas em memória
  ou via projeções materializadas derivadas do corpo — nunca duplicando a fonte da verdade.
- Testes: `infra-persistence/tests/music_roundtrip.rs` (identidade salvar → carregar com partitura
  rica, cabeçalho só nas colunas, documento com versão inválida rejeitado) e
  `music_tables::tests::v1_database_is_migrated_to_v2_and_loads_with_empty_music_aggregates`.
