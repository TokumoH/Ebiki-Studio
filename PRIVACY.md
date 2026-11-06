# Privacidade

O Ebiki Studio **não coleta, não envia e não compartilha dados**.

| Item | Como funciona |
|---|---|
| Telemetria / analytics | Nenhuma |
| Conexões de rede | Nenhuma por padrão. A única exceção é a **verificação de atualizações**, e só quando você pede (*Ajuda › Verificar atualizações…*) ou liga *Verificar ao abrir o app* (desligado por padrão). Ela baixa o arquivo público `latest.json` da página de releases do GitHub e, se você aceitar, o instalador da nova versão. Nada é enviado além da requisição em si (o GitHub vê o seu IP, como em qualquer download). A interface roda com CSP que bloqueia qualquer origem remota |
| Contas / login | Não existem |
| Microfone / entradas de áudio | Usados só quando você grava ou liga o afinador. O áudio vai direto para arquivos no seu projeto; nada sai do computador. O sistema pede permissão (macOS) |
| Onde ficam os dados | Projetos: a pasta `Nome.ebiki` que você escolhe. Biblioteca (recentes, bases do usuário, progressões, preferências): pasta de dados do app `com.ebikistudio.studio` — veja o [guia](docs/manual/guia-instalacao-local.md#8-onde-ficam-os-dados). Preferências de interface (tema, layout, atalhos, "não mostrar o tutorial", verificação de atualizações) ficam no armazenamento local da janela do app, no mesmo computador |
| Arquivos importados | Copiados para dentro do projeto; os originais nunca são alterados |
| Apagar seus dados | Apague a pasta do projeto e a pasta de dados do app. Autosaves ficam em `Nome.ebiki/backups/` |

## Cuidados ao compartilhar

- Um projeto `.ebiki` contém **todos** os áudios gravados e importados (inclusive takes antigos
  ainda referenciados pelo histórico salvo). Revise antes de enviar a alguém.
- Nomes originais dos arquivos importados ficam registrados no projeto.
- Não versione projetos pessoais neste repositório: `*.ebiki/` e arquivos de áudio soltos estão no
  `.gitignore`.
