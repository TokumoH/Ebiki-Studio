// Estado de aplicação: projeção da sessão vinda do núcleo (fonte da verdade continua no Rust).
import { create } from "zustand";
import { api, ApiFailure } from "../ipc/api";
import type { SessionView } from "../ipc/types";
import { notify } from "./toasts";

interface SessionState {
  session: SessionView | null;
  busy: boolean;
  selectedTrackId: string | null;
  selectedClipId: string | null;
  setSession: (s: SessionView | null) => void;
  select: (sel: { trackId?: string | null; clipId?: string | null }) => void;
}

export const useSession = create<SessionState>((set) => ({
  session: null,
  busy: false,
  selectedTrackId: null,
  selectedClipId: null,
  setSession: (session) =>
    set((st) => ({
      session,
      selectedTrackId: session?.tracks.some((t) => t.id === st.selectedTrackId) ? st.selectedTrackId : (session?.tracks[0]?.id ?? null),
      selectedClipId: session?.tracks.some((t) => t.clips.some((c) => c.id === st.selectedClipId)) ? st.selectedClipId : null,
    })),
  select: (sel) => set((st) => ({ selectedTrackId: sel.trackId !== undefined ? sel.trackId : st.selectedTrackId, selectedClipId: sel.clipId !== undefined ? sel.clipId : st.selectedClipId })),
}));

export function errorMessage(e: unknown): string {
  if (e instanceof ApiFailure) return e.message;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return String(e);
}

/** Executa um comando que devolve a sessão; erros viram aviso (sem quebrar a UI). */
export async function run(fn: () => Promise<SessionView | null | undefined | void>, opts: { success?: string; busy?: boolean } = {}): Promise<boolean> {
  if (opts.busy) useSession.setState({ busy: true });
  try {
    const r = await fn();
    if (r && typeof r === "object" && "project" in r) useSession.getState().setSession(r);
    if (opts.success) notify.success(opts.success);
    return true;
  } catch (e) {
    notify.error(errorMessage(e));
    return false;
  } finally {
    if (opts.busy) useSession.setState({ busy: false });
  }
}

let gestureSeq = Date.now() % 1_000_000;

/** Identificador de gesto contínuo (agrupa ações num único desfazer). */
export function newGesture(): number {
  gestureSeq += 1;
  return gestureSeq;
}

export async function endGesture() {
  try {
    await api.endGesture();
  } catch {
    /* sem projeto: ignorar */
  }
}
