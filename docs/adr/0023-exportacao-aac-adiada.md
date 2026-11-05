# ADR-0023: Exportação AAC/M4A adiada (importação suportada)

- **Data:** 2026-11-05
- **Status:** Aceita (modo autônomo)
- **Relacionados:** ADR-0022, spec `docs/specs/2026-11-05-fase-9-formatos.md`, RF-9.1, RF-9.2

## Contexto
RF-9.2 pede exportação AAC. Pesquisa de codificadores embutíveis em Rust (2026-11):

| Opção | Licença | Build | Problema |
|---|---|---|---|
| `fdk-aac` (bindings ao Fraunhofer FDK AAC) | "Software License for The Fraunhofer FDK AAC Codec Library" | `cc` | Licença não-OSI, incompatível com GPL, **exclui explicitamente licença de patentes**; distribuidores Linux removem o pacote; risco jurídico para distribuição |
| `ffmpeg` (`aac` nativo) via `ffmpeg-next` | LGPL/GPL | bibliotecas de sistema + `pkg-config` | dependência pesada de sistema nos 3 SOs; empacotamento complexo |
| `faac` | LGPL + código com restrições de patente (ISO) | `cc`/autotools | qualidade baixa; sem crate mantido |
| Media Foundation (Windows) / AudioToolbox (macOS) | APIs do SO | nativo | **não existe no Linux**; exigiria uma porta por plataforma |
| Codificador AAC em Rust puro | — | — | não existe crate maduro |

A **decodificação** AAC-LC e ALAC (M4A/MP4/ADTS) é feita pelo `symphonia` (MPL-2.0) e está suportada.

## Decisão
1. Nesta volta, `ExportFormat::Aac` existe no contrato, mas `Encoder::new` devolve
   `codec.unsupported_format` com mensagem PT-BR orientando a usar MP3, OGG, FLAC ou WAV
   (`ExportFormat::is_available()` = `false`, para a UI desabilitar a opção).
2. Importação de AAC-LC/ALAC (`.m4a`, `.mp4`, `.aac`) continua suportada.
3. Reavaliar quando houver porta de codificação nativa por SO (Media Foundation/AudioToolbox, com
   fallback "indisponível" no Linux) ou codificador livre maduro.

## Consequências
- Requisito RF-9.2 parcialmente atendido (AAC pendente) — registrado na spec como limitação.
- Nenhum risco jurídico de patentes/licença introduzido.
- O contrato IPC já prevê o formato; habilitar no futuro não quebra a UI.

## Alternativas consideradas
- Embutir `fdk-aac` atrás de *feature* desligada: mantém o código, mas incentiva builds com licença
  problemática; preferimos não incluir até haver decisão do usuário.
- Chamar um `ffmpeg` externo instalado pelo usuário: viola "sem dependências de sistema" e é frágil.
