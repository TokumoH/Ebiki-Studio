// Matemática pura do afinador e do editor de afinação (ponteiro, alturas, arraste → edição).
import type { DetectedNote, NoteEdit } from "../../ipc/types";

export const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
export const NOTE_NAMES_PT = ["Dó", "Dó♯", "Ré", "Ré♯", "Mi", "Fá", "Fá♯", "Sol", "Sol♯", "Lá", "Lá♯", "Si"];

/** Ângulo do ponteiro (graus): ±50 cents → ±`maxDeg`, saturando nos extremos. */
export function centsToAngle(cents: number, maxDeg = 45): number {
  if (!Number.isFinite(cents)) return 0;
  return (Math.max(-50, Math.min(50, cents)) / 50) * maxDeg;
}

/** Afinado quando o desvio está dentro da tolerância (padrão ±5 cents). */
export function isInTune(cents: number, tolerance = 5): boolean {
  return Math.abs(cents) <= tolerance;
}

export function frequencyOf(midi: number, a4Hz = 440): number {
  return a4Hz * Math.pow(2, (midi - 69) / 12);
}

/** Nome PT-BR com oitava (ex.: 69 → "Lá4"); aceita MIDI fracionário (arredonda). */
export function midiName(midi: number): string {
  const n = Math.round(midi);
  return `${NOTE_NAMES_PT[((n % 12) + 12) % 12]}${Math.floor(n / 12) - 1}`;
}

/** Desvio em cents da nota detectada em relação ao semitom mais próximo. */
export function centsOff(midi: number): number {
  return Math.round((midi - Math.round(midi)) * 100);
}

/** Altura resultante (MIDI fracionário) de uma nota com edição (mesma regra do ctx-tuning). */
export function effectiveMidi(note: DetectedNote, edit: NoteEdit | undefined): number {
  if (!edit) return note.midi;
  return (edit.target ?? note.midi) + edit.centsOffset / 100;
}

export const NEUTRAL_EDIT: NoteEdit = { target: null, centsOffset: 0, flatten: 0, formantSemitones: 0 };

/**
 * Arraste vertical de `deltaSemis` semitons a partir da altura efetiva atual.
 * Normal: encaixa no semitom (alvo inteiro, sem cents). Fino: ajuste em cents relativo à nota
 * detectada (sem alvo), limitado a ±1200. Preserva achatamento e formante.
 */
export function dragEdit(note: DetectedNote, edit: NoteEdit | undefined, deltaSemis: number, fine: boolean): NoteEdit {
  const base = edit ?? NEUTRAL_EDIT;
  const from = effectiveMidi(note, edit);
  if (fine) {
    const cents = Math.round((from + deltaSemis - note.midi) * 100);
    return { ...base, target: null, centsOffset: Math.max(-1200, Math.min(1200, cents)) };
  }
  const target = Math.max(0, Math.min(127, Math.round(from + deltaSemis)));
  return { ...base, target, centsOffset: 0 };
}

/** Classes de altura permitidas para uma tônica e um tipo de escala. */
export function scaleClasses(key: number, kind: "major" | "minor" | "chromatic"): number[] {
  const iv = kind === "major" ? [0, 2, 4, 5, 7, 9, 11] : kind === "minor" ? [0, 2, 3, 5, 7, 8, 10] : [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
  return iv.map((i) => (key + i) % 12).sort((a, b) => a - b);
}

/** Identifica o tipo de escala a partir das classes (para os seletores). */
export function scaleKind(key: number, allowed: number[]): "major" | "minor" | "chromatic" | "custom" {
  for (const k of ["major", "minor", "chromatic"] as const) if (JSON.stringify(scaleClasses(key, k)) === JSON.stringify([...allowed].sort((a, b) => a - b))) return k;
  return "custom";
}

/** Faixa vertical do piano-roll (com margem), mínimo de uma oitava. */
export function pitchRange(notes: DetectedNote[], edits: Record<string, NoteEdit>): { low: number; high: number } {
  if (notes.length === 0) return { low: 57, high: 72 };
  const all = notes.flatMap((n) => [n.midi, effectiveMidi(n, edits[n.id])]);
  let low = Math.floor(Math.min(...all)) - 2;
  let high = Math.ceil(Math.max(...all)) + 2;
  if (high - low < 12) {
    const pad = Math.ceil((12 - (high - low)) / 2);
    low -= pad;
    high += pad;
  }
  return { low: Math.max(0, low), high: Math.min(127, high) };
}
