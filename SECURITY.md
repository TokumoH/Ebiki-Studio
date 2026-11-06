# Política de segurança

## Como reportar uma vulnerabilidade

**Não abra issue pública** para falhas de segurança.

Use o reporte privado do GitHub: aba **Security → Report a vulnerability** deste repositório.
Inclua passos para reproduzir, versão/commit afetado, sistema operacional e impacto esperado.

- Confirmação de recebimento: em até 7 dias.
- Correção: priorizada pela gravidade; o crédito é dado a quem reportou (se desejar).

## Versões suportadas

| Versão | Suporte |
|---|---|
| Branch de desenvolvimento (`branchTrabalho`) e última release | ✅ |
| Releases anteriores | ❌ |

## Modelo de ameaça (resumo)

O Ebiki Studio é um app **local e offline**. Superfícies relevantes:

| Superfície | Proteção |
|---|---|
| Arquivos abertos pelo usuário (áudio, MIDI, MusicXML, Guitar Pro, SoundFont, projetos) | Parsers com limites de tamanho, sem entidades XML externas (anti-XXE/"billion laughs"), erros tipados em vez de pânico, testes de propriedade (proptest) |
| Projeto `.ebiki` de terceiros | Caminhos relativos validados (`safe_join`, sem `..`), banco SQLite só com consultas parametrizadas |
| Webview (interface) | CSP estrita sem origens remotas, `freezePrototype`, capacidades Tauri mínimas (sem acesso direto a arquivos ou shell) |
| Comandos IPC | Argumentos validados (UUIDs, números finitos, caminhos absolutos, extensões de saída por formato) |
| Thread de áudio | Sem alocação/bloqueio; pânico em efeito vira silêncio, não derruba o app |
| Dependências | Dependabot, `cargo audit` e `npm audit` na CI, CodeQL |

## Para mantenedores — configurações do GitHub

Ative em **Settings → Code security**:
- [ ] Private vulnerability reporting
- [ ] Dependabot alerts e Dependabot security updates
- [ ] Secret scanning e **Push protection**
- [ ] Code scanning (o workflow `codeql.yml` já está no repositório)

Em **Settings → Branches**: proteja `main` (pull request obrigatório, CI verde, sem force push).
Na conta: autenticação em dois fatores.
