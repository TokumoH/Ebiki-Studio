# Procedimento de Varredura (obrigatório ao fim de cada grande trabalho)

**Objetivo:** procurar e corrigir erros de lógica, programação e design.
**Restrição:** a varredura só termina depois de verificar **todas** as páginas/telas e questões abertas.

## Checklist

| # | Verificação | Como verificar (mínimo) |
|---|---|---|
| 1 | Integridade arquitetural: GRASP Pure Fabrication, DDD, Componentes | Domínio sem I/O; dependências entre crates apontando para o domínio; contextos sem dependência cruzada; UI: contextos não se importam |
| 2 | Reutilização de funções, componentes e objetos | Procurar duplicação; extrair para shared-kernel / `ui/src/shared` |
| 3 | UX/UI e conforto (comparar com DAWs consolidadas) | Estouro de componentes/telas em resoluções pequenas; atalhos; contraste; estados vazios |
| 4 | SQL injection | Todas as queries parametrizadas; nenhuma concatenação de entrada em SQL |
| 5 | Falhas de carregamento e lotação de disco/render | Cache regenerável; limites de backups; tratamento de disco cheio; render sem travar |
| 6 | Velocidade, congelamento, alocação indevida | `assert_no_alloc` no callback; nada pesado na thread da UI; benchmarks |
| 7 | Falhas de funcionalidade | Fluxos ponta a ponta dos requisitos da fase |
| 8 | Erros de cálculo | Conversões dB/ganho, pan, tempo (tick↔amostra), arredondamentos, limites |
| 9 | Caminhos de API e tratamento de dados | Comandos IPC: validação de entrada, erros tipados, DTOs |
| 10 | Ambiente e caminhos seguros | Path traversal, CSP do Tauri, capabilities mínimas, links externos |
| 11 | Lógica de links/páginas conectadas | Navegação entre painéis e telas implementada |
| 12 | Dependências desatualizadas | `cargo outdated`/`npm outdated`; `cargo audit`/`npm audit` |
| 13 | Documentação atualizada e coerente | Specs, arquitetura, ADRs, manual batem com o código |
| 14 | Testes: integridade e qualidade | Todos passam; testes realmente verificam a função; substituir testes fracos |
| 15 | Correção | **Corrigir todos os problemas encontrados** |

## Saída

Relatório em `docs/relatorios/AAAA-MM-DD-varredura-<trabalho>.md` contendo:
- decisões tomadas;
- commits mais importantes;
- erros encontrados, explicados e corrigidos (principais em destaque, demais em resumo).
