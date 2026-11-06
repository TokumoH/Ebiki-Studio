import { describe, expect, it } from "vitest";
import type { NoteDuration, Pitch } from "../../ipc/types";
import { parseNoteText, type ParseOptions } from "./noteText";

const Q: NoteDuration = { value: "quarter", dots: 0, tuplet: null };
const C_MAJOR = { fifths: 0, mode: "major" as const };
const opts = (o: Partial<ParseOptions> = {}): ParseOptions => ({ duration: Q, key: C_MAJOR, previous: null, defaultOctave: 4, ...o });

/** Resumo legível dos eventos: "C4/4", "x/4", "C4+E4+G4/2", "D5/8." */
function summary(text: string, o: Partial<ParseOptions> = {}): string[] {
  const r = parseNoteText(text, opts(o));
  if (!r.ok) throw new Error(r.error.message);
  const den = { whole: 1, half: 2, quarter: 4, eighth: 8, sixteenth: 16, thirtySecond: 32, sixtyFourth: 64, oneTwentyEighth: 128 } as const;
  const name = (p: Pitch) => `${p.step}${p.alter > 0 ? "#".repeat(p.alter) : "b".repeat(-p.alter)}${p.octave}`;
  return r.events.map((e) => {
    const d = `/${den[e.duration.value]}${e.duration.dots ? "." : ""}`;
    return e.kind.type === "rest" ? `x${d}` : e.kind.notes.map((n) => name(n.pitch)).join("+") + d;
  });
}

describe("parseNoteText", () => {
  it("o exemplo do usuário: notas, pausas com x, duração selecionada", () => {
    expect(summary("C,C,C,x,x,x,D,D,D")).toEqual(["C4/4", "C4/4", "C4/4", "x/4", "x/4", "x/4", "D4/4", "D4/4", "D4/4"]);
  });

  it("aceita vírgula, espaço, barra de compasso e ponto e vírgula como separadores", () => {
    expect(summary("C D | E;F,  G")).toEqual(["C4/4", "D4/4", "E4/4", "F4/4", "G4/4"]);
  });

  it("sem oitava, escolhe a mais próxima da nota anterior (como digitar no teclado)", () => {
    expect(summary("C B")).toEqual(["C4/4", "B3/4"]);
    expect(summary("G E", { previous: { step: "A", alter: 0, octave: 5 } })).toEqual(["G5/4", "E5/4"]);
  });

  it("oitava explícita, sustenido e bemol (também ♯ ♭) e bequadro", () => {
    expect(summary("C5 D# B♭ G♯3 Eb")).toEqual(["C5/4", "D#5/4", "Bb4/4", "G#3/4", "Eb3/4"]);
    // em Sol maior o Fá é sustenido pela armadura; "Fn" ou "F♮" força o natural
    expect(summary("F Fn F♮", { key: { fifths: 1, mode: "major" } })).toEqual(["F#4/4", "F4/4", "F4/4"]);
  });

  it("duração por nota com :valor e ponto de aumento", () => {
    expect(summary("C:8 D:8 E:2 F:4. x:16 G:1")).toEqual(["C4/8", "D4/8", "E4/2", "F4/4.", "x/16", "G4/1"]);
  });

  it("acordes com + (notas sobem a partir da primeira)", () => {
    expect(summary("C+E+G:2 G3+B+D")).toEqual(["C4+E4+G4/2", "G3+B3+D4/4"]);
  });

  it("solfejo em português, sem diferenciar maiúsculas e acentos", () => {
    expect(summary("dó ré mi fá sol lá si DO re:8")).toEqual(["C4/4", "D4/4", "E4/4", "F4/4", "G4/4", "A4/4", "B4/4", "C5/4", "D5/8"]);
    expect(summary("X pausa:2")).toEqual(["x/4", "x/2"]);
  });

  it("texto vazio não gera eventos (sem erro)", () => {
    const r = parseNoteText("  , | ", opts());
    expect(r).toEqual({ ok: true, events: [] });
  });

  it("erros apontam o trecho e a posição, sem eventos parciais", () => {
    const r = parseNoteText("C, D, H, E", opts());
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.token).toBe("H");
      expect(r.error.index).toBe(3);
      expect(r.error.start).toBe(6);
      expect(r.error.message).toMatch(/H.*posição 3/);
    }
    for (const bad of ["C:3", "C:", "C9", "C##b", "x+C", "C+", ":4"]) expect(parseNoteText(bad, opts()).ok, bad).toBe(false);
  });

  it("limita o tamanho do lote", () => {
    const r = parseNoteText(Array(600).fill("C").join(","), opts());
    expect(r.ok).toBe(false);
  });
});
