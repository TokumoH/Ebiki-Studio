# VER-2026-11-16-01 — Varredura 5 (UX) e diagnósticos

| Campo | Valor |
|---|---|
| Id | VER-2026-11-16-01 |
| Título | Varredura priorizando UX; diagnóstico de áudio baixo e da vista que não segue a reprodução |
| Gravidade | alta (UX-01, UX-02, vista não segue) · média · baixa |
| Tipo | varredura |
| Data | 2026-11-16 |
| Usuário responsável | TokumoH |
| Status | somente documentação (sugestões S1–S3 aguardando aprovação; mockup 11) |
| Commits | este commit |
| Arquivos | `docs/relatorios/2026-11-16-varredura-5-ux.md`, `docs/relatorios/README.md`, este registro, `docs/verificacoes/README.md` |

## Problema

O usuário relatou: áudio baixo; configurações sem deixar claro o que é microfone e o que é fone; a vista não acompanha
a linha de reprodução. Também pediu uma varredura priorizando UX.

## Era proposital?

- **Seguir reprodução:** não. Não há código, ADR, especificação nem commit sobre isso; nunca foi implementado.
- **Monitor a −6 dB:** escolha da entrega da 0.2.1 (proteção contra microfonia), sem registro de que deveria soar baixo.
- **Rótulos "Saída/Entrada":** sem decisão registrada; vêm da Fase 1.

## Solução

Somente documentação: relatório com os diagnósticos, as sugestões S1 (diálogo microfone × fone com ganho e teste de som),
S2 (seguir reprodução) e S3 (23 achados de UX priorizados), e mockup 11 para aprovação. Nenhum código foi alterado.

## Por que esta solução

O CLAUDE.md (§3) exige sugestão e mockup antes de mudanças visuais e de pedidos novos. Os diagnósticos ficam registrados
para a implementação.

## Por que não outras soluções

| Opção | Motivo de descarte |
|---|---|
| Implementar direto | Mudança visual e de comportamento sem aprovação, contra a §3 |
| Mexer no ganho do motor "às cegas" | A medição mostrou que a reprodução não perde volume; um ganho extra mascararia a causa |

## Onde

| Arquivo | Linhas | O que mudou |
|---|---|---|
| [docs/relatorios/2026-11-16-varredura-5-ux.md](../relatorios/2026-11-16-varredura-5-ux.md) | todo | Relatório e anexo com os 23 achados |
| [docs/relatorios/README.md](../relatorios/README.md) | Onde | Nova linha |

## Lógica

Pedido → diagnóstico com dupla verificação → sugestões no formato §5 + mockup → aprovação → implementação com TDD
e novo registro.

## Exemplo

Tom de amplitude 0,5 (−6,02 dBFS) importado e exportado sem normalizar sai com pico 0,5000 nos dois canais: o volume
não cai no app.

## Verificação

- **Verificação 1:** Playwright/Chromium no modo simulado em 5 tamanhos de janela e nos 2 temas; leitura de código;
  teste temporário de ganho de ponta a ponta (`infra-adapters`, `full_chain`), removido depois.
- **Verificação 2 (outra abordagem):** conferência no código de UX-01, UX-03, UX-11, UX-12 e do monitor a −6 dB, todos
  confirmados; revisão do caminho de ganho do motor (balanço, master, saída, gravação em float 32).
- `scripts/check_readmes.py` sem pendências.

## Riscos

- Sem hardware de áudio no ambiente: o volume baixo no aparelho do usuário não foi reproduzido. A causa exata depende de
  qual áudio está baixo (pergunta feita ao usuário).
- Achados "a confirmar" do anexo não foram reproduzidos.
