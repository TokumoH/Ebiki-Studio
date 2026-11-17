import { formatBarBeat, formatDb, formatDecimal, formatPan, formatTime } from "./format";

it("formata valores para exibição", () => {
  expect(formatDb(-144)).toBe("−∞");
  expect(formatDb(-6)).toBe("−6,0");
  expect(formatDb(3.25)).toBe("+3,3");
  expect(formatTime(65.5)).toBe("01:05.500");
  expect(formatBarBeat({ bar: 5, beat: 2, tickInBeat: 48 })).toBe("005.2.048");
  expect(formatPan(0)).toBe("C");
  expect(formatPan(-0.5)).toBe("E50");
  expect(formatPan(1)).toBe("D100");
});

it("decimal com vírgula (UX-15)", () => {
  expect(formatDecimal(10.66)).toBe("10,7");
  expect(formatDecimal(120, 1)).toBe("120,0");
  expect(formatDecimal(-0.25, 2)).toBe("-0,25");
});
