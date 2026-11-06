// Regras de edição da partitura no editor (puras, testáveis): teclado → ação, cursor de eventos,
// altura pela armadura e redistribuição de pausas para manter o compasso cheio.
import type { Clef, KeySignature, NoteDuration, NoteValue, Pitch, Score, ScoreEvent, ScoreVoice, Step, VoiceEdit } from "../../ipc/types";

export interface Cursor {
  part: number;
  measure: number;
  voice: number;
  index: number;
}

export type ScoreAction =
  | { type: "note"; step: Step }
  | { type: "rest" }
  | { type: "duration"; value: NoteValue }
  | { type: "dot" }
  | { type: "move"; delta: -1 | 1 }
  | { type: "semitone"; delta: number }
  | { type: "delete" }
  | { type: "tie" };

/** Dígitos de duração no estilo MuseScore (5 = semínima). */
export const DIGIT_DURATIONS: Record<string, NoteValue> = {
  "1": "sixtyFourth",
  "2": "thirtySecond",
  "3": "sixteenth",
  "4": "eighth",
  "5": "quarter",
  "6": "half",
  "7": "whole",
};

export const DURATION_LABELS: Record<NoteValue, string> = {
  whole: "Semibreve",
  half: "Mínima",
  quarter: "Semínima",
  eighth: "Colcheia",
  sixteenth: "Semicolcheia",
  thirtySecond: "Fusa",
  sixtyFourth: "Semifusa",
  oneTwentyEighth: "Quartifusa",
};

export const STEP_NAMES: Record<Step, string> = { C: "Dó", D: "Ré", E: "Mi", F: "Fá", G: "Sol", A: "Lá", B: "Si" };

/** Mapeia uma tecla para a ação do editor (null = tecla não é do editor). */
export function keyToAction(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">): ScoreAction | null {
  const k = e.key;
  const mod = e.ctrlKey || e.metaKey;
  if (k === "ArrowLeft") return { type: "move", delta: -1 };
  if (k === "ArrowRight") return { type: "move", delta: 1 };
  if (k === "ArrowUp") return { type: "semitone", delta: mod ? 12 : 1 };
  if (k === "ArrowDown") return { type: "semitone", delta: mod ? -12 : -1 };
  if (mod || e.altKey) return null;
  if (k === "Delete" || k === "Backspace") return { type: "delete" };
  if (/^[a-gA-G]$/.test(k)) return { type: "note", step: k.toUpperCase() as Step };
  if (k === "0") return { type: "rest" };
  if (DIGIT_DURATIONS[k]) return { type: "duration", value: DIGIT_DURATIONS[k]! };
  if (k === ".") return { type: "dot" };
  if (k === "+") return { type: "tie" };
  return null;
}

const VALUE_FRAC: Record<NoteValue, number> = { whole: 1, half: 1 / 2, quarter: 1 / 4, eighth: 1 / 8, sixteenth: 1 / 16, thirtySecond: 1 / 32, sixtyFourth: 1 / 64, oneTwentyEighth: 1 / 128 };
const EPS = 1e-9;

/** Duração em semibreves (pontos e quiálteras incluídos). */
export function durationFrac(d: NoteDuration): number {
  let f = VALUE_FRAC[d.value];
  let add = f;
  for (let i = 0; i < d.dots; i++) {
    add /= 2;
    f += add;
  }
  return d.tuplet ? (f * d.tuplet.normal) / d.tuplet.actual : f;
}

const eventFrac = (e: ScoreEvent) => (e.grace ? 0 : durationFrac(e.duration));

/** Pausas (sem ponto) que somam `frac` semibreves, da maior para a menor. */
export function decomposeRests(frac: number): NoteDuration[] {
  const out: NoteDuration[] = [];
  let left = frac;
  for (const v of Object.keys(VALUE_FRAC) as NoteValue[]) {
    while (left + EPS >= VALUE_FRAC[v]) {
      out.push({ value: v, dots: 0, tuplet: null });
      left -= VALUE_FRAC[v];
    }
  }
  return out;
}

export function restEvent(duration: NoteDuration): ScoreEvent {
  return { id: 0, duration, kind: { type: "rest" }, articulations: [], dynamic: null, grace: null, lyric: null };
}

export function noteEvent(duration: NoteDuration, pitch: Pitch): ScoreEvent {
  return { id: 0, duration, kind: { type: "chord", notes: [{ pitch, tie: false, tab: null, techniques: [] }] }, articulations: [], dynamic: null, grace: null, lyric: null };
}

/** Capacidade (semibreves) do compasso `measure`, pela fórmula vigente. */
export function measureCapacity(score: Score, measure: number): number {
  const m = meterAt(score, measure);
  return m.numerator / m.denominator;
}

export function meterAt(score: Score, measure: number): { numerator: number; denominator: number } {
  const m0 = score.tempo.meters[0] ?? { numerator: 4, denominator: 4 };
  const ms = score.parts[0]?.measures ?? [];
  let cur = { numerator: m0.numerator, denominator: m0.denominator };
  for (let i = 0; i <= measure && i < ms.length; i++) if (ms[i]!.meter) cur = ms[i]!.meter!;
  return cur;
}

/**
 * Edições que colocam `ev` na posição `index` mantendo a soma do compasso: o evento atual é
 * substituído; se `ev` é mais longo, consome as pausas seguintes (nunca notas); se é mais curto,
 * o resto vira pausas logo depois dele. Erro (texto PT-BR) se não couber.
 */
export function placeEvent(voice: ScoreVoice, index: number, ev: ScoreEvent, capacity: number): VoiceEdit[] | { error: string } {
  const events = voice.events;
  const cur = events[index];
  if (!cur) return { error: "Posição inválida." };
  const edits: VoiceEdit[] = [{ kind: "replace", index, event: ev }];
  let delta = eventFrac(ev) - eventFrac(cur);
  if (delta < -EPS) {
    decomposeRests(-delta).forEach((d, i) => edits.push({ kind: "insert", index: index + 1 + i, event: restEvent(d) }));
    return edits;
  }
  const total = events.reduce((a, e) => a + eventFrac(e), 0);
  let slack = Math.max(0, capacity - total);
  // percorre os eventos seguintes da voz original; cada remoção acontece em `index + 1`
  let k = index + 1;
  while (delta > EPS) {
    const nx = events[k];
    if (!nx) {
      if (slack + EPS >= delta) {
        slack -= delta;
        delta = 0;
        break;
      }
      return { error: "A duração não cabe no compasso." };
    }
    if (nx.kind.type !== "rest") return { error: "A duração não cabe no compasso sem apagar notas." };
    const d = eventFrac(nx);
    const at = index + 1; // posição do próximo evento depois das remoções anteriores
    if (d <= delta + EPS) {
      edits.push({ kind: "delete", index: at });
      delta -= d;
    } else {
      edits.push({ kind: "delete", index: at });
      decomposeRests(d - delta).forEach((r, i) => edits.push({ kind: "insert", index: at + i, event: restEvent(r) }));
      delta = 0;
    }
    k++;
  }
  return edits;
}

/** Aplica edições de voz localmente (mesma semântica do núcleo) — usado em testes e prévias. */
export function applyVoiceEdits(voice: ScoreVoice, edits: VoiceEdit[]): ScoreVoice {
  const events = [...voice.events];
  for (const e of edits) {
    if (e.kind === "replace") events[e.index] = e.event;
    else if (e.kind === "insert") events.splice(e.index, 0, e.event);
    else if (e.kind === "delete") events.splice(e.index, 1);
  }
  return { ...voice, events };
}

export function voiceFill(voice: ScoreVoice): number {
  return voice.events.reduce((a, e) => a + eventFrac(e), 0);
}

// ------------------------------------------------------------------------- alturas

const STEP_PC: Record<Step, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const SHARPS: Step[] = ["F", "C", "G", "D", "A", "E", "B"];
const FLATS: Step[] = ["B", "E", "A", "D", "G", "C", "F"];

export function midiOf(p: Pitch): number {
  return (p.octave + 1) * 12 + STEP_PC[p.step] + p.alter;
}

/** Acidente da armadura para um nome de nota (−1, 0, +1). */
export function keyAlter(step: Step, fifths: number): number {
  if (fifths > 0 && SHARPS.slice(0, fifths).includes(step)) return 1;
  if (fifths < 0 && FLATS.slice(0, -fifths).includes(step)) return -1;
  return 0;
}

/** Nota com o nome `step`, acidente da armadura, na oitava mais próxima de `prev`. */
export function pitchNear(step: Step, key: KeySignature, prev: Pitch | null, defaultOctave = 4): Pitch {
  const alter = keyAlter(step, key.fifths);
  if (!prev) return { step, alter, octave: defaultOctave };
  const ref = midiOf(prev);
  let best: Pitch = { step, alter, octave: defaultOctave };
  let bestDist = Infinity;
  for (let octave = 0; octave <= 9; octave++) {
    const p = { step, alter, octave };
    const m = midiOf(p);
    if (m < 0 || m > 127) continue;
    const d = Math.abs(m - ref);
    if (d < bestDist) {
      best = p;
      bestDist = d;
    }
  }
  return best;
}

/** Grafia de uma altura MIDI preferindo as notas da armadura (sustenidos/bemóis pelo tom). */
export function spellMidi(midi: number, key: KeySignature): Pitch {
  const pc = ((midi % 12) + 12) % 12;
  const steps = Object.keys(STEP_PC) as Step[];
  // 1) nota diatônica no tom
  for (const step of steps) {
    const alter = keyAlter(step, key.fifths);
    if ((STEP_PC[step] + alter + 12) % 12 === pc) return { step, alter, octave: Math.floor((midi - STEP_PC[step] - alter) / 12) - 1 };
  }
  // 2) cromática: sustenidos em tons com sustenidos (ou Dó), bemóis em tons com bemóis
  const useFlat = key.fifths < 0;
  for (const step of steps) {
    const alter = useFlat ? -1 : 1;
    if ((STEP_PC[step] + alter + 12) % 12 === pc) return { step, alter, octave: Math.floor((midi - STEP_PC[step] - alter) / 12) - 1 };
  }
  const step = steps.find((s) => STEP_PC[s] === pc)!;
  return { step, alter: 0, octave: Math.floor(midi / 12) - 1 };
}

const ALTER_LABEL: Record<number, string> = { [-2]: "𝄫", [-1]: "♭", 0: "", 1: "♯", 2: "𝄪" };

export function pitchLabel(p: Pitch): string {
  return `${STEP_NAMES[p.step]}${ALTER_LABEL[p.alter] ?? ""}${p.octave}`;
}

export function eventLabel(e: ScoreEvent): string {
  const dur = `${DURATION_LABELS[e.duration.value].toLowerCase()}${e.duration.dots ? " pontuada" : ""}`;
  if (e.kind.type === "rest") return `pausa de ${dur}`;
  return `${e.kind.notes.map((n) => pitchLabel(n.pitch)).join("+")} ${dur}`;
}

// -------------------------------------------------------------------------- cursor

export function voiceAt(score: Score, c: Cursor): ScoreVoice | null {
  return score.parts[c.part]?.measures[c.measure]?.voices[c.voice] ?? null;
}

export function eventAt(score: Score, c: Cursor): ScoreEvent | null {
  return voiceAt(score, c)?.events[c.index] ?? null;
}

/** Mantém o cursor dentro da partitura (após edições/desfazer). */
export function clampCursor(score: Score, c: Cursor): Cursor {
  const part = Math.min(Math.max(0, c.part), Math.max(0, score.parts.length - 1));
  const measures = score.parts[part]?.measures ?? [];
  const measure = Math.min(Math.max(0, c.measure), Math.max(0, measures.length - 1));
  const voices = measures[measure]?.voices ?? [];
  const voice = Math.min(Math.max(0, c.voice), Math.max(0, voices.length - 1));
  const len = voices[voice]?.events.length ?? 0;
  return { part, measure, voice, index: Math.min(Math.max(0, c.index), Math.max(0, len - 1)) };
}

/** Move o cursor `delta` eventos, atravessando compassos (mesma parte/voz). */
export function moveCursor(score: Score, c: Cursor, delta: number): Cursor {
  let { measure, index } = c;
  const measures = score.parts[c.part]?.measures ?? [];
  const len = (m: number) => measures[m]?.voices[c.voice]?.events.length ?? 0;
  index += delta;
  while (index < 0 && measure > 0) {
    measure--;
    index += Math.max(1, len(measure));
  }
  while (index >= len(measure) && measure < measures.length - 1) {
    index -= Math.max(1, len(measure));
    measure++;
  }
  return clampCursor(score, { ...c, measure, index });
}

/** Última altura antes do cursor na mesma voz (referência de oitava para a próxima nota). */
export function previousPitch(score: Score, c: Cursor): Pitch | null {
  const measures = score.parts[c.part]?.measures ?? [];
  for (let m = c.measure; m >= 0; m--) {
    const events = measures[m]?.voices[c.voice]?.events ?? [];
    const from = m === c.measure ? Math.min(c.index, events.length - 1) : events.length - 1;
    for (let i = from; i >= 0; i--) {
      const e = events[i]!;
      if (e.kind.type === "chord" && e.kind.notes[0]) return e.kind.notes[0].pitch;
    }
  }
  return null;
}

/** Oitava padrão pela clave da pauta da voz. */
export function defaultOctave(score: Score, c: Cursor): number {
  const v = voiceAt(score, c);
  const clef = score.parts[c.part]?.staves[v?.staff ?? 0]?.clef;
  return clef === "bass" || clef === "treble8vb" ? 3 : 4;
}

// ------------------------------------------------------------------- clique na pauta

const STEP_ORDER: Step[] = ["C", "D", "E", "F", "G", "A", "B"];
/** Nota na linha de cima da pauta (altura real: a clave 8vb soa uma oitava abaixo do escrito). */
const TOP_LINE: Record<Clef, { step: Step; octave: number }> = {
  treble: { step: "F", octave: 5 },
  treble8vb: { step: "F", octave: 4 },
  bass: { step: "A", octave: 3 },
  alto: { step: "G", octave: 4 },
  tenor: { step: "E", octave: 4 },
  percussion: { step: "F", octave: 5 },
};

/** Meias-distâncias entre linhas a partir da linha de cima (0 = linha de cima, 1 = 1º espaço…), até 6 linhas suplementares. */
export function snapHalfSpaces(y: number, topLineY: number, lineSpacing: number): number {
  const h = Math.round((y - topLineY) / (lineSpacing / 2));
  return Math.max(-12, Math.min(20, h));
}

/**
 * Altura de uma posição da pauta: `halfSpaces` abaixo da linha de cima na clave `clef`.
 * Acidente: o escolhido (−1/0/+1) ou, sem escolha, o da armadura.
 */
export function staffPitch(clef: Clef, halfSpaces: number, key: KeySignature, accidental: number | null = null): Pitch {
  const top = TOP_LINE[clef];
  const index = top.octave * 7 + STEP_ORDER.indexOf(top.step) - halfSpaces;
  const step = STEP_ORDER[((index % 7) + 7) % 7]!;
  const octave = Math.floor(index / 7);
  return { step, alter: accidental ?? keyAlter(step, key.fifths), octave };
}
