# ADR-0013: Ajustes de implementação da Fundação

- **Data:** 2026-11-05
- **Status:** Aceita (substitui partes das ADR-0006 e ADR-0009)

## Contexto
Ao implementar a Fase 0 algumas escolhas da arquitetura inicial se mostraram arriscadas ou desnecessárias.

## Decisão
1. **Migrações próprias** com `PRAGMA user_version` em vez de `refinery` (menos dependências, compatível com a versão atual do `rusqlite`; uma transação por versão; schema mais novo que o app é recusado).
2. **Contrato IPC por fixtures + zod** em vez de `tauri-specta` (que segue em release candidate): o Rust gera fixtures JSON (`crates/app-core/tests/contract_fixtures.rs`), a UI valida com esquemas `zod` (`ui/src/ipc/schema.ts`) e a CI falha se as fixtures mudarem sem commit. Além disso, `crates/app-desktop/src/ipc_tests.rs` chama os comandos reais com os payloads da UI.
3. **Camada `app-core`** dedicada à aplicação (Session, Studio, compilador sessão→grafo, portas) para que os casos de uso sejam testáveis sem Tauri.
4. **Áudio decodificado em memória** (`Arc<SourceBuffer>` compartilhado por hash) nesta versão; streaming de disco fica para uma volta futura da espiral (limite prático ≈ 1 h de áudio estéreo por projeto em máquinas de 8 GB).
5. **Lei de balanço** (centro = ganho unitário) para pan em canais estéreo.
6. **Herança de estado por chave** ao trocar o grafo (ADR-0005 refinada): todo ajuste recompila o grafo, mas processadores e suavizadores são transferidos pelo id do efeito/canal na thread de áudio, sem alocar.
7. **Ids UUID v7 em TEXT** gerados no domínio (estáveis em desfazer/refazer).

## Consequências
- Menos dependências instáveis; contrato verificado em três camadas (fixture, zod, IPC real).
- Projetos muito longos consomem RAM proporcional (documentado no manual).

## Alternativas consideradas
- `refinery` e `tauri-specta`: descartadas pelos motivos acima.
- Mensagens de parâmetro separadas da troca de grafo: mais rápidas, porém duplicariam o caminho de compilação.
