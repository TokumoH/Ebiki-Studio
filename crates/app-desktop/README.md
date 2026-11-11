# app-desktop

| Campo | Valor |
|---|---|
| Id | DIR-crates-app-desktop |
| Título | app-desktop |
| Tipo | composition root e adaptador de entrada (Tauri) |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `0c43cb9`; README por módulo em 2026-11-11 (ver `git log -- crates/app-desktop/README.md`) |
| Arquivos | 43 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Ebiki Studio — aplicativo desktop (Tauri): adaptador de entrada (comandos IPC) e composition root.

## Origem

Criado em `0c43cb9 2026-11-05 feat(app-desktop): app Tauri com comandos IPC, runtime de áudio (hardware ou nulo`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

`src/` (comandos por área, estado, áudio), `capabilities/` (permissões mínimas), `icons/` e `installer/` (marca), `tauri.conf.json`.

## Por que esta organização

Toda a composição num lugar: fácil ver o que cada comando expõe e com quais permissões.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Comandos IPC no app-core | Prenderia a aplicação ao Tauri |
| Vários binários | Um único ponto de composição é mais simples de auditar |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [Info.plist](Info.plist) | Permissões do macOS (uso do microfone) mescladas no app |
| [build.rs](build.rs) | Script de build (tauri-build: gera o contexto e os esquemas de permissões) |
| [tauri.conf.json](tauri.conf.json) | Dados/configuração JSON |
| [tauri.unsigned-updates.conf.json](tauri.unsigned-updates.conf.json) | Dados/configuração JSON |
| [capabilities/](capabilities/) | Subdiretório (coberto por este README) |
| [capabilities/default.json](capabilities/default.json) | Dados/configuração JSON |
| [icons/](icons/) | Subdiretório (coberto por este README) |
| [icons/128x128.png](icons/128x128.png) | Imagem |
| [icons/128x128@2x.png](icons/128x128@2x.png) | Imagem |
| [icons/32x32.png](icons/32x32.png) | Imagem |
| [icons/64x64.png](icons/64x64.png) | Imagem |
| [icons/Square107x107Logo.png](icons/Square107x107Logo.png) | Imagem |
| [icons/Square142x142Logo.png](icons/Square142x142Logo.png) | Imagem |
| [icons/Square150x150Logo.png](icons/Square150x150Logo.png) | Imagem |
| [icons/Square284x284Logo.png](icons/Square284x284Logo.png) | Imagem |
| [icons/Square30x30Logo.png](icons/Square30x30Logo.png) | Imagem |
| [icons/Square310x310Logo.png](icons/Square310x310Logo.png) | Imagem |
| [icons/Square44x44Logo.png](icons/Square44x44Logo.png) | Imagem |
| [icons/Square71x71Logo.png](icons/Square71x71Logo.png) | Imagem |
| [icons/Square89x89Logo.png](icons/Square89x89Logo.png) | Imagem |
| [icons/StoreLogo.png](icons/StoreLogo.png) | Imagem |
| [icons/icon.icns](icons/icon.icns) | Ícone (macOS) |
| [icons/icon.ico](icons/icon.ico) | Ícone (Windows) |
| [icons/icon.png](icons/icon.png) | Imagem |
| [icons/source.png](icons/source.png) | Imagem |
| [installer/](installer/) | Subdiretório (coberto por este README) |
| [installer/dmg-background.png](installer/dmg-background.png) | Imagem |
| [installer/nsis-header.bmp](installer/nsis-header.bmp) | Imagem do instalador (BMP) |
| [installer/nsis-sidebar.bmp](installer/nsis-sidebar.bmp) | Imagem do instalador (BMP) |
| [installer/wix-banner.bmp](installer/wix-banner.bmp) | Imagem do instalador (BMP) |
| [installer/wix-dialog.bmp](installer/wix-dialog.bmp) | Imagem do instalador (BMP) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/audio.rs](src/audio.rs) | Ciclo de vida do dispositivo de áudio: abre o hardware ou cai para o dispositivo nulo. |
| [src/commands.rs](src/commands.rs) | Comandos IPC (adaptador de entrada). Finos: validam/convertem DTOs e delegam ao 'Studio'. |
| [src/commands_composition.rs](src/commands_composition.rs) | Comandos IPC de composição (Fase 5 Harmonia + Fase 4 Bases) e do sintetizador. |
| [src/commands_edit.rs](src/commands_edit.rs) | Comandos IPC da Fase 3 — Edição / Pós-produção. Finos: validam argumentos e delegam aos |
| [src/commands_music.rs](src/commands_music.rs) | Comandos IPC das Fases 6–8 (partituras, letra, afinação, afinador, time-stretch). |
| [src/ipc_args.rs](src/ipc_args.rs) | Validação comum dos argumentos recebidos pela IPC (compartilhada por todos os módulos de comandos). |
| [src/ipc_tests.rs](src/ipc_tests.rs) | Testes de IPC com o runtime simulado do Tauri: os comandos são chamados com o MESMO JSON que a UI |
| [src/main.rs](src/main.rs) | Ebiki Studio — ponto de entrada do aplicativo desktop. |
| [src/state.rs](src/state.rs) | Estado global do app (composition root). |
| [src/updates.rs](src/updates.rs) | Atualização do app (ADR-0034). A verificação, o download e a validação da assinatura são do |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/app-desktop`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
