import { describe, expect, it } from "vitest";
import { barToTick, loopLabel, loopRangeBars, projectLoopRange, tickToBar, PPQ } from "../../core/bars";
import { gridLines } from "./grid";

const BAR_44 = PPQ * 4;
const fourFour = { tempos: [{ tick: 0, bpm: 120 }], meters: [{ tick: 0, numerator: 4, denominator: 4 }] };
// 4/4 nos compassos 1–2, 3/4 a partir do compasso 3; e uma troca no meio de um compasso (compasso cortado)
const changing = { ...fourFour, meters: [...fourFour.meters, { tick: 2 * BAR_44, numerator: 3, denominator: 4 }] };
const cut = { ...fourFour, meters: [...fourFour.meters, { tick: BAR_44 + PPQ * 2, numerator: 3, denominator: 4 }] };

describe("compassos ↔ tiques (mesma numeração da régua)", () => {
  it.each([["4/4", fourFour], ["troca de fórmula", changing], ["compasso cortado", cut]] as const)("%s: bate com os números da régua", (_, tempo) => {
    const bars = gridLines(tempo, 0, BAR_44 * 12).filter((l) => l.isBar);
    for (const l of bars) {
      expect(barToTick(tempo, l.bar), `compasso ${l.bar}`).toBe(l.tick);
      expect(tickToBar(tempo, l.tick)).toBe(l.bar);
      expect(tickToBar(tempo, l.tick + 1)).toBe(l.bar);
    }
  });
  it("limites", () => {
    expect(barToTick(fourFour, 0)).toBe(0);
    expect(tickToBar(fourFour, -50)).toBe(1);
  });
});

describe("predefinições de loop", () => {
  it("compassos X até Y incluem o compasso Y inteiro", () => {
    expect(loopRangeBars(fourFour, 1, 8)).toEqual([0, 8 * BAR_44]);
    expect(loopRangeBars(fourFour, 3, 3)).toEqual([2 * BAR_44, 3 * BAR_44]);
    expect(loopRangeBars(changing, 2, 3)).toEqual([BAR_44, 2 * BAR_44 + PPQ * 3]);
    expect(loopRangeBars(fourFour, 5, 2)).toEqual([BAR_44, 5 * BAR_44]); // invertido: ordena
    expect(loopRangeBars(fourFour, 0, 1)).toEqual([0, BAR_44]); // compasso mínimo 1
  });
  it("projeto inteiro vai do início ao fim do compasso do último clipe", () => {
    expect(projectLoopRange(fourFour, 0)).toBeNull(); // sem clipes
    expect(projectLoopRange(fourFour, 5 * BAR_44 + 10)).toEqual([0, 6 * BAR_44]);
    expect(projectLoopRange(fourFour, 4 * BAR_44)).toEqual([0, 4 * BAR_44]); // termina na barra: não sobra compasso vazio
  });
  it("rótulo com o primeiro e o último compasso", () => {
    expect(loopLabel(fourFour, [0, 8 * BAR_44])).toBe("1–8");
    expect(loopLabel(fourFour, [2 * BAR_44, 3 * BAR_44])).toBe("3");
    expect(loopLabel(fourFour, [PPQ, BAR_44 + PPQ])).toBe("1–2");
  });
});
