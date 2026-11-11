import { describe, expect, it } from "vitest";
import { menuPosition } from "./ClipContextMenu";

describe("posição de menus flutuantes", () => {
  const vp = { width: 1000, height: 700 };
  it("menu do clipe abre no ponteiro e vira quando não cabe", () => {
    expect(menuPosition(100, 100, { width: 240, height: 300 }, vp)).toEqual({ left: 100, top: 100 });
    expect(menuPosition(100, 600, { width: 240, height: 300 }, vp)).toEqual({ left: 100, top: 300 }); // vira para cima
    expect(menuPosition(900, 100, { width: 240, height: 300 }, vp)).toEqual({ left: 660, top: 100 }); // vira para a esquerda
    expect(menuPosition(5, 690, { width: 240, height: 900 }, vp).top).toBe(8); // maior que a janela: gruda no topo
  });
});
