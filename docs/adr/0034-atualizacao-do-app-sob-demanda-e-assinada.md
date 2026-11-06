# ADR-0034: Atualização do app sob demanda e assinada

- **Status:** Aceita (revê o item "Auto-update" de *Fora do escopo* nos requisitos)
- **Data:** 2026-11-06
- **Contexto delimitado:** UI (`ipc`, `app`) e empacotamento (`app-desktop`, release)

## Contexto

O usuário pediu atualização de software. Ela colide com duas decisões: auto-update estava fora do
escopo e a política de privacidade prometia nenhuma conexão de rede. Decisões do usuário:
**verificação manual + opt-in ao abrir** e **instalação dentro do app**.

## Decisão

1. **Quando verificar:** *Ajuda › Verificar atualizações…* ou, se o usuário ligar, ao abrir o app
   (**desligado por padrão**). A verificação automática nunca abre diálogo: só acende um aviso na
   barra de status, e falhas dela são silenciosas.
2. **Como instalar:** `tauri-plugin-updater` baixa o pacote indicado em
   `releases/latest/download/latest.json`, **verifica a assinatura minisign** contra a chave pública
   do `tauri.conf.json` e instala; `tauri-plugin-process` reinicia. O projeto é salvo antes, se houver
   alterações. Windows usa o instalador NSIS em modo passivo.
3. **Onde não dá para instalar:** `.deb`/`.rpm` pertencem ao gerenciador de pacotes. O comando
   `app_update_mode` (Rust, função pura testada) devolve `manual` fora de Windows/macOS/AppImage, e a UI
   abre a página de releases (`tauri-plugin-opener`, permissão restrita a essa URL).
4. **Arquitetura:** a UI conhece só a porta `ipc/updater.ts` (`Updater`); o adaptador real
   (`updaterTauri.ts`) e o mock (cenários por `localStorage`, usado no E2E) são Pure Fabrication.
   O fluxo (estado, versão pulada, preferências) é o store `app/updates.ts`, sem dependência de Tauri.
5. **Release:** a CI assina com os segredos `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)` e o
   `tauri-action` publica `latest.json` e os `.sig`. Sem o segredo, a release sai normalmente com
   `tauri.unsigned-updates.conf.json` (sem pacotes de atualização) e um aviso no log.

## Alternativas

| Opção | Por que não |
|---|---|
| Verificar sempre ao abrir (padrão ligado) | Quebra a promessa offline sem consentimento |
| Só avisar e abrir a página | Mais passos para o usuário; escolhido apenas onde a instalação no app é impossível (.deb/.rpm) |
| Servidor próprio de atualizações | Infraestrutura e custo; GitHub Releases já hospeda os instaladores |
| Atualizar pacotes .deb/.rpm pelo app | Exigiria root e brigaria com o gerenciador de pacotes |

## Consequências

- PRIVACY.md, SECURITY.md e README descrevem a única conexão (opcional) e a assinatura.
- **Perder a chave privada impede atualizar as instalações existentes** (precisariam reinstalar):
  guardar chave e senha fora do GitHub também.
- Releases até a 0.1.0 não têm o atualizador; a 0.2.0 é a primeira que se atualiza sozinha.
- O webview continua sem acesso à rede: o download acontece no Rust, e a CSP não muda.
