# Privacidade

O Ebiki Studio **não coleta, não envia e não compartilha dados**.

| Item | Como funciona |
|---|---|
| Telemetria / analytics | Nenhuma |
| Conexões de rede | Nenhuma. A interface roda com CSP que bloqueia qualquer origem remota; não há atualização automática |
| Contas / login | Não existem |
| Microfone / entradas de áudio | Usados só quando você grava ou liga o afinador. O áudio vai direto para arquivos no seu projeto; nada sai do computador. O sistema pede permissão (macOS) |
| Onde ficam os dados | Projetos: a pasta `Nome.ebiki` que você escolhe. Biblioteca (recentes, bases do usuário, progressões, preferências): pasta de dados do app `com.ebikistudio.studio` — veja o [guia](docs/manual/guia-instalacao-local.md#8-onde-ficam-os-dados) |
| Arquivos importados | Copiados para dentro do projeto; os originais nunca são alterados |
| Apagar seus dados | Apague a pasta do projeto e a pasta de dados do app. Autosaves ficam em `Nome.ebiki/backups/` |

## Cuidados ao compartilhar

- Um projeto `.ebiki` contém **todos** os áudios gravados e importados (inclusive takes antigos
  ainda referenciados pelo histórico salvo). Revise antes de enviar a alguém.
- Nomes originais dos arquivos importados ficam registrados no projeto.
- Não versione projetos pessoais neste repositório: `*.ebiki/` e arquivos de áudio soltos estão no
  `.gitignore`.
