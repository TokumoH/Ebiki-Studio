// Exibição de atalhos em PT-BR (UX-13): "Space" → "Espaço", setas como símbolos e, no macOS, Ctrl → ⌘.
// O formato gravado continua "Ctrl+Shift+K" (eventKeys); isto só muda o que a pessoa lê.
const NAMES: Record<string, string> = {
  Space: "Espaço",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Escape: "Esc",
  Backspace: "⌫",
  PageUp: "PgUp",
  PageDown: "PgDn",
};

const MAC: Record<string, string> = { Ctrl: "⌘", Alt: "⌥", Shift: "⇧" };

export const isMac = (): boolean => typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export function displayKeys(keys: string, mac = isMac()): string {
  if (!keys) return "";
  return keys
    .split("+")
    .map((k) => (mac && MAC[k] ? MAC[k] : (NAMES[k] ?? k)))
    .join(mac ? "" : "+");
}
