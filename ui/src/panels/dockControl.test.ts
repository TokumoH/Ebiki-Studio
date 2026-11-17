import { expect, it } from "vitest";
import { bottomDockHeight } from "./dockControl";

it("grupo de baixo ocupa metade da área dos painéis, entre 120 e 330 px (UX-03)", () => {
  expect(bottomDockHeight(1000)).toBe(330);
  expect(bottomDockHeight(600)).toBe(300);
  expect(bottomDockHeight(378)).toBe(189);
  expect(bottomDockHeight(200)).toBe(120);
  expect(bottomDockHeight(0)).toBe(330);
  expect(bottomDockHeight(Number.NaN)).toBe(330);
});
