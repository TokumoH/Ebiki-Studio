// Porta de transporte: o único ponto da UI que conhece o mecanismo de IPC.
export interface DialogFilter {
  name: string;
  extensions: string[];
}

export interface Transport {
  readonly kind: "tauri" | "mock";
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  invokeBinary(cmd: string, args?: Record<string, unknown>): Promise<ArrayBuffer>;
  listen<T>(event: string, cb: (payload: T) => void): Promise<() => void>;
  pickOpen(opts: { directory?: boolean; multiple?: boolean; filters?: DialogFilter[]; title?: string }): Promise<string[] | null>;
  pickSave(opts: { defaultPath?: string; filters?: DialogFilter[]; title?: string }): Promise<string | null>;
}

let current: Transport | null = null;

export function setTransport(t: Transport) {
  current = t;
}

export function transport(): Transport {
  if (!current) throw new Error("transporte IPC não inicializado");
  return current;
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
