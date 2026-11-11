# app

| Campo | Valor |
|---|---|
| Id | DIR-ui-src-app |
| Título | app |
| Tipo | shell do aplicativo |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `269e11d`; README por módulo em 2026-11-11 (ver `git log -- ui/src/app/README.md`) |
| Arquivos | 29 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Tela inicial, workspace, menu principal, atalhos, central de ajuda, tutorial, legendas, atualizações, diálogos de nível do app e tela de carregamento.

## Origem

Criado em `269e11d 2026-11-05 feat(ui): interface React — tela inicial, workspace com painéis encaixáveis, arra`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Um arquivo por tela/diálogo; modelo do menu e glossário de atalhos como funções puras com testes.

## Por que esta organização

Camadas com direção única de dependência (app → panels → contexts → core/ipc/shared) mantêm componentes independentes; o teste `architecture.test.ts` impede violações.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Organizar por tipo de arquivo (components/, hooks/…) | Espalharia cada contexto por várias pastas e esconderia as dependências |
| Tudo em `src/` | Sem fronteiras, o acoplamento cresce sem ser notado |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [AboutDialog.tsx](AboutDialog.tsx) | Sobre: marca, versão, licença e privacidade em uma janela curta. |
| [App.tsx](App.tsx) | Raiz da interface: tela inicial ou workspace, tema, telemetria, avisos e camada de legendas. |
| [AudioSettingsDialog.tsx](AudioSettingsDialog.tsx) | ao abrir: lista os dispositivos e mostra a configuração ATUAL ("" = padrão do sistema / nenhuma) |
| [CloseProjectDialog.test.tsx](CloseProjectDialog.test.tsx) | Transporte simulado em que salvar pode falhar sob demanda. */ |
| [CloseProjectDialog.tsx](CloseProjectDialog.tsx) | falha ao salvar (ou ao fechar): mantém o diálogo aberto; o erro já aparece como aviso |
| [ExportDialog.tsx](ExportDialog.tsx) | Diálogo de exportação: formato, resolução/qualidade, intervalo, normalização e stems. |
| [HelpCenter.tsx](HelpCenter.tsx) | Central de ajuda (F1): busca, tópicos do manual, atalhos de teclado e acesso ao tutorial. |
| [ShortcutsDialog.tsx](ShortcutsDialog.tsx) | Diálogo de atalhos de teclado: lista e edita os atalhos (gravados no armazenamento local). |
| [StartScreen.tsx](StartScreen.tsx) | Tela inicial: novo projeto, abrir e recentes (com verificação de recuperação de autosave). */ |
| [Toasts.tsx](Toasts.tsx) | Avisos temporários (sucesso/erro) no canto da tela. |
| [Tutorial.test.tsx](Tutorial.test.tsx) | Testes: posição do cartão do tutorial; abaixo do alvo quando cabe; acima quando não cabe; centralizado sem alvo |
| [Tutorial.tsx](Tutorial.tsx) | Tutorial de início pulável: destaca áreas reais da interface (seletores 'data-tour') em 7 passos. |
| [UpdateDialog.test.tsx](UpdateDialog.test.tsx) | Testes: plainNotes; usa só a seção Novidades e remove a marcação |
| [UpdateDialog.tsx](UpdateDialog.tsx) | Diálogo de atualização (mockup "6 · Atualizações"): nova versão, download com progresso, |
| [Workspace.tsx](Workspace.tsx) | só o número de falhas de buffer: assinar o objeto de medidores re-renderizava o Workspace ~30x/s |
| [about.ts](about.ts) | Versão exibida na interface (mantida igual ao tauri.conf.json; conferida por teste). |
| [app.css](app.css) | ---- central de ajuda ---- |
| [commands.test.ts](commands.test.ts) | Ligações do menu e dos atalhos: todo item do menu e todo atalho padrão precisa de uma ação no |
| [helpTopics.test.ts](helpTopics.test.ts) | Testes: busca da central de ajuda; ignora acentos e maiúsculas |
| [helpTopics.ts](helpTopics.ts) | Conteúdo da central de ajuda (resumo prático do manual) e busca sem acentos. |
| [menuModel.test.ts](menuModel.test.ts) | tema: oferece o oposto do atual |
| [menuModel.ts](menuModel.ts) | Modelo do menu principal: função pura do estado da sessão + ações, para a MenuBar desenhar. |
| [shortcuts.test.ts](shortcuts.test.ts) | Testes: normaliza combinações; não dispara atalho ao digitar |
| [shortcuts.ts](shortcuts.ts) | Atalhos de teclado padrão de DAW. Ignorados quando o foco está em campos de texto. |
| [splash.ts](splash.ts) | Controle da tela de carregamento definida em index.html (fica fora do React para aparecer já). |
| [tooltips.test.tsx](tooltips.test.tsx) | chave inexistente não aparece crua para o usuário |
| [tooltips.ts](tooltips.ts) | Resolvedor das legendas do app: liga a camada genérica (shared/TooltipLayer) ao glossário (core). |
| [updates.test.ts](updates.test.ts) | Testes: isNewer; compara versões numericamente, não como texto |
| [updates.ts](updates.ts) | Fluxo de atualização (ADR-0034): verificação manual (Ajuda › Verificar atualizações) ou ao abrir, |

## Lógica

Regras de dependência da UI (testadas em `src/architecture.test.ts`): `shared` não importa nada do app; `ipc`/`core` não importam apresentação; contextos não importam outros contextos; só `ipc` conhece o Tauri. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- ui/src/app`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
