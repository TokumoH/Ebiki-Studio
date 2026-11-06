// Entrada de notas por texto corrido (mockup 8): "C, C, C, x, x, x, D, D, D, E5:8, F#:8, C+E+G:2".
// Função pura: texto → eventos, ou um erro que aponta o trecho. Nada é inserido com erro.
//
// Gramática (sem diferenciar maiúsculas/acentos; separadores: vírgula, espaço, | e ;):
//   evento  := pausa | acorde
//   pausa   := ("x" | "pausa") duração?
//   acorde  := nota ("+" nota)* duração?
//   nota    := nome acidente? oitava?      nome: A–G ou Dó Ré Mi Fá Sol Lá Si
//   acidente:= # | ## | b | bb | ♯ | ♭ | n | ♮   (n/♮ = bequadro, ignora a armadura)
//   oitava  := 0–8 (sem oitava: a mais próxima da nota anterior)
//   duração := ":" (1|2|4|8|16|32|64) "."?   (sem duração: a selecionada na barra)
import type { KeySignature, NoteDuration, NoteValue, Pitch, ScoreEvent, Step } from "../../ipc/types";
import { keyAlter, midiOf, noteEvent, restEvent } from "./scoreEditing";

export interface ParseOptions {
  /** Duração para eventos sem ":valor". */
  duration: NoteDuration;
  key: KeySignature;
  /** Nota antes do cursor (para escolher a oitava mais próxima). */
  previous: Pitch | null;
  defaultOctave: number;
}

export interface ParseError {
  message: string;
  /** Trecho com problema, como digitado. */
  token: string;
  /** Posição do trecho na lista (1 = primeiro). */
  index: number;
  /** Deslocamento do trecho no texto (para destacar). */
  start: number;
}

export type ParseResult = { ok: true; events: ScoreEvent[] } | { ok: false; error: ParseError };

/** Lote máximo por inserção (evita travar a interface com textos enormes). */
export const MAX_BATCH = 512;

const NAMES: Record<string, Step> = {
  c: "C", d: "D", e: "E", f: "F", g: "G", a: "A", b: "B",
  do: "C", re: "D", mi: "E", fa: "F", sol: "G", la: "A", si: "B",
};

const DENOMINATORS: Record<string, NoteValue> = {
  "1": "whole", "2": "half", "4": "quarter", "8": "eighth", "16": "sixteenth", "32": "thirtySecond", "64": "sixtyFourth",
};

const ACCIDENTALS: Record<string, number | "natural"> = { "#": 1, "##": 2, "♯": 1, b: -1, bb: -2, "♭": -1, n: "natural", "♮": "natural" };

const NOTE_RE = /^(sol|do|re|mi|fa|la|si|[a-g])(##|bb|#|b|♯|♭|n|♮)?([0-8])?$/;

const fold = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

interface Token {
  text: string;
  start: number;
}

function tokenize(text: string): Token[] {
  const out: Token[] = [];
  const re = /[^\s,|;]+/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) out.push({ text: m[0], start: m.index });
  return out;
}

/** Altura com o nome/acidente pedidos, na oitava mais próxima de `ref` (MIDI). */
function nearest(step: Step, alter: number, ref: number): Pitch {
  let best: Pitch = { step, alter, octave: 4 };
  let bestDist = Infinity;
  for (let octave = 0; octave <= 8; octave++) {
    const p = { step, alter, octave };
    const d = Math.abs(midiOf(p) - ref);
    if (d < bestDist) {
      best = p;
      bestDist = d;
    }
  }
  return best;
}

type NoteSpec = { step: Step; accidental: number | "natural" | null; octave: number | null };

function parseNote(raw: string): NoteSpec | null {
  const m = NOTE_RE.exec(fold(raw));
  if (!m) return null;
  const accidental = m[2] ? ACCIDENTALS[m[2]]! : null;
  return { step: NAMES[m[1]!]!, accidental, octave: m[3] !== undefined ? Number(m[3]) : null };
}

function resolve(n: NoteSpec, key: KeySignature, ref: number | null, defaultOctave: number): Pitch {
  const alter = n.accidental === null ? keyAlter(n.step, key.fifths) : n.accidental === "natural" ? 0 : n.accidental;
  if (n.octave !== null) return { step: n.step, alter, octave: n.octave };
  if (ref === null) return { step: n.step, alter, octave: defaultOctave };
  return nearest(n.step, alter, ref);
}

export function parseNoteText(text: string, o: ParseOptions): ParseResult {
  const tokens = tokenize(text);
  if (tokens.length > MAX_BATCH) {
    const t = tokens[MAX_BATCH]!;
    return { ok: false, error: { message: `No máximo ${MAX_BATCH} eventos por vez.`, token: t.text, index: MAX_BATCH + 1, start: t.start } };
  }
  const events: ScoreEvent[] = [];
  let prev: number | null = o.previous ? midiOf(o.previous) : null;
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i]!;
    const fail = (why: string): ParseResult => ({ ok: false, error: { message: `"${t.text}" na posição ${i + 1}: ${why}`, token: t.text, index: i + 1, start: t.start } });
    const colon = t.text.indexOf(":");
    const head = colon < 0 ? t.text : t.text.slice(0, colon);
    let duration = o.duration;
    if (colon >= 0) {
      const dm = /^(\d+)(\.)?$/.exec(t.text.slice(colon + 1));
      const value = dm ? DENOMINATORS[dm[1]!] : undefined;
      if (!dm || !value) return fail("duração inválida (use :1 :2 :4 :8 :16 :32 :64, com . para pontuar).");
      duration = { value, dots: dm[2] ? 1 : 0, tuplet: null };
    }
    if (!head) return fail("falta a nota antes da duração.");
    const folded = fold(head);
    if (folded === "x" || folded === "pausa") {
      events.push(restEvent(duration));
      continue;
    }
    const parts = head.split("+");
    const specs: NoteSpec[] = [];
    for (const p of parts) {
      if (!p) return fail("acorde incompleto (use C+E+G).");
      if (fold(p) === "x" || fold(p) === "pausa") return fail("pausa não pode fazer parte de um acorde.");
      const n = parseNote(p);
      if (!n) return fail("não é uma nota (use A–G, Dó–Si ou x para pausa; oitava de 0 a 8).");
      specs.push(n);
    }
    // primeira nota: perto da anterior; as demais sobem a partir da nota de baixo do acorde
    const pitches: Pitch[] = [];
    let ref = prev;
    for (const [k, n] of specs.entries()) {
      let p = resolve(n, o.key, ref, o.defaultOctave);
      if (k > 0 && n.octave === null && midiOf(p) < midiOf(pitches[k - 1]!)) p = { ...p, octave: p.octave + 1 };
      pitches.push(p);
      ref = midiOf(p);
    }
    const ev = noteEvent(duration, pitches[0]!);
    if (ev.kind.type === "chord") ev.kind.notes = pitches.map((pitch) => ({ pitch, tie: false, tab: null, techniques: [] }));
    events.push(ev);
    prev = midiOf(pitches[0]!);
  }
  return { ok: true, events };
}
