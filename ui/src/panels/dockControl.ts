// Fabricação pura: ponte entre a área de painéis (dockview) e quem precisa abrir/focar painéis
// (ex.: menu "Painéis" da barra superior), sem que estes conheçam o dockview.
import { create } from "zustand";
import type { AddPanelOptions, DockviewApi } from "dockview-react";
import { PANEL_CATALOG } from "./catalog";

interface DockState {
  /** Ids dos painéis presentes no layout. */
  open: string[];
}

export const useDockState = create<DockState>(() => ({ open: [] }));

let dock: DockviewApi | null = null;
let subs: { dispose: () => void }[] = [];

function syncOpen() {
  useDockState.setState({ open: dock ? dock.panels.map((p) => p.id) : [] });
}

/** Conecta (ou desconecta, com `null`) a API da área de painéis. Idempotente. */
export function attachDock(api: DockviewApi | null) {
  if (api === dock) return;
  subs.forEach((s) => s.dispose());
  subs = api ? [api.onDidAddPanel(syncOpen), api.onDidRemovePanel(syncOpen)] : [];
  dock = api;
  syncOpen();
}

/** Opções para inserir um painel do catálogo junto aos da mesma área (ou ao lado do layout atual). */
export function panelOptions(api: DockviewApi, id: string): AddPanelOptions<{ id: string }> | null {
  const meta = PANEL_CATALOG.find((p) => p.id === id);
  if (!meta) return null;
  const sibling = PANEL_CATALOG.filter((p) => p.area === meta.area && p.id !== id)
    .map((p) => api.getPanel(p.id))
    .find((p) => p !== undefined);
  const any = api.panels[0];
  const position = sibling
    ? ({ referencePanel: sibling.id, direction: "within" } as const)
    : any
      ? ({ referencePanel: any.id, direction: meta.area === "main" ? "above" : "below" } as const)
      : undefined;
  return { id, component: "host", title: meta.title, params: { id }, ...(position ? { position } : {}) };
}

/** Reabre um painel fechado ou traz para a frente um já aberto. */
export function openPanel(id: string): boolean {
  if (!dock) return false;
  const existing = dock.getPanel(id);
  if (existing) {
    existing.api.setActive();
    return true;
  }
  const opts = panelOptions(dock, id);
  if (!opts) return false;
  dock.addPanel(opts).api.setActive();
  return true;
}
