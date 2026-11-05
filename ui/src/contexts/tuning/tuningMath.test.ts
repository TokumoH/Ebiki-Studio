import { centsOff, centsToAngle, dragEdit, effectiveMidi, frequencyOf, isInTune, midiName, pitchRange, scaleClasses, scaleKind } from "./tuningMath";

const note = { id: "n1", startFrame: 0, endFrame: 4800, midi: 69.47 };

describe("afinador", () => {
  it("ponteiro: ±50 cents → ±45°, satura e ignora valores inválidos", () => {
    expect(centsToAngle(0)).toBe(0);
    expect(centsToAngle(25)).toBe(22.5);
    expect(centsToAngle(-50)).toBe(-45);
    expect(centsToAngle(120)).toBe(45);
    expect(centsToAngle(Number.NaN)).toBe(0);
  });
  it("tolerância de afinado e frequências", () => {
    expect(isInTune(4.9)).toBe(true);
    expect(isInTune(-6)).toBe(false);
    expect(frequencyOf(69)).toBe(440);
    expect(frequencyOf(81, 442)).toBeCloseTo(884);
    expect(midiName(60)).toBe("Dó4");
    expect(midiName(69.47)).toBe("Lá4");
    expect(centsOff(69.47)).toBe(47);
    expect(centsOff(68.6)).toBe(-40);
  });
});

describe("editor de afinação: arraste → edição", () => {
  it("arraste normal encaixa no semitom a partir da altura efetiva", () => {
    expect(dragEdit(note, undefined, 0.2, false)).toMatchObject({ target: 70, centsOffset: 0 });
    expect(dragEdit(note, undefined, -0.4, false)).toMatchObject({ target: 69, centsOffset: 0 });
    const e = dragEdit(note, { target: 69, centsOffset: 0, flatten: 0.5, formantSemitones: 0 }, 2, false);
    expect(e).toEqual({ target: 71, centsOffset: 0, flatten: 0.5, formantSemitones: 0 });
    expect(effectiveMidi(note, e)).toBe(71);
  });
  it("arraste fino (Shift) ajusta cents relativos à nota detectada, limitado a ±1200", () => {
    const e = dragEdit(note, undefined, -0.47, true);
    expect(e).toMatchObject({ target: null, centsOffset: -47 });
    expect(effectiveMidi(note, e)).toBeCloseTo(69);
    expect(dragEdit(note, undefined, 30, true).centsOffset).toBe(1200);
  });
  it("limites MIDI 0–127", () => {
    expect(dragEdit({ ...note, midi: 126.8 }, undefined, 5, false).target).toBe(127);
    expect(dragEdit({ ...note, midi: 0.3 }, undefined, -5, false).target).toBe(0);
  });
});

describe("escalas e faixa do piano-roll", () => {
  it("classes por tônica e tipo; reconhece o tipo", () => {
    expect(scaleClasses(7, "major")).toEqual([0, 2, 4, 6, 7, 9, 11]);
    expect(scaleClasses(9, "minor")).toEqual([0, 2, 4, 5, 7, 9, 11]);
    expect(scaleKind(0, [0, 2, 4, 5, 7, 9, 11])).toBe("major");
    expect(scaleKind(9, [0, 2, 4, 5, 7, 9, 11])).toBe("minor");
    expect(scaleKind(0, [0, 3])).toBe("custom");
  });
  it("faixa com margem e no mínimo uma oitava", () => {
    const r = pitchRange([note], {});
    expect(r.high - r.low).toBeGreaterThanOrEqual(12);
    expect(r.low).toBeLessThan(69);
    expect(r.high).toBeGreaterThan(70);
    expect(pitchRange([], {})).toEqual({ low: 57, high: 72 });
  });
});
