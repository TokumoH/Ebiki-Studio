// Ligações do menu e dos atalhos: todo item do menu e todo atalho padrão precisa de uma ação no
// registro de comandos do Workspace (senão vira um botão morto).
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { MenuEntry } from "../shared/MenuBar";
import { PANEL_CATALOG } from "../panels/catalog";
import { buildMenus, type MenuContext } from "./menuModel";
import { DEFAULT_SHORTCUTS } from "./shortcuts";

const workspace = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "Workspace.tsx"), "utf8");
const block = workspace.slice(workspace.indexOf("actions.current = {"), workspace.indexOf("const runCommand"));
const actionIds = new Set([...block.matchAll(/^\s{6}([a-zA-Z]+):/gm)].map((m) => m[1]!));

const ctx: MenuContext = {
  shortcuts: DEFAULT_SHORTCUTS,
  run: () => undefined,
  canUndo: true,
  canRedo: true,
  theme: "dark",
  openPanels: [],
  statusBar: true,
  tooltips: true,
  metronome: false,
  loop: false,
  hasSelectedTrack: true,
  selectedTrackArmed: false,
  selectedTrackSpectrum: false,
  clip: { polarityInverted: false, reversed: false },
};

function commands(items: MenuEntry[]): string[] {
  return items.flatMap((i) => (i.kind === "item" ? [i.id] : i.kind === "submenu" ? commands(i.items) : []));
}

describe("ligações de comandos", () => {
  it("o registro de ações foi encontrado (o teste não pode passar vazio)", () => {
    expect(actionIds.size).toBeGreaterThan(30);
  });

  it("todo item de todos os menus (nos dois temas) tem ação ou abre um painel existente", () => {
    const ids = new Set([...buildMenus(ctx), ...buildMenus({ ...ctx, theme: "light" })].flatMap((m) => commands(m.items)));
    const panels = new Set(PANEL_CATALOG.map((p) => p.id));
    const dead = [...ids].filter((id) => {
      const [prefix, panel] = id.split(":");
      if (panel !== undefined) return ["panel", "view", "window"].includes(prefix!) ? !panels.has(panel) : true;
      return !actionIds.has(id);
    });
    expect(dead).toEqual([]);
  });

  it("todo atalho padrão tem ação", () => {
    expect(DEFAULT_SHORTCUTS.map((s) => s.id).filter((id) => !actionIds.has(id))).toEqual([]);
  });
});
