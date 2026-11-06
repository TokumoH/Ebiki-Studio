import { barStartAt, loopRangeFromCursor, PPQ } from "./bars";

const BAR_44 = PPQ * 4; // 3840
const BAR_34 = PPQ * 3; // 2880
const fourFour = { tempos: [{ tick: 0, bpm: 120 }], meters: [{ tick: 0, numerator: 4, denominator: 4 }] };
// 4/4 nos compassos 1–2 e 3/4 a partir do compasso 3 (tick 7680)
const changing = { ...fourFour, meters: [...fourFour.meters, { tick: 2 * BAR_44, numerator: 3, denominator: 4 }] };

describe("barStartAt", () => {
  it("encontra o início do compasso que contém o tick", () => {
    expect(barStartAt(fourFour, 0)).toBe(0);
    expect(barStartAt(fourFour, BAR_44 - 1)).toBe(0);
    expect(barStartAt(fourFour, BAR_44)).toBe(BAR_44);
    expect(barStartAt(fourFour, BAR_44 * 5 + 100)).toBe(BAR_44 * 5);
  });

  it("respeita mudança de fórmula de compasso", () => {
    expect(barStartAt(changing, 2 * BAR_44 + BAR_34 + 10)).toBe(2 * BAR_44 + BAR_34);
    expect(barStartAt(changing, 2 * BAR_44 - 1)).toBe(BAR_44);
  });

  it("usa 4/4 sem fórmulas e trata ticks negativos como 0", () => {
    expect(barStartAt({ tempos: [], meters: [] }, BAR_44 + 1)).toBe(BAR_44);
    expect(barStartAt(fourFour, -500)).toBe(0);
  });

  it("6/8 tem compasso de 6 colcheias", () => {
    const sixEight = { ...fourFour, meters: [{ tick: 0, numerator: 6, denominator: 8 }] };
    expect(barStartAt(sixEight, 2880 + 5)).toBe(2880);
  });

  it("aceita fórmulas fora de ordem", () => {
    const unordered = { ...fourFour, meters: [{ tick: 2 * BAR_44, numerator: 3, denominator: 4 }, { tick: 0, numerator: 4, denominator: 4 }] };
    expect(barStartAt(unordered, 2 * BAR_44 + BAR_34 + 10)).toBe(2 * BAR_44 + BAR_34);
  });
});

describe("loopRangeFromCursor", () => {
  it("cobre 4 compassos a partir do compasso do cursor (4/4)", () => {
    expect(loopRangeFromCursor(fourFour, 0)).toEqual([0, 4 * BAR_44]);
    expect(loopRangeFromCursor(fourFour, BAR_44 * 2 + 700)).toEqual([BAR_44 * 2, BAR_44 * 6]);
  });

  it("atravessa a mudança de fórmula (2 compassos 4/4 + 2 compassos 3/4)", () => {
    expect(loopRangeFromCursor(changing, 100)).toEqual([0, 2 * BAR_44 + 2 * BAR_34]);
    expect(loopRangeFromCursor(changing, BAR_44 + 1)).toEqual([BAR_44, 2 * BAR_44 + 3 * BAR_34]);
  });

  it("aceita outro número de compassos", () => {
    expect(loopRangeFromCursor(fourFour, 0, 2)).toEqual([0, 2 * BAR_44]);
  });
});
