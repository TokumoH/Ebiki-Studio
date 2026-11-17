# verificacoes

| Campo | Valor |
|---|---|
| Id | DIR-docs-verificacoes |
| Título | verificacoes |
| Tipo | documentação de processo (registros de commit) |
| Data | 2026-11-10 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | commit que adotou o CORE de 2026-11-10 (ver `git log -- docs/verificacoes`) |
| Arquivos | `README.md`, `_modelo-registro-de-correcao.md`, `VER-*.md` |

## Propósito

Guarda um **registro por commit**: o que mudou, por quê, alternativas descartadas, onde, como foi
verificado (duas verificações seguidas sem erro) e riscos. É a trilha de auditoria das decisões do
projeto, complementar aos ADRs (decisões de arquitetura) e aos relatórios de varredura.

## Origem

Criado ao incorporar o *Prompt CORE* do usuário ao [`CLAUDE.md`](../../CLAUDE.md) (§6), em
2026-11-10: "todo commit leva documento em `docs/verificacoes/`".

## Organização

- `_modelo-registro-de-correcao.md`: modelo obrigatório (o `_` o mantém no topo da listagem).
- `VER-AAAA-MM-DD-NN-assunto.md`: um registro por commit; `NN` é a sequência do dia.
- Sem subdiretórios.

## Por que esta organização

Nome com data e sequência ordena cronologicamente e é único; o assunto no nome facilita a busca.
Um arquivo por commit casa 1:1 com o histórico do git.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Registrar só na mensagem do commit | Não comporta o formato completo (tabelas, verificação, riscos) nem links |
| Um arquivo por mês | Conflitos de edição e registros difíceis de citar individualmente |
| Misturar com `docs/relatorios/` | Relatórios são de varredura (vários commits); registros são por commit |

## Onde

| Arquivo | O que contém |
|---|---|
| [_modelo-registro-de-correcao.md](_modelo-registro-de-correcao.md) | Modelo do registro |
| [VER-2026-11-10-01-core-no-claude-md.md](VER-2026-11-10-01-core-no-claude-md.md) | Adoção do CORE no CLAUDE.md e criação deste diretório |
| [VER-2026-11-10-02-legendas.md](VER-2026-11-10-02-legendas.md) | Legendas ao passar o mouse (glossário + camada única) |
| [VER-2026-11-10-03-zoom-pela-roda.md](VER-2026-11-10-03-zoom-pela-roda.md) | Zoom pela roda, altura das faixas e camadas acima do dockview |
| [VER-2026-11-10-04-loop-configuravel.md](VER-2026-11-10-04-loop-configuravel.md) | Loop configurável (opções, projeto inteiro, régua) |
| [VER-2026-11-10-05-menu-da-logo-e-entrada-recolhivel.md](VER-2026-11-10-05-menu-da-logo-e-entrada-recolhivel.md) | Menu da logo e entrada de áudio recolhível |
| [VER-2026-11-10-06-compassos-do-mapa-de-tempo.md](VER-2026-11-10-06-compassos-do-mapa-de-tempo.md) | Mapa de tempo com a mesma numeração de compassos da régua |
| [VER-2026-11-11-01-varredura-4.md](VER-2026-11-11-01-varredura-4.md) | Correções da varredura 4 (popovers, renders, caches, CI, docs) |
| [VER-2026-11-11-02-readme-por-modulo.md](VER-2026-11-11-02-readme-por-modulo.md) | README por módulo e verificador automático |
| [VER-2026-11-11-03-release-v0-3-0.md](VER-2026-11-11-03-release-v0-3-0.md) | Versão 0.3.0 (versão e notas da release) |
| [VER-2026-11-16-01-varredura-5-ux.md](VER-2026-11-16-01-varredura-5-ux.md) | Varredura 5 (UX): diagnósticos e sugestões aguardando aprovação |
| [VER-2026-11-16-02-audio-microfone-fone.md](VER-2026-11-16-02-audio-microfone-fone.md) | S1: Configurações de áudio fone × microfone, ganho de entrada 0..+24 dB, Testar som (−12 dBFS) e retorno em 0 dB |
| [VER-2026-11-16-03-seguir-reproducao-e-espaco.md](VER-2026-11-16-03-seguir-reproducao-e-espaco.md) | Seguir reprodução (S2) e Espaço após clique (UX-01) |
| [VER-2026-11-17-01-ux-barra-dock-dialogos.md](VER-2026-11-17-01-ux-barra-dock-dialogos.md) | Barra superior, dock e diálogos em telas pequenas (UX-02, UX-03, UX-07) |
| [VER-2026-11-17-02-ux-primeiro-uso-e-avisos.md](VER-2026-11-17-02-ux-primeiro-uso-e-avisos.md) | Primeiro uso e avisos (UX-09, UX-12, UX-05, UX-17, UX-21) |
| [VER-2026-11-17-03-ux-acessibilidade.md](VER-2026-11-17-03-ux-acessibilidade.md) | Acessibilidade: foco nos diálogos, contraste AA, tutorial (UX-06, UX-10, UX-11) |
| [VER-2026-11-17-04-ux-mixer-e-cursor.md](VER-2026-11-17-04-ux-mixer-e-cursor.md) | M/S do Mixer à vista e clique posiciona o cursor (UX-04, UX-08) |
| [VER-2026-11-17-05-ux-atalhos-numeros-dock.md](VER-2026-11-17-05-ux-atalhos-numeros-dock.md) | Atalhos, números e dock em PT-BR (UX-13, UX-14, UX-15, UX-16) |
| [VER-2026-11-17-06-ux-exportar-bpm-faixa-nomes.md](VER-2026-11-17-06-ux-exportar-bpm-faixa-nomes.md) | Exportar trecho, BPM editável, remover faixa com desfazer e nomes (UX-19, UX-20, UX-22, UX-23) |
| [VER-2026-11-17-07-integracao-s1-status-e-seguir.md](VER-2026-11-17-07-integracao-s1-status-e-seguir.md) | Integração do S1, barra de status (UX-18) e corrida do seguir |
| [VER-2026-11-17-08-release-v0-4-0.md](VER-2026-11-17-08-release-v0-4-0.md) | Versão 0.4.0 (versão e notas da release) |
| [VER-2026-11-17-09-mostrar-na-pasta.md](VER-2026-11-17-09-mostrar-na-pasta.md) | "Mostrar na pasta" após exportar (ADR-0037) |
| [VER-2026-11-17-10-release-sem-corrida.md](VER-2026-11-17-10-release-sem-corrida.md) | Release sem corrida entre os sistemas (v0.4.0 duplicada) |

## Lógica

1. Antes do commit: dupla verificação (testes, simulações, sabotagens).
2. Copiar o modelo para `VER-…-NN-assunto.md` e preencher.
3. Commitar o registro **junto** com a mudança; atualizar a tabela "Onde" deste README no mesmo commit.
4. ADRs citados no registro explicam decisões de arquitetura; o registro explica o commit.

## Exemplo

Para saber por que o `CLAUDE.md` mudou em 2026-11-10: abrir
[VER-2026-11-10-01](VER-2026-11-10-01-core-no-claude-md.md) ou rodar
`git log -- docs/verificacoes/VER-2026-11-10-01-core-no-claude-md.md`.

## Verificação

Links relativos conferidos por script (todos existem); todo arquivo do diretório está em "Onde";
nenhum item listado inexistente; sem subdiretórios.

## Riscos

Commits anteriores a 2026-11-10 não têm registro aqui (estão nos relatórios de varredura e ADRs).
