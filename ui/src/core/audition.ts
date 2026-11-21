// Audição: pede ao núcleo que toque um trecho pelo MOTOR DE ÁUDIO (S-3), um de cada vez.
// Assim Harmonia, Bases e Partitura saem no fone escolhido em Configurações de áudio,
// na mesma saída do resto do app (antes saíam pela WebAudio, no dispositivo padrão do sistema).
// Serviço técnico compartilhado; a geração e a reprodução do áudio ficam no núcleo Rust.
import { create } from "zustand";
import { compositionApi } from "../ipc/api";

interface AuditionState {
  /** Identificador do que está tocando (ex.: "prog:pop-axis") ou null. */
  playing: string | null;
  loading: string | null;
}

export const useAudition = create<AuditionState>(() => ({ playing: null, loading: null }));

/** Margem após a duração informada, para o fim do trecho sair no fone antes de o botão voltar. */
const TAIL_MS = 150;
let timer: ReturnType<typeof setTimeout> | null = null;

function clearTimer() {
  if (timer !== null) clearTimeout(timer);
  timer = null;
}

/** Para a audição em andamento (se houver) — no motor e na UI. */
export function stopAudition() {
  const active = useAudition.getState();
  clearTimer();
  useAudition.setState({ playing: null, loading: null });
  if (active.playing || active.loading) void compositionApi.auditionStop().catch(() => undefined);
}

/**
 * Pede ao núcleo que toque (`play` chama o comando de audição, que já toca no motor) e marca
 * o item como tocando pela duração devolvida. Uma nova audição substitui a anterior no motor.
 * Erros de IPC são propagados para o chamador mostrar o aviso.
 */
export async function audition(id: string, play: () => Promise<{ seconds: number }>): Promise<void> {
  clearTimer();
  useAudition.setState({ loading: id, playing: null });
  try {
    const { seconds } = await play();
    const now = useAudition.getState();
    if (now.loading !== id) {
      // Parado durante o preparo: o motor começou a tocar depois do "Parar" — para de novo.
      // Se outra audição tomou o lugar, ela já substituiu esta no motor: nada a fazer.
      if (now.loading === null && now.playing === null) void compositionApi.auditionStop().catch(() => undefined);
      return;
    }
    useAudition.setState({ playing: id, loading: null });
    timer = setTimeout(() => {
      timer = null;
      if (useAudition.getState().playing === id) useAudition.setState({ playing: null });
    }, seconds * 1000 + TAIL_MS);
  } catch (e) {
    if (useAudition.getState().loading === id) useAudition.setState({ loading: null, playing: null });
    throw e;
  }
}
