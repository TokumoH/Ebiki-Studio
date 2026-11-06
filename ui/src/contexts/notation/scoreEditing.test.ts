import type { NoteDuration, Score, ScoreVoice } from "../../ipc/types";
import { applyVoiceEdits, decomposeRests, durationFrac, keyAlter, keyToAction, moveCursor, noteEvent, pitchNear, placeEvent, restEvent, spellMidi, voiceFill, midiOf, eventLabel, snapHalfSpaces, staffPitch } from "./scoreEditing";

const d = (value: NoteDuration["value"], dots = 0): NoteDuration => ({ value, dots, tuplet: null });
const key = (k: string, mods: Partial<KeyboardEvent> = {}) => keyToAction({ key: k, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods });
const voice = (...events: ReturnType<typeof restEvent>[]): ScoreVoice => ({ staff: 0, events: events.map((e, i) => ({ ...e, id: i + 1 })) });
const C4 = { step: "C" as const, alter: 0, octave: 4 };

describe("mapa de teclado do editor de partitura", () => {
  it("A–G, 0, dígitos, ponto, setas, Delete e ligadura", () => {
    expect(key("g")).toEqual({ type: "note", step: "G" });
    expect(key("C")).toEqual({ type: "note", step: "C" });
    expect(key("0")).toEqual({ type: "rest" });
    expect(key("5")).toEqual({ type: "duration", value: "quarter" });
    expect(key("7")).toEqual({ type: "duration", value: "whole" });
    expect(key("4")).toEqual({ type: "duration", value: "eighth" });
    expect(key(".")).toEqual({ type: "dot" });
    expect(key("ArrowRight")).toEqual({ type: "move", delta: 1 });
    expect(key("ArrowUp")).toEqual({ type: "semitone", delta: 1 });
    expect(key("ArrowDown", { ctrlKey: true })).toEqual({ type: "semitone", delta: -12 });
    expect(key("Delete")).toEqual({ type: "delete" });
    expect(key("+")).toEqual({ type: "tie" });
    expect(key("h")).toBeNull();
    expect(key("c", { ctrlKey: true })).toBeNull(); // Ctrl+C não é nota
    expect(key("z", { ctrlKey: true })).toBeNull();
  });
});

describe("durações e pausas", () => {
  it("frações com pontos e quiálteras", () => {
    expect(durationFrac(d("quarter"))).toBe(0.25);
    expect(durationFrac(d("half", 1))).toBe(0.75);
    expect(durationFrac({ value: "eighth", dots: 0, tuplet: { actual: 3, normal: 2 } })).toBeCloseTo(1 / 12);
  });
  it("decompõe o resto em pausas da maior para a menor", () => {
    expect(decomposeRests(0.75).map((x) => x.value)).toEqual(["half", "quarter"]);
    expect(decomposeRests(1).map((x) => x.value)).toEqual(["whole"]);
    expect(decomposeRests(0)).toEqual([]);
  });
});

describe("placeEvent mantém o compasso cheio", () => {
  it("nota mais curta que a pausa: o resto vira pausas depois dela", () => {
    const v = voice(restEvent(d("whole")));
    const edits = placeEvent(v, 0, noteEvent(d("quarter"), C4), 1);
    expect(Array.isArray(edits)).toBe(true);
    const after = applyVoiceEdits(v, edits as never);
    expect(after.events.map((e) => e.kind.type)).toEqual(["chord", "rest", "rest"]);
    expect(voiceFill(after)).toBe(1);
  });
  it("nota mais longa consome pausas seguintes (parcialmente, se preciso)", () => {
    const v = voice(restEvent(d("quarter")), restEvent(d("half")), noteEvent(d("quarter"), C4));
    const edits = placeEvent(v, 0, noteEvent(d("half"), C4), 1) as never;
    const after = applyVoiceEdits(v, edits);
    expect(after.events.map((e) => `${e.kind.type}:${e.duration.value}`)).toEqual(["chord:half", "rest:quarter", "chord:quarter"]);
    expect(voiceFill(after)).toBe(1);
  });
  it("não apaga notas e recusa o que não cabe", () => {
    const v = voice(restEvent(d("quarter")), noteEvent(d("half"), C4), restEvent(d("quarter")));
    expect(placeEvent(v, 0, noteEvent(d("half"), C4), 1)).toMatchObject({ error: expect.stringContaining("apagar notas") });
    const full = voice(restEvent(d("half")), restEvent(d("half")));
    expect(placeEvent(full, 1, noteEvent(d("whole"), C4), 1)).toMatchObject({ error: expect.any(String) });
  });
  it("voz incompleta usa a folga no fim", () => {
    const v = voice(restEvent(d("quarter")));
    const after = applyVoiceEdits(v, placeEvent(v, 0, noteEvent(d("half"), C4), 1) as never);
    expect(voiceFill(after)).toBe(0.5);
  });
});

describe("alturas", () => {
  it("armadura aplica acidentes (Sol maior: Fá♯; Fá maior: Si♭)", () => {
    expect(keyAlter("F", 1)).toBe(1);
    expect(keyAlter("C", 1)).toBe(0);
    expect(keyAlter("B", -1)).toBe(-1);
    expect(pitchNear("F", { fifths: 1, mode: "major" }, null)).toEqual({ step: "F", alter: 1, octave: 4 });
  });
  it("oitava mais próxima da nota anterior", () => {
    expect(pitchNear("B", { fifths: 0, mode: "major" }, C4)).toEqual({ step: "B", alter: 0, octave: 3 });
    expect(pitchNear("D", { fifths: 0, mode: "major" }, C4)).toEqual({ step: "D", alter: 0, octave: 4 });
    expect(pitchNear("G", { fifths: 0, mode: "major" }, { step: "B", alter: 0, octave: 4 })).toEqual({ step: "G", alter: 0, octave: 4 });
  });
  it("semitom acima grafado pela armadura", () => {
    expect(spellMidi(66, { fifths: 1, mode: "major" })).toEqual({ step: "F", alter: 1, octave: 4 });
    expect(spellMidi(70, { fifths: -1, mode: "major" })).toEqual({ step: "B", alter: -1, octave: 4 });
    expect(spellMidi(61, { fifths: 0, mode: "major" })).toEqual({ step: "C", alter: 1, octave: 4 });
    expect(spellMidi(61, { fifths: -2, mode: "major" })).toEqual({ step: "D", alter: -1, octave: 4 });
    for (let m = 0; m < 128; m++) expect(midiOf(spellMidi(m, { fifths: 3, mode: "major" }))).toBe(m);
  });
  it("rótulo PT-BR do evento", () => {
    expect(eventLabel(noteEvent(d("quarter"), { step: "G", alter: 0, octave: 4 }))).toBe("Sol4 semínima");
    expect(eventLabel(restEvent(d("half", 1)))).toBe("pausa de mínima pontuada");
  });
});

describe("cursor de eventos", () => {
  const sc: Score = {
    title: "t",
    composer: "",
    tempo: { tempos: [{ tick: 0, bpm: 120 }], meters: [{ tick: 0, numerator: 4, denominator: 4 }] },
    nextEventId: 10,
    parts: [
      {
        name: "Voz",
        program: 0,
        percussion: false,
        spanners: [],
        staves: [{ clef: "treble", standard: true, tab: null }],
        measures: [0, 1].map(() => ({ key: { fifths: 0, mode: "major" as const }, meter: null, repeatStart: false, repeatEnd: null, voices: [voice(restEvent(d("half")), restEvent(d("half")))] })),
      },
    ],
  };
  it("atravessa compassos e para nas pontas", () => {
    const c = { part: 0, measure: 0, voice: 0, index: 1 };
    expect(moveCursor(sc, c, 1)).toMatchObject({ measure: 1, index: 0 });
    expect(moveCursor(sc, { ...c, measure: 1, index: 0 }, -1)).toMatchObject({ measure: 0, index: 1 });
    expect(moveCursor(sc, { ...c, measure: 1, index: 1 }, 1)).toMatchObject({ measure: 1, index: 1 });
    expect(moveCursor(sc, { ...c, index: 0 }, -1)).toMatchObject({ measure: 0, index: 0 });
  });
});

describe("clique na pauta", () => {
  const C = { fifths: 0, mode: "major" as const };
  it("linhas e espaços da clave de Sol", () => {
    expect(staffPitch("treble", 0, C)).toEqual({ step: "F", alter: 0, octave: 5 }); // linha de cima
    expect(staffPitch("treble", 8, C)).toEqual({ step: "E", alter: 0, octave: 4 }); // linha de baixo
    expect(staffPitch("treble", 10, C)).toEqual({ step: "C", alter: 0, octave: 4 }); // 1ª suplementar inferior
    expect(staffPitch("treble", -2, C)).toEqual({ step: "A", alter: 0, octave: 5 }); // 1ª suplementar superior
  });
  it("Fá, Dó e 8vb", () => {
    expect(staffPitch("bass", 0, C)).toEqual({ step: "A", alter: 0, octave: 3 });
    expect(staffPitch("bass", 8, C)).toEqual({ step: "G", alter: 0, octave: 2 });
    expect(staffPitch("alto", 4, C)).toEqual({ step: "C", alter: 0, octave: 4 }); // linha do meio = Dó central
    expect(staffPitch("treble8vb", 8, C)).toEqual({ step: "E", alter: 0, octave: 3 });
  });
  it("armadura e acidente escolhido", () => {
    const G = { fifths: 1, mode: "major" as const };
    expect(staffPitch("treble", 0, G).alter).toBe(1); // Fá♯ pela armadura
    expect(staffPitch("treble", 0, G, 0).alter).toBe(0); // bequadro escolhido
    expect(staffPitch("treble", 9, C, -1)).toEqual({ step: "D", alter: -1, octave: 4 });
  });
  it("arredonda para a linha ou espaço mais próximo e limita as suplementares", () => {
    expect(snapHalfSpaces(100, 100, 8)).toBe(0);
    expect(snapHalfSpaces(103.9, 100, 8)).toBe(1);
    expect(snapHalfSpaces(102.1, 100, 8)).toBe(1);
    expect(snapHalfSpaces(101.9, 100, 8)).toBe(0);
    expect(snapHalfSpaces(-500, 100, 8)).toBe(-12);
    expect(snapHalfSpaces(900, 100, 8)).toBe(20);
  });
});
