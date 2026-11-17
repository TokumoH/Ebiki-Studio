// "Seguir reprodução" (compartilhado pela timeline, transporte e menu Exibir — por isso fica em core):
// quando a linha de reprodução sai da vista, a timeline vira a página. Rolar à mão pausa até a linha
// reaparecer ou até o próximo ▶. A escolha liga/desliga é lembrada (localStorage); a pausa não.
import { create } from "zustand";
import type { TransportView } from "../ipc/types";

const KEY = "ebiki.follow.v1";
/** A partir desta fração da largura visível a vista vira a página… */
export const FOLLOW_EDGE = 0.9;
/** …e a linha fica a esta fração da borda esquerda. */
export const FOLLOW_LEAD = 0.1;

/**
 * Novo scrollLeft para manter a linha (`playheadPx`, em px do conteúdo) na vista, ou `null` se não
 * precisa rolar. Vira a página ao passar de 90% e quando a linha está antes da vista (fim do loop).
 */
export function followScrollLeft(playheadPx: number, scrollLeft: number, width: number): number | null {
  if (!(width > 0) || !Number.isFinite(playheadPx) || !Number.isFinite(scrollLeft)) return null;
  const rel = playheadPx - scrollLeft;
  if (rel >= 0 && rel < width * FOLLOW_EDGE) return null;
  return Math.max(0, Math.round(playheadPx - width * FOLLOW_LEAD));
}

/** O transporte está andando (a linha se move)? */
export function isRolling(state: TransportView["state"] | undefined): boolean {
  return state === "playing" || state === "recording" || state === "countin";
}

function load(): boolean {
  try {
    return window.localStorage.getItem(KEY) !== "0";
  } catch {
    return true;
  }
}

interface FollowState {
  enabled: boolean;
  /** Pausado porque o usuário rolou a vista durante a reprodução. */
  suspended: boolean;
  toggle: () => void;
  suspend: () => void;
  resume: () => void;
}

export const useFollow = create<FollowState>((set, get) => ({
  enabled: load(),
  suspended: false,
  toggle: () => {
    const enabled = !get().enabled;
    try {
      window.localStorage.setItem(KEY, enabled ? "1" : "0");
    } catch {
      /* sem armazenamento: vale só nesta sessão */
    }
    set({ enabled, suspended: false });
  },
  suspend: () => {
    if (!get().suspended) set({ suspended: true });
  },
  resume: () => {
    if (get().suspended) set({ suspended: false });
  },
}));
