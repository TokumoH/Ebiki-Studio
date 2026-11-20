import { expect, it } from "vitest";
import { recentXruns } from "./xruns";

it("conta só as falhas dentro da janela de 10 s (AU-11)", () => {
  const s = [
    { at: 0, total: 0 },
    { at: 1_000, total: 3 }, // 3 falhas há muito tempo
    { at: 20_000, total: 5 }, // 2 falhas recentes
  ];
  expect(recentXruns(s, 21_000)).toBe(2);
  expect(recentXruns(s, 40_000)).toBe(0); // passou: a barra para de pedir buffer maior
});

it("núcleo reaberto (contagem recomeça) não vira número negativo", () => {
  const s = [
    { at: 0, total: 50 },
    { at: 5_000, total: 2 },
  ];
  expect(recentXruns(s, 6_000)).toBe(2);
});

it("sem amostras ou uma só: zero", () => {
  expect(recentXruns([], 0)).toBe(0);
  expect(recentXruns([{ at: 0, total: 9 }], 1)).toBe(0);
});
