import { buildMenus, type MenuContext } from "./menuModel";
import { DEFAULT_SHORTCUTS } from "./shortcuts";
import type { MenuEntry } from "../shared/MenuBar";
import { PANEL_CATALOG } from "../panels/catalog";

const base: MenuContext = {
  shortcuts: DEFAULT_SHORTCUTS,
  run: () => undefined,
  canUndo: true,
  canRedo: false,
  undoLabel: "Mover clipe",
  redoLabel: null,
  theme: "dark",
  openPanels: ["timeline", "mixer"],
  statusBar: true,
  tooltips: true,
  metronome: false,
  loop: true,
  hasSelectedTrack: false,
  selectedTrackArmed: false,
  selectedTrackSpectrum: false,
  clip: null,
};

const items = (c: MenuContext, menu: string) => buildMenus(c).find((m) => m.id === menu)!.items.filter((i): i is Extract<MenuEntry, { kind: "item" }> => i.kind === "item");
const byId = (c: MenuContext, menu: string, id: string) => items(c, menu).find((i) => i.id === id)!;

describe("modelo do menu principal", () => {
  it("tem o menu da logo e os 9 menus na ordem do header", () => {
    expect(buildMenus(base).map((m) => m.label)).toEqual(["Menu do Ebiki", "Arquivo", "Editar", "Exibir", "Transporte", "Faixa", "Clipe", "Ferramentas", "Janela", "Ajuda"]);
  });

  it("menu da logo: cabeçalho do projeto e o essencial (exportar, configurações, legendas, ajuda)", () => {
    const app = buildMenus({ ...base, projectName: "Demo", projectDetail: "48 kHz · 24 bits" })[0]!;
    expect(app.items[0]).toEqual({ kind: "label", text: "Demo", detail: "48 kHz · 24 bits" });
    const ids = app.items.flatMap((i) => (i.kind === "item" ? [i.id] : []));
    for (const id of ["export", "import", "save", "audioSettings", "shortcutsDialog", "toggleTooltips", "checkUpdates", "help", "about", "closeProject"]) expect(ids).toContain(id);
    // tema: oferece o oposto do atual
    expect(ids).toContain("themeLight");
    expect(buildMenus({ ...base, theme: "light" })[0]!.items.some((i) => i.kind === "item" && i.id === "themeDark")).toBe(true);
  });

  it("mostra o atalho configurado (inclusive personalizado)", () => {
    expect(byId(base, "arquivo", "save").keys).toBe("Ctrl+S");
    const custom = { ...base, shortcuts: DEFAULT_SHORTCUTS.map((s) => (s.id === "save" ? { ...s, keys: "Ctrl+Shift+S" } : s)) };
    expect(byId(custom, "arquivo", "save").keys).toBe("Ctrl+Shift+S");
  });

  it("desfazer/refazer refletem o histórico", () => {
    expect(byId(base, "editar", "undo")).toMatchObject({ label: "Desfazer: Mover clipe", disabled: false });
    expect(byId(base, "editar", "redo")).toMatchObject({ label: "Refazer", disabled: true });
  });

  it("ações de clipe e faixa ficam desativadas sem seleção", () => {
    for (const id of ["normalizeClip", "polarityClip", "reverseClip", "deleteClip"]) expect(byId(base, "clipe", id).disabled).toBe(true);
    expect(byId(base, "faixa", "deleteTrack").disabled).toBe(true);
    const sel = { ...base, hasSelectedTrack: true, clip: { polarityInverted: true, reversed: false } };
    expect(byId(sel, "clipe", "polarityClip")).toMatchObject({ disabled: false, checked: true });
    expect(byId(sel, "faixa", "deleteTrack").disabled).toBe(false);
  });

  it("tema, metrônomo, loop e painéis aparecem marcados conforme o estado", () => {
    expect(byId(base, "exibir", "themeDark")).toMatchObject({ checked: true, radio: true });
    expect(byId(base, "exibir", "themeLight").checked).toBe(false);
    expect(byId(base, "transporte", "loop").checked).toBe(true);
    expect(byId(base, "transporte", "metronome").checked).toBe(false);
    const janela = items(base, "janela").filter((i) => i.id.startsWith("window:"));
    expect(janela.map((i) => i.label)).toEqual(PANEL_CATALOG.map((p) => p.title));
    expect(janela.find((i) => i.id === "window:mixer")!.checked).toBe(true);
    expect(janela.find((i) => i.id === "window:lyrics")!.checked).toBe(false);
  });

  it("cada item chama o comando certo", () => {
    const calls: string[] = [];
    const c = { ...base, run: (id: string) => calls.push(id) };
    byId(c, "ajuda", "help").run();
    items(c, "janela").find((i) => i.id === "window:mixer")!.run();
    byId(c, "ferramentas", "panel:advisor").run();
    expect(calls).toEqual(["help", "togglePanel:mixer", "panel:advisor"]);
  });
});
