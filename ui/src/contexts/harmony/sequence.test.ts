import { appendChord, historyOf, label, MAX_SEQUENCE, removeLast, scoreBars, toCifraText } from "./sequence";

const G7 = { symbol: "G7", roman: "V7" };
const C = { symbol: "C", roman: "I" };

describe("construtor de sequência", () => {
  it("acrescenta sem mutar e remove o último", () => {
    const a: { symbol: string; roman: string }[] = [];
    const b = appendChord(a, G7);
    const c = appendChord(b, C);
    expect(a).toEqual([]);
    expect(historyOf(c)).toEqual(["G7", "C"]);
    expect(historyOf(removeLast(c))).toEqual(["G7"]);
    expect(removeLast([])).toEqual([]);
  });

  it("respeita o limite de passos do domínio", () => {
    let s: { symbol: string; roman: string }[] = [];
    for (let i = 0; i < MAX_SEQUENCE + 5; i++) s = appendChord(s, C);
    expect(s).toHaveLength(MAX_SEQUENCE);
  });

  it("gera texto de cifra com tempos limitados", () => {
    expect(toCifraText([G7, C])).toBe("G7:4 C:4");
    expect(toCifraText([G7], 2)).toBe("G7:2");
    expect(toCifraText([G7], 0)).toBe("G7:1");
    expect(toCifraText([G7], 1000)).toBe("G7:64");
    expect(toCifraText([G7], Number.NaN)).toBe("G7:4");
    expect(toCifraText([])).toBe("");
  });

  it("alterna notação e converte pontuação em barras", () => {
    expect(label(G7, "cifra")).toBe("G7");
    expect(label(G7, "roman")).toBe("V7");
    expect(scoreBars(1)).toBe(5);
    expect(scoreBars(0.62)).toBe(3);
    expect(scoreBars(0)).toBe(1);
    expect(scoreBars(Number.NaN)).toBe(1);
  });
});
