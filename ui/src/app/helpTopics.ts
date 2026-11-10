// Conteúdo da central de ajuda (resumo prático do manual) e busca sem acentos.
// O tópico "Atalhos de teclado" é gerado a partir dos atalhos configurados.

export interface HelpTopic {
  id: string;
  title: string;
  /** Termos extras para a busca (sinônimos, nomes de recursos). */
  keywords: string;
  intro: string;
  steps: string[];
  /** Passo do tutorial que mostra a área correspondente no app (1-based). */
  tourStep?: number;
}

export const HELP_TOPICS: HelpTopic[] = [
  {
    id: "inicio",
    title: "Primeiros passos",
    keywords: "começar novo projeto abrir salvar autosave recuperar pasta ebiki",
    intro: "Um projeto é uma pasta Nome.ebiki com tudo dentro: banco do projeto, áudios e autosaves. Pode ser copiada para outro computador.",
    steps: [
      "Na tela inicial, escolha nome, pasta, taxa de amostragem (48 kHz) e resolução (24 bits) e clique em Criar projeto.",
      "Arquivo › Configurações de áudio: escolha entrada, saída e buffer. Sem dispositivo, o app funciona em modo silencioso.",
      "Importe áudios com Arquivo › Importar (Ctrl+I) ou arrastando arquivos para a janela.",
      "Salve com Ctrl+S. O autosave guarda uma cópia a cada 2 minutos e oferece recuperação se o app fechar sem salvar.",
      "Dúvida sobre um botão, efeito ou parâmetro? Pare o mouse em cima dele: a legenda explica (desligue em Exibir › Legendas).",
    ],
    tourStep: 1,
  },
  {
    id: "gravacao",
    title: "Gravação",
    keywords: "gravar take punch metrônomo contagem entrada microfone latência monitorar armar",
    intro: "Cada gravação vira um take novo; nada é sobrescrito.",
    steps: [
      "No painel Gravação, escolha o dispositivo em Entrada de áudio e confira o nível nos medidores; 🎧 Ouvir entrada toca o microfone direto na saída (sem gravar).",
      "Arme a faixa no botão ● do cabeçalho e escolha a entrada dela (mono ou estéreo).",
      "Ligue o metrônomo (C) e, se quiser, a contagem prévia e o punch-in/out no painel Gravação.",
      "Pressione R para gravar e Espaço para parar. Buffer de 64 a 128 amostras dá a menor latência.",
      "Para trocar de take, use a lista de takes da faixa no painel Gravação.",
    ],
    tourStep: 3,
  },
  {
    id: "mixagem",
    title: "Mixagem e efeitos",
    keywords: "mixer volume pan efeito eq compressor reverb delay limiter bus envio send master presets",
    intro: "O mixer tem um canal por faixa, buses para efeitos compartilhados e o master.",
    steps: [
      "Arraste o fader e o pan (ou use as setas do teclado com o controle focado).",
      "+ efeito adiciona EQ, compressor, gate, saturação, chorus, delay, reverb, limiter, de-esser ou correção de afinação.",
      "Crie um bus (+ Bus) para reverb/delay compartilhados e envie as faixas para ele.",
      "Efeitos com atraso (limiter, afinação) são compensados nas faixas automaticamente.",
    ],
    tourStep: 4,
  },
  {
    id: "edicao",
    title: "Edição de clipes",
    keywords: "cortar dividir aparar fade crossfade normalizar inverter fase reverter ganho espectrograma",
    intro: "A edição é não destrutiva: o arquivo original nunca muda e tudo pode ser desfeito (Ctrl+Z).",
    steps: [
      "Selecione um clipe e pressione S para dividir no cursor.",
      "Arraste as bordas do clipe para aparar e os cantos superiores para fazer fades.",
      "Clique com o botão direito para Normalizar, Inverter fase, Reverter ou fazer crossfade com o vizinho.",
      "O botão ≋ da faixa alterna entre forma de onda e espectrograma.",
    ],
    tourStep: 2,
  },
  {
    id: "harmonia",
    title: "Harmonia e composição",
    keywords: "acordes campo harmônico progressão gênero próximo acorde cifra graus romanos compor",
    intro: "Sugestões de progressões por tom, modo e gênero, sem IA: regras de harmonia.",
    steps: [
      "No painel Harmonia, escolha o tom e o modo; o campo harmônico aparece em cifra ou graus romanos.",
      "Filtre progressões por gênero, ouça e use Inserir no arranjo.",
      "Em Próximo acorde, monte uma sequência com sugestões a cada passo.",
      "Cadastre as suas em Minhas progressões.",
    ],
  },
  {
    id: "bases",
    title: "Bases",
    keywords: "base instrumental loop gerar soundfont bpm tom importar base",
    intro: "Bases geradas pelo app a partir de progressões e ritmos, ou bases suas.",
    steps: [
      "Gerar base…: escolha progressão, ritmo, andamento e compassos.",
      "Adicionar áudio…: cadastre um loop seu com BPM, tom e gênero.",
      "Ao inserir uma base com BPM ou tom diferente do projeto, o app pergunta se deve adaptar ou manter.",
    ],
  },
  {
    id: "partitura",
    title: "Partitura e tablatura",
    keywords: "notação partitura tablatura musicxml midi guitar pro imprimir pdf notas inserir clicar digitar texto lote acorde pausa",
    intro: "Editor completo de partitura e tablatura com reprodução por SoundFont.",
    steps: [
      "No painel Partitura, crie uma partitura em branco ou importe MusicXML, MIDI ou Guitar Pro.",
      "✎ Inserir notas: passe o mouse na pauta e clique — a altura define a nota (balão mostra o nome).",
      "Digitar notas: escreva C, C, C, x, x, x, D, D, D (x = pausa, C5 = oitava, :8 = duração, C+E+G = acorde) e confira a prévia; Enter insere tudo com um único desfazer.",
      "Teclado: A–G nota, 0 pausa, 1–7 duração; a validação aponta compassos incompletos.",
      "Exporte em MusicXML ou MIDI, ou use Imprimir / PDF.",
    ],
  },
  {
    id: "letra",
    title: "Letra sincronizada",
    keywords: "letra karaokê lrc srt sílaba sincronizar toque",
    intro: "Sincronização por linha, palavra ou sílaba (use hífens para separar sílabas).",
    steps: [
      "Cole a letra no painel Letra e clique em Aplicar texto.",
      "Sincronizar por toque: toque o projeto e pressione Espaço a cada unidade; Enter conclui.",
      "Ajuste tempos manualmente selecionando a sílaba e exporte em LRC ou SRT.",
    ],
  },
  {
    id: "afinacao",
    title: "Afinação",
    keywords: "afinador pitch correção afinação nota escala time-stretch",
    intro: "Afinador em tempo real e correção de afinação ao vivo (efeito) ou em pós-produção.",
    steps: [
      "No painel Afinação, ligue o afinador e escolha a entrada e a referência (Lá 440 Hz).",
      "Selecione um clipe de voz e use o editor para detectar e corrigir notas, uma a uma ou pela escala.",
      "Para correção ao vivo, adicione o efeito Correção de afinação no mixer.",
    ],
  },
  {
    id: "exportar",
    title: "Exportar",
    keywords: "exportar mixdown wav mp3 flac ogg aiff stems normalizar",
    intro: "Exporte a mixagem inteira, um trecho ou cada faixa separada (stems).",
    steps: [
      "Arquivo › Exportar (Ctrl+E): escolha formato, qualidade e intervalo.",
      "Normalizar ajusta o pico final; a Análise da mixagem mede LUFS antes de exportar.",
    ],
  },
  {
    id: "atualizacoes",
    title: "Atualizações",
    keywords: "atualizar atualização versão nova update instalar reiniciar download assinatura",
    intro: "O app só procura versões novas quando você pede (ou se ligar a verificação ao abrir). Cada pacote é assinado e conferido antes de instalar.",
    steps: [
      "Ajuda › Verificar atualizações…: se houver versão nova, veja as novidades e clique em Instalar e reiniciar.",
      "Antes de instalar, o app para a reprodução ou gravação e salva o projeto; se salvar falhar, nada é instalado.",
      "Marque Verificar ao abrir o app para receber um aviso discreto na barra de status; Pular esta versão silencia esse aviso.",
      "Instalações .deb/.rpm no Linux: o botão abre a página de download para instalar a nova versão por cima.",
    ],
  },
  {
    id: "problemas",
    title: "Solução de problemas",
    keywords: "sem som áudio estalos buffer latência dispositivo microfone permissão erro travar",
    intro: "Os problemas mais comuns e como resolver.",
    steps: [
      "Sem som: confira a saída em Arquivo › Configurações de áudio; a barra de status avisa o modo silencioso.",
      "Estalos: aumente o buffer (256 ou 512 amostras).",
      "Gravação muda no macOS: Ajustes do Sistema › Privacidade › Microfone › ative o Ebiki Studio.",
      "Painel sumiu: reabra pelo menu Janela (ou Exibir › Painéis), ou use Exibir › Restaurar layout padrão.",
    ],
  },
];

/** Minúsculas sem acentos, para comparar termos. */
export function normalize(s: string): string {
  return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

const words = (s: string) => normalize(s).split(/[^a-z0-9]+/).filter(Boolean);

/** Pontos de um termo em um conjunto de palavras: palavra exata vale mais que prefixo. */
function hit(term: string, ws: string[]): number {
  if (ws.includes(term)) return 2;
  return ws.some((w) => w.startsWith(term)) ? 1 : 0;
}

/**
 * Tópicos que contêm todos os termos da busca (palavra inteira ou início de palavra).
 * Ordem: título > palavras-chave > texto; palavra exata > prefixo; empate mantém a ordem original.
 * Sem busca, todos na ordem original.
 */
export function searchTopics<T extends HelpTopic>(topics: T[], query: string): T[] {
  const terms = words(query);
  if (!terms.length) return topics;
  const scored = topics
    .map((t, i) => {
      const title = words(t.title);
      const keys = words(t.keywords);
      const body = words([t.intro, ...t.steps].join(" "));
      let score = 0;
      for (const w of terms) {
        const s = Math.max(hit(w, title) * 100, hit(w, keys) * 10, hit(w, body));
        if (s === 0) return null;
        score += s;
      }
      return { t, i, score };
    })
    .filter((x): x is { t: T; i: number; score: number } => x !== null);
  scored.sort((a, b) => b.score - a.score || a.i - b.i);
  return scored.map((x) => x.t);
}
