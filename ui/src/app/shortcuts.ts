// Atalhos de teclado padrão de DAW. Ignorados quando o foco está em campos de texto.
export interface Shortcut {
  id: string;
  label: string;
  keys: string; // ex.: "Ctrl+Shift+Z", "Space"
}

export const DEFAULT_SHORTCUTS: Shortcut[] = [
  { id: "playPause", label: "Tocar / pausar", keys: "Space" },
  { id: "stopReturn", label: "Parar e voltar ao início", keys: "Enter" },
  { id: "record", label: "Gravar", keys: "R" },
  { id: "undo", label: "Desfazer", keys: "Ctrl+Z" },
  { id: "redo", label: "Refazer", keys: "Ctrl+Shift+Z" },
  { id: "redoAlt", label: "Refazer", keys: "Ctrl+Y" },
  { id: "save", label: "Salvar", keys: "Ctrl+S" },
  { id: "newTrack", label: "Nova faixa", keys: "Ctrl+T" },
  { id: "import", label: "Importar áudio", keys: "Ctrl+I" },
  { id: "export", label: "Exportar", keys: "Ctrl+E" },
  { id: "metronome", label: "Metrônomo", keys: "C" },
  { id: "loop", label: "Loop", keys: "L" },
  { id: "home", label: "Voltar ao início", keys: "Home" },
];

/** Normaliza um evento de teclado para o formato "Ctrl+Shift+K". */
export function eventKeys(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">): string {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  const k = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (!["Control", "Shift", "Alt", "Meta"].includes(k)) parts.push(k);
  return parts.join("+");
}

export function isTypingTarget(t: EventTarget | null): boolean {
  if (!(t instanceof HTMLElement)) return false;
  return t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName);
}

const ACTIVATION_ROLES = ["button", "menuitem", "menuitemcheckbox", "menuitemradio", "option", "tab", "checkbox", "radio", "switch", "link"];

/** Enter/Espaço num controle focado pertencem ao controle (ativação por teclado), não ao atalho global. */
export function isActivationKey(keys: string, t: EventTarget | null): boolean {
  if (keys !== "Enter" && keys !== "Space") return false;
  if (!(t instanceof HTMLElement)) return false;
  if (["BUTTON", "A", "SUMMARY"].includes(t.tagName)) return true;
  return ACTIVATION_ROLES.includes(t.getAttribute("role") ?? "");
}

export function loadShortcuts(): Shortcut[] {
  try {
    const raw = localStorage.getItem("ebiki.shortcuts");
    if (!raw) return DEFAULT_SHORTCUTS;
    const custom = JSON.parse(raw) as Record<string, string>;
    return DEFAULT_SHORTCUTS.map((s) => ({ ...s, keys: custom[s.id] ?? s.keys }));
  } catch {
    return DEFAULT_SHORTCUTS;
  }
}

export function saveShortcut(id: string, keys: string) {
  try {
    const raw = localStorage.getItem("ebiki.shortcuts");
    const custom = raw ? (JSON.parse(raw) as Record<string, string>) : {};
    custom[id] = keys;
    localStorage.setItem("ebiki.shortcuts", JSON.stringify(custom));
  } catch {
    /* armazenamento indisponível */
  }
}
