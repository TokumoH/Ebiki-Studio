import { gridLines, snapToBeat } from "./grid";

const fourFour = { tempos: [{ tick: 0, bpm: 120 }], meters: [{ tick: 0, numerator: 4, denominator: 4 }] };

it("gera compassos e tempos em 4/4", () => {
  const l = gridLines(fourFour, 0, 3840 * 2);
  expect(l.filter((x) => x.isBar).map((x) => x.bar)).toEqual([1, 2, 3]);
  expect(l[1]).toMatchObject({ tick: 960, bar: 1, beat: 2, isBar: false });
});

it("respeita mudança de fórmula de compasso", () => {
  const t = { ...fourFour, meters: [...fourFour.meters, { tick: 7680, numerator: 3, denominator: 4 }] };
  const bars = gridLines(t, 0, 7680 + 2880 * 2).filter((x) => x.isBar);
  expect(bars.map((b) => [b.bar, b.tick])).toEqual([[1, 0], [2, 3840], [3, 7680], [4, 10560], [5, 13440]]);
});

it("ajusta à grade", () => {
  expect(snapToBeat(fourFour, 1000)).toBe(960);
  expect(snapToBeat(fourFour, 1500)).toBe(1920);
  expect(snapToBeat(fourFour, -50)).toBe(0);
});
