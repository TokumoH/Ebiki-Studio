// Audição: toca um WAV (ArrayBuffer vindo do núcleo) pela WebAudio, um de cada vez.
// Serviço técnico compartilhado (Harmonia e Bases); a geração do áudio fica no núcleo Rust.
import { create } from "zustand";

interface AuditionState {
  /** Identificador do que está tocando (ex.: "prog:pop-axis") ou null. */
  playing: string | null;
  loading: string | null;
}

export const useAudition = create<AuditionState>(() => ({ playing: null, loading: null }));

let ctx: AudioContext | null = null;
let current: AudioBufferSourceNode | null = null;

function audioContext(): AudioContext | null {
  if (typeof window === "undefined") return null;
  const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctor) return null;
  ctx ??= new Ctor();
  return ctx;
}

/** Para a audição em andamento (se houver). */
export function stopAudition() {
  const src = current;
  current = null;
  if (src) {
    src.onended = null;
    try {
      src.stop();
    } catch {
      /* já parado */
    }
  }
  useAudition.setState({ playing: null, loading: null });
}

/**
 * Busca o WAV com `load` e toca. Uma nova audição interrompe a anterior.
 * Erros de decodificação/IPC são propagados para o chamador mostrar o aviso.
 */
export async function audition(id: string, load: () => Promise<ArrayBuffer>): Promise<void> {
  stopAudition();
  useAudition.setState({ loading: id });
  try {
    const wav = await load();
    if (useAudition.getState().loading !== id) return; // cancelado durante o carregamento
    const ac = audioContext();
    if (!ac) {
      useAudition.setState({ loading: null });
      return;
    }
    if (ac.state === "suspended") await ac.resume();
    const buffer = await ac.decodeAudioData(wav.slice(0));
    if (useAudition.getState().loading !== id) return;
    const src = ac.createBufferSource();
    src.buffer = buffer;
    src.connect(ac.destination);
    src.onended = () => {
      if (current === src) {
        current = null;
        useAudition.setState({ playing: null });
      }
    };
    current = src;
    src.start();
    useAudition.setState({ playing: id, loading: null });
  } catch (e) {
    useAudition.setState({ loading: null, playing: null });
    throw e;
  }
}
