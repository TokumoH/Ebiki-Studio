import { PANEL_CATALOG } from "./catalog";
import { COMPONENTS, PANELS } from "./registry";

describe("registro de painéis", () => {
  it("todo painel do catálogo tem componente e vice-versa", () => {
    expect(Object.keys(COMPONENTS).sort()).toEqual(PANEL_CATALOG.map((p) => p.id).sort());
    expect(PANELS.map((p) => p.id)).toEqual(PANEL_CATALOG.map((p) => p.id));
  });

  it("ids e títulos são únicos e há um painel principal", () => {
    expect(new Set(PANEL_CATALOG.map((p) => p.id)).size).toBe(PANEL_CATALOG.length);
    expect(new Set(PANEL_CATALOG.map((p) => p.title)).size).toBe(PANEL_CATALOG.length);
    expect(PANEL_CATALOG.filter((p) => p.area === "main").map((p) => p.id)).toEqual(["timeline"]);
  });
});
