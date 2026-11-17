// Estado dos avisos temporários (notify.success / notify.error), com ação opcional.
// Aviso igual a um que está na tela não empilha: substitui o anterior e o tempo recomeça.
import { create } from "zustand";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  kind: "info" | "success" | "error";
  message: string;
  action?: ToastAction;
}

interface ToastState {
  toasts: Toast[];
  push: (kind: Toast["kind"], message: string, action?: ToastAction) => void;
  dismiss: (id: number) => void;
}

let seq = 1;

export const useToasts = create<ToastState>((set) => ({
  toasts: [],
  push: (kind, message, action) => {
    const id = seq++;
    set((s) => ({ toasts: [...s.toasts.filter((t) => !(t.kind === kind && t.message === message)).slice(-4), { id, kind, message, action }] }));
    setTimeout(() => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })), kind === "error" ? 8000 : 4000);
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

export const notify = {
  info: (m: string, action?: ToastAction) => useToasts.getState().push("info", m, action),
  success: (m: string, action?: ToastAction) => useToasts.getState().push("success", m, action),
  error: (m: string, action?: ToastAction) => useToasts.getState().push("error", m, action),
};
