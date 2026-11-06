// Fluxo de atualização (ADR-0034): verificação manual (Ajuda › Verificar atualizações) ou ao abrir,
// se o usuário ligar (desligado por padrão — o app continua offline). Estado em zustand; o
// mecanismo fica atrás da porta `updater()` (ipc/).
import { create } from "zustand";
import { updater, UpdateError, type UpdateErrorKind, type UpdateInfo, type UpdateMode } from "../ipc/updater";
import { APP_VERSION } from "./about";

const PREFS_KEY = "ebiki.updates.v1";

export interface UpdatePrefs {
  checkOnOpen: boolean;
  /** Versão que o usuário pediu para pular (só afeta a verificação automática). */
  skipped: string | null;
}

const DEFAULT_PREFS: UpdatePrefs = { checkOnOpen: false, skipped: null };

export function loadUpdatePrefs(): UpdatePrefs {
  try {
    const raw = window.localStorage.getItem(PREFS_KEY);
    if (!raw) return DEFAULT_PREFS;
    const v = JSON.parse(raw) as Partial<UpdatePrefs>;
    return { checkOnOpen: v.checkOnOpen === true, skipped: typeof v.skipped === "string" ? v.skipped : null };
  } catch {
    return DEFAULT_PREFS;
  }
}

export function saveUpdatePrefs(p: UpdatePrefs) {
  try {
    window.localStorage.setItem(PREFS_KEY, JSON.stringify(p));
  } catch {
    /* sem armazenamento: a preferência vale só nesta sessão */
  }
}

/** `candidate` é mais nova que `current`? Compara x.y.z numericamente (ignora "v" e sufixos). */
export function isNewer(candidate: string, current: string): boolean {
  const parts = (v: string) => v.replace(/^v/, "").split(/[-+]/)[0]!.split(".").map((n) => Number.parseInt(n, 10) || 0);
  const a = parts(candidate);
  const b = parts(current);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const d = (a[i] ?? 0) - (b[i] ?? 0);
    if (d !== 0) return d > 0;
  }
  return false;
}

export type UpdateStatus = "idle" | "checking" | "upToDate" | "available" | "downloading" | "installing" | "error";

interface UpdateState {
  status: UpdateStatus;
  info: UpdateInfo | null;
  mode: UpdateMode;
  progress: { done: number; total: number | null };
  error: UpdateErrorKind | null;
  /** Diálogo aberto. */
  open: boolean;
  /** Aviso discreto na barra de status (verificação automática encontrou versão nova). */
  notice: boolean;
  prefs: UpdatePrefs;
  check: (opts: { manual: boolean }) => Promise<void>;
  /** Baixa → `beforeInstall` (parar o transporte, salvar) → instala → reinicia. */
  install: (beforeInstall: () => Promise<void>) => Promise<void>;
  openPage: () => Promise<void>;
  show: () => void;
  later: () => void;
  skip: () => void;
  setCheckOnOpen: (on: boolean) => void;
}

export const useUpdates = create<UpdateState>((set, get) => ({
  status: "idle",
  info: null,
  mode: "inApp",
  progress: { done: 0, total: null },
  error: null,
  open: false,
  notice: false,
  prefs: loadUpdatePrefs(),

  check: async ({ manual }) => {
    const busy = ["checking", "downloading", "installing"].includes(get().status);
    if (busy) {
      if (manual) set({ open: true });
      return;
    }
    set({ status: "checking", error: null, ...(manual ? { open: true } : {}) });
    try {
      const u = updater();
      const mode = await u.mode();
      const info = await u.check();
      if (!info || !isNewer(info.version, APP_VERSION)) {
        set({ status: manual ? "upToDate" : "idle", info: null, mode, notice: false });
        return;
      }
      if (!manual && get().prefs.skipped === info.version) {
        set({ status: "idle", info: null, mode });
        return;
      }
      set({ status: "available", info, mode, ...(manual ? { open: true } : { notice: true }) });
    } catch (e) {
      // automática: falha silenciosa (o app é offline por natureza); manual: explica
      if (manual) set({ status: "error", error: e instanceof UpdateError ? e.kind : "other" });
      else set({ status: "idle" });
    }
  },

  install: async (beforeInstall) => {
    if (get().status !== "available" || get().mode !== "inApp") return;
    set({ status: "downloading", progress: { done: 0, total: null }, error: null, open: true });
    try {
      await updater().download((done, total) => set({ progress: { done, total } }));
      set({ status: "installing" });
      // antes de instalar: no Windows a instalação encerra o app imediatamente
      await beforeInstall();
      await updater().install();
      await updater().relaunch();
    } catch (e) {
      set({ status: "error", error: e instanceof UpdateError ? e.kind : "other" });
    }
  },

  openPage: () => updater().openReleasePage(),
  show: () => set({ open: true }),
  later: () => set({ open: false }),
  skip: () => {
    const v = get().info?.version ?? null;
    const prefs = { ...get().prefs, skipped: v };
    saveUpdatePrefs(prefs);
    set({ prefs, open: false, notice: false, status: "idle", info: null });
  },
  setCheckOnOpen: (on) => {
    const prefs = { ...get().prefs, checkOnOpen: on };
    saveUpdatePrefs(prefs);
    set({ prefs });
  },
}));

/** Verificação ao abrir o app, se o usuário ligou. */
export function autoCheckForUpdates() {
  if (useUpdates.getState().prefs.checkOnOpen) void useUpdates.getState().check({ manual: false });
}

/** "8,7 de 14 MB" */
export function formatProgress(done: number, total: number | null): string {
  const mb = (n: number) => (n / 1_000_000).toLocaleString("pt-BR", { maximumFractionDigits: 1 });
  return total ? `${mb(done)} de ${mb(total)} MB` : `${mb(done)} MB`;
}
