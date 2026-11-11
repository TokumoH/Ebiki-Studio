# app-core

| Campo | Valor |
|---|---|
| Id | DIR-crates-app-core |
| Título | app-core |
| Tipo | camada de aplicação |
| Data | 2026-11-11 |
| Usuário responsável | TokumoH |
| Status | ativo |
| Commits | origem: `e1704ca`; README por módulo em 2026-11-11 (ver `git log -- crates/app-core/README.md`) |
| Arquivos | 31 item(ns) — ver "Onde" (inclui as subpastas, que não têm README próprio) |

## Propósito

Camada de aplicação: sessão (agregados abertos), casos de uso, histórico e portas para infraestrutura.

## Origem

Criado em `e1704ca 2026-11-05 feat: ctx-recording, app-core (sessão e portas) e infra-persistence (project.db/l`. README por módulo adotado em 2026-11-11 ([CLAUDE.md §6](../../CLAUDE.md)): um README por crate e por camada da UI, cobrindo as subpastas.

## Organização

Serviços por área (`studio`, `edit_service`, `notation_service`…), `ports` (interfaces), `view` (DTOs) e testes de integração com adaptadores falsos em `tests/support`.

## Por que esta organização

Os casos de uso dependem só de portas: testáveis com adaptadores falsos e sem interface gráfica.

## Por que não outras organizações

| Opção | Motivo de descarte |
|---|---|
| Casos de uso nos comandos Tauri | Acoplaria a aplicação ao framework de UI e impediria testar sem Tauri |
| Infra chamada direto pelo domínio | Viola a arquitetura hexagonal (portas e adaptadores) |

## Onde

| Arquivo ou subdiretório | O que contém |
|---|---|
| [Cargo.toml](Cargo.toml) | Manifesto do crate (nome, dependências, features) |
| [src/](src/) | Subdiretório (coberto por este README) |
| [src/compile.rs](src/compile.rs) | Compilador: domínio (Session) → grafo de renderização da engine. |
| [src/device_monitor.rs](src/device_monitor.rs) | "Ouvir entrada" (monitor direto do dispositivo) — caso de uso da camada de aplicação. |
| [src/edit_service.rs](src/edit_service.rs) | Casos de uso da Fase 3 — Edição / Pós-produção (RF-3.1 a RF-3.5). |
| [src/lib.rs](src/lib.rs) | Camada de aplicação do Ebiki Studio. |
| [src/lyrics_service.rs](src/lyrics_service.rs) | Casos de uso de letra sincronizada (Fase 6): texto → sílabas, tempos por unidade, deslocamento, |
| [src/music_support.rs](src/music_support.rs) | Apoio comum aos casos de uso das Fases 6–8 (Pure Fabrication): acesso ao projeto aberto, |
| [src/notation_service.rs](src/notation_service.rs) | Casos de uso de notação (Fase 7): criar/editar/importar/exportar partituras, ouvir e renderizar |
| [src/ports.rs](src/ports.rs) | Portas (interfaces) implementadas pela infraestrutura. |
| [src/render.rs](src/render.rs) | Renderização offline (exportação, stems, análise) e medições. |
| [src/session.rs](src/session.rs) | Sessão: agregados de um projeto aberto e adaptação de comandos de cada contexto. |
| [src/studio.rs](src/studio.rs) | 'Studio': serviço de aplicação (Pure Fabrication / Controller do GRASP) que coordena casos de uso, |
| [src/tuning_service.rs](src/tuning_service.rs) | Casos de uso de afinação (Fase 8) e time-stretch (Fase 6). |
| [src/view.rs](src/view.rs) | DTOs de leitura para a UI (projeções da sessão; o núcleo continua sendo a fonte da verdade). |
| [src/composition/](src/composition/) | Subdiretório (coberto por este README) |
| [src/composition/bases.rs](src/composition/bases.rs) | Casos de uso de Bases (RF-4.1 a RF-4.5): busca, importação de áudio do usuário, geração |
| [src/composition/harmony.rs](src/composition/harmony.rs) | Casos de uso de Harmonia (RF-5.1 a RF-5.7): tons/modos, campo harmônico, sugestões, |
| [src/composition/library.rs](src/composition/library.rs) | Biblioteca de composição do usuário: agregado ['Library'] (progressões do usuário + catálogo de |
| [src/composition/mod.rs](src/composition/mod.rs) | Casos de uso de composição (Fase 5 Harmonia + Fase 4 Bases) na camada de aplicação. |
| [src/composition/studio_ops.rs](src/composition/studio_ops.rs) | Casos de uso de composição que tocam o projeto aberto ('impl Studio'): |
| [src/composition/wav.rs](src/composition/wav.rs) | Codificação WAV PCM 16 bits em memória (audição na UI via WebAudio, sem tocar o disco). |
| [tests/](tests/) | Subdiretório (coberto por este README) |
| [tests/composition.rs](tests/composition.rs) | Integração dos casos de uso de composição (Fase 5 Harmonia + Fase 4 Bases) com o 'Studio'. |
| [tests/contract_fixtures.rs](tests/contract_fixtures.rs) | Gera fixtures JSON do contrato IPC (fonte da verdade para os tipos/zod da UI). |
| [tests/device_monitor.rs](tests/device_monitor.rs) | "Ouvir entrada" e medidores de entrada pela camada de aplicação. |
| [tests/edit.rs](tests/edit.rs) | Fase 3 — edição ponta a ponta: casos de uso do 'Studio' → engine (render offline). |
| [tests/music.rs](tests/music.rs) | Integração das Fases 6–8 pelo 'Studio': notação, letra, afinação, time-stretch, afinador, |
| [tests/studio.rs](tests/studio.rs) | Simulação ponta a ponta do caso de uso principal (Fase 0 + gravação). |
| [tests/support/](tests/support/) | Subdiretório (coberto por este README) |
| [tests/support/mod.rs](tests/support/mod.rs) | Dublês de teste para as portas (mídia WAV simples, fábrica de ganho, relógio controlável). |

## Lógica

As dependências entre módulos seguem a arquitetura do CLAUDE.md (§2): contextos só dependem do shared-kernel; serviços técnicos não conhecem o domínio de outros contextos; a aplicação orquestra pelas portas; o app desktop compõe tudo. Ao adicionar, remover ou mover arquivos deste módulo, atualize "Onde" no mesmo commit (o script `scripts/check_readmes.py` confere).

## Exemplo

Para achar onde algo está implementado: procure na tabela "Onde" (descrições vêm do comentário de cabeçalho de cada arquivo) ou rode `git log --oneline -- crates/app-core`.

## Verificação

Gerado por script e conferido por `scripts/check_readmes.py`: todo arquivo do módulo e das subpastas está em "Onde", nenhum item listado inexiste, links relativos abrem.

## Riscos

As descrições dependem dos comentários de cabeçalho dos arquivos; arquivo sem cabeçalho aparece com descrição genérica.
