// Resolvedor das legendas do app: liga a camada genérica (shared/TooltipLayer) ao glossário (core).
// Ordem: chave explícita `data-tip` → aba de setor → nome acessível (padrões do glossário) → título
// nativo do elemento. Preferência "legendas ligadas" fica no armazenamento local.
import { create } from "zustand";
import { tipByKey, tipByName, tipForPanelTitle, type TipEntry } from "../core/glossary";
import type { TipContent } from "../shared/TooltipLayer";
import { PANEL_CATALOG } from "../panels/catalog";

const KEY = "ebiki.tooltips.v1";

const toContent = (t: TipEntry): TipContent => ({ title: t.title, body: t.body, meta: t.meta });

/** Nome acessível simplificado: aria-label, depois texto de botões/abas curtas. */
function accessibleName(el: Element): string {
  const label = el.getAttribute("aria-label");
  if (label) return label;
  const role = el.getAttribute("role");
  const tag = el.tagName;
  if (tag === "BUTTON" || role === "button" || role === "tab" || role === "radio") {
    const text = (el.textContent ?? "").trim();
    return text.length <= 40 ? text : "";
  }
  return "";
}

export function resolveTip(el: Element): TipContent | null {
  // chave desconhecida não vira texto cru: segue para os próximos critérios (e o teste de cobertura acusa)
  const key = el.getAttribute("data-tip");
  const byKey = key ? tipByKey(key) : undefined;
  if (byKey) return toContent(byKey);
  const menu = el.getAttribute("data-menu-id");
  if (menu) {
    const m = tipByKey(`menu.${menu}`);
    if (m) return toContent(m);
  }
  if (el.classList.contains("dv-tab")) {
    const t = tipForPanelTitle(el.textContent ?? "", PANEL_CATALOG);
    if (t) return toContent(t);
  }
  const name = accessibleName(el);
  if (name) {
    const t = tipByName(name);
    if (t) return toContent(t);
  }
  const text = el.getAttribute("data-tip-text");
  return text ? { body: text } : null;
}

function load(): boolean {
  try {
    return window.localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

interface TooltipPrefs {
  enabled: boolean;
  toggle: () => void;
}

export const useTooltipPrefs = create<TooltipPrefs>((set, get) => ({
  enabled: load(),
  toggle: () => {
    const enabled = !get().enabled;
    try {
      window.localStorage.setItem(KEY, enabled ? "on" : "off");
    } catch {
      /* sem armazenamento: vale só nesta sessão */
    }
    set({ enabled });
  },
}));

// gancho só no modo simulado (testes E2E medem a cobertura das legendas na interface real)
if (import.meta.env.MODE === "mock" && typeof window !== "undefined") {
  (window as unknown as { __ebikiResolveTip?: typeof resolveTip }).__ebikiResolveTip = resolveTip;
}
