// Porta de atualização do app (ADR-0034): a UI só conhece esta interface. A implementação real
// (plugin updater do Tauri, assinatura verificada no Rust) fica em updaterTauri.ts; o mock serve
// aos testes e ao modo navegador.

export type UpdateMode = "inApp" | "manual";

export interface UpdateInfo {
  version: string;
  /** Data de publicação (ISO), quando o manifesto informa. */
  date?: string;
  notes: string;
}

export type UpdateErrorKind = "offline" | "signature" | "other";

export class UpdateError extends Error {
  constructor(
    readonly kind: UpdateErrorKind,
    message: string,
  ) {
    super(message);
    this.name = "UpdateError";
  }
}

/** Classifica a mensagem de erro do plugin/servidor em um tipo que a UI sabe explicar. */
export function classifyUpdateError(message: string): UpdateErrorKind {
  const m = message.toLowerCase();
  if (/signature|minisign|public key|pubkey/.test(m)) return "signature";
  if (/network|connect|dns|resolve|timed? ?out|offline|unreachable|tcp|tls|sending request/.test(m)) return "offline";
  return "other";
}

export interface Updater {
  /** Como esta instalação se atualiza (pacotes .deb/.rpm: só pela página de download). */
  mode(): Promise<UpdateMode>;
  /** Consulta o manifesto da última release; `null` = já está na mais recente. */
  check(): Promise<UpdateInfo | null>;
  /** Baixa, verifica a assinatura e instala a versão encontrada pelo último `check`. */
  install(onProgress: (downloaded: number, total: number | null) => void): Promise<void>;
  relaunch(): Promise<void>;
  openReleasePage(): Promise<void>;
}

export const RELEASES_URL = "https://github.com/TokumoH/Ebiki-Studio/releases/latest";

let current: Updater | null = null;

export function setUpdater(u: Updater) {
  current = u;
}

export function updater(): Updater {
  if (!current) throw new Error("atualizador não inicializado");
  return current;
}

/**
 * Atualizador simulado. O cenário vem do armazenamento local (testes E2E o definem antes de
 * carregar a página): `ebiki.mock.update` = JSON de UpdateInfo, "offline" ou "signature";
 * `ebiki.mock.updateMode` = "manual" para simular .deb/.rpm. Sem cenário: já atualizado.
 */
export function createMockUpdater(read: (key: string) => string | null = safeRead): Updater {
  const scenario = () => read("ebiki.mock.update");
  return {
    mode: async () => (read("ebiki.mock.updateMode") === "manual" ? "manual" : "inApp"),
    check: async () => {
      const s = scenario();
      if (!s) return null;
      if (s === "offline") throw new UpdateError("offline", "network unreachable");
      if (s === "signature") return { version: "9.9.9", notes: "Versão adulterada" };
      return JSON.parse(s) as UpdateInfo;
    },
    install: async (onProgress) => {
      if (scenario() === "signature") throw new UpdateError("signature", "signature verification failed");
      const total = 14_000_000;
      for (const done of [0, total / 2, total]) {
        onProgress(done, total);
        await new Promise((r) => setTimeout(r, 30));
      }
    },
    relaunch: async () => {
      (window as unknown as { __ebikiRelaunched?: boolean }).__ebikiRelaunched = true;
    },
    openReleasePage: async () => {
      (window as unknown as { __ebikiOpenedUrl?: string }).__ebikiOpenedUrl = RELEASES_URL;
    },
  };
}

function safeRead(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}
