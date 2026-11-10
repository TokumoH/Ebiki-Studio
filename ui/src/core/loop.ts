// Loop (compartilhado pelo transporte e pelo arranjo — por isso fica em core): liga/desliga lembrando
// o último trecho e abre/fecha as opções; botão ▾, menu Transporte, L e régua usam as mesmas funções.
// O trecho em si fica no núcleo (recording.loopRange).
import { create } from "zustand";
import { api } from "../ipc/api";
import { loopRangeFromCursor } from "./bars";
import { run, useSession } from "./session";
import { useLive } from "./live";

interface LoopUiState {
  /** Opções de loop abertas. */
  open: boolean;
  /** Último trecho usado (religar volta a ele em vez de recomeçar em 4 compassos). */
  last: [number, number] | null;
  setOpen: (open: boolean) => void;
  remember: (range: [number, number]) => void;
}

export const useLoopUi = create<LoopUiState>((set) => ({
  open: false,
  last: null,
  setOpen: (open) => set({ open }),
  remember: (last) => set({ last }),
}));

/** Define o trecho do loop (ou desliga com `null`) e lembra o trecho escolhido. */
export async function setLoop(range: [number, number] | null): Promise<boolean> {
  if (range) useLoopUi.getState().remember(range);
  return run(() => api.recordingLoop(range));
}

/** L / botão Loop: desliga se ligado; senão religa o último trecho ou 4 compassos a partir do cursor. */
export function toggleLoop(): Promise<boolean> {
  const s = useSession.getState().session;
  if (!s) return Promise.resolve(false);
  const cur = s.recording.loopRange;
  if (cur) {
    useLoopUi.getState().remember(cur);
    return run(() => api.recordingLoop(null));
  }
  const range = useLoopUi.getState().last ?? loopRangeFromCursor(s.tempo, useLive.getState().transport?.tick ?? 0);
  return setLoop(range);
}
