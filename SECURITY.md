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
| Atualizações do app | Só sob demanda ou com opt-in; HTTPS para o GitHub Releases; cada pacote é **assinado** (minisign) na CI e verificado no app contra a chave pública embutida antes de instalar — arquivo adulterado é recusado (ADR-0034). A chave privada fica só nos segredos do repositório |
| Dependências | Dependabot, `cargo audit` e `npm audit` na CI, CodeQL |

## Para mantenedores — configurações do GitHub

Ative em **Settings → Code security**:
- [ ] Private vulnerability reporting
- [ ] Dependabot alerts e Dependabot security updates
- [ ] Secret scanning e **Push protection**
- [ ] Code scanning (o workflow `codeql.yml` já está no repositório)

Em **Settings → Secrets and variables → Actions**, os segredos `TAURI_SIGNING_PRIVATE_KEY` e
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` assinam as atualizações. Guarde uma cópia da chave e da senha
em um gerenciador de senhas: **sem elas, as versões instaladas não aceitam novas atualizações** e o
usuário precisa reinstalar manualmente. Se a chave vazar, gere outra e publique uma release
instalável manualmente com a nova chave pública.

Em **Settings → Branches**: proteja `main` (pull request obrigatório, CI verde, sem force push).
Na conta: autenticação em dois fatores.
