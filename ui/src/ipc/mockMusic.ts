// Núcleo simulado das Fases 6–8 (partituras, letra, afinação, afinador, time-stretch).
// Suficiente para desenvolvimento no navegador e E2E: regras simplificadas, sem áudio de verdade.
import type {
  DetectedNote,
  KeySignature,
  LyricLine,
  LyricUnit,
  NoteDuration,
  NoteEdit,
  NoteValue,
  Pitch,
  PitchCorrectionMap,
  Score,
  ScoreEdit,
  ScoreEvent,
  ScoreMeasure,
  ScoreNote,
  ScorePart,
  ScoreReport,
  ScoreTemplate,
  SessionView,
  VoiceEdit,
  VoiceLoc,
} from "./types";

const PPQ = 960;

/** Dados que o núcleo simulado guarda junto da sessão (entram nos snapshots de desfazer). */
interface MusicStore {
  scores: { id: string; score: Score }[];
  maps: Record<string, PitchCorrectionMap>;
}
type MusicSession = SessionView & { _music?: MusicStore };

export interface MockCtx {
  s: () => SessionView;
  mutate: (label: string, gesture: number | null, fn: (v: SessionView) => void) => SessionView;
  view: () => SessionView;
  fail: (code: string, message: string) => never;
  sr: () => number;
  uuid: () => string;
}

export function emptyMusicView(uuid: () => string): Pick<SessionView, "lyrics" | "scores" | "tuning"> {
  return { lyrics: { id: uuid(), lines: [] }, scores: [], tuning: { a4Hz: 440, maps: [] } };
}

// ----------------------------------------------------------------------------- notação (modelo)

const VALUE_FRAC: Record<NoteValue, number> = { whole: 1, half: 1 / 2, quarter: 1 / 4, eighth: 1 / 8, sixteenth: 1 / 16, thirtySecond: 1 / 32, sixtyFourth: 1 / 64, oneTwentyEighth: 1 / 128 };
const STEP_PC: Record<Pitch["step"], number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const SHARP_SPELL: [Pitch["step"], number][] = [["C", 0], ["C", 1], ["D", 0], ["D", 1], ["E", 0], ["F", 0], ["F", 1], ["G", 0], ["G", 1], ["A", 0], ["A", 1], ["B", 0]];
const FLAT_SPELL: [Pitch["step"], number][] = [["C", 0], ["D", -1], ["D", 0], ["E", -1], ["E", 0], ["F", 0], ["G", -1], ["G", 0], ["A", -1], ["A", 0], ["B", -1], ["B", 0]];

const durFrac = (d: NoteDuration) => {
  let f = VALUE_FRAC[d.value];
  let add = f;
  for (let i = 0; i < d.dots; i++) {
    add /= 2;
    f += add;
  }
  return d.tuplet ? (f * d.tuplet.normal) / d.tuplet.actual : f;
};
const midiOf = (p: Pitch) => (p.octave + 1) * 12 + STEP_PC[p.step] + p.alter;
const spell = (midi: number, fifths: number): Pitch => {
  const [step, alter] = (fifths < 0 ? FLAT_SPELL : SHARP_SPELL)[((midi % 12) + 12) % 12]!;
  return { step, alter, octave: Math.floor((midi - STEP_PC[step] - alter) / 12) - 1 };
};
const diatonic = (k: KeySignature) => {
  const tonic = (((k.fifths * 7) % 12) + 12 + (k.mode === "minor" ? 9 : 0)) % 12;
  return (k.mode === "minor" ? [0, 2, 3, 5, 7, 8, 10] : [0, 2, 4, 5, 7, 9, 11]).map((i) => (tonic + i) % 12);
};

function restsFor(capacity: number): NoteDuration[] {
  const out: NoteDuration[] = [];
  let left = capacity;
  for (const v of Object.keys(VALUE_FRAC) as NoteValue[]) {
    while (left + 1e-9 >= VALUE_FRAC[v]) {
      out.push({ value: v, dots: 0, tuplet: null });
      left -= VALUE_FRAC[v];
    }
  }
  return out;
}

function meterAt(score: Score, measure: number): { numerator: number; denominator: number } {
  const m0 = score.tempo.meters[0] ?? { numerator: 4, denominator: 4 };
  let cur = { numerator: m0.numerator, denominator: m0.denominator };
  const ms = score.parts[0]?.measures ?? [];
  for (let i = 0; i <= measure && i < ms.length; i++) if (ms[i]!.meter) cur = ms[i]!.meter!;
  return cur;
}
const capacityOf = (score: Score, measure: number) => {
  const m = meterAt(score, measure);
  return m.numerator / m.denominator;
};

function newEvent(score: Score, duration: NoteDuration, kind: ScoreEvent["kind"]): ScoreEvent {
  return { id: score.nextEventId++, duration, kind, articulations: [], dynamic: null, grace: null, lyric: null };
}

function emptyMeasure(score: Score, key: KeySignature, staves: number, capacity: number): ScoreMeasure {
  return {
    key,
    meter: null,
    repeatStart: false,
    repeatEnd: null,
    voices: Array.from({ length: staves }, (_, staff) => ({ staff, events: restsFor(capacity).map((d) => newEvent(score, d, { type: "rest" })) })),
  };
}

const GUITAR_TAB = { tuning: [64, 59, 55, 50, 45, 40], capo: 0, frets: 24 };
const BASS_TAB = { tuning: [43, 38, 33, 28], capo: 0, frets: 24 };

function scoreFromTemplate(t: ScoreTemplate, fail: MockCtx["fail"]): Score {
  if (!t.title.trim()) fail("notation.invalid_name", "Informe um título.");
  if (!(t.measures >= 1 && t.measures <= 1000)) fail("app.invalid_argument", "Número de compassos deve estar entre 1 e 1000.");
  if (t.instruments.length === 0) fail("app.invalid_argument", "Escolha ao menos um instrumento.");
  const score: Score = {
    title: t.title.trim(),
    composer: (t.composer ?? "").trim(),
    tempo: { tempos: [{ tick: 0, bpm: t.bpm }], meters: [{ tick: 0, numerator: t.meterNumerator, denominator: t.meterDenominator }] },
    parts: [],
    nextEventId: 1,
  };
  const key: KeySignature = { fifths: t.keyFifths ?? 0, mode: t.keyMode ?? "major" };
  const cap = t.meterNumerator / t.meterDenominator;
  for (const inst of t.instruments) {
    const defs: Record<string, [string, number, ScorePart["staves"]]> = {
      piano: ["Piano", 0, [{ clef: "treble", standard: true, tab: null }, { clef: "bass", standard: true, tab: null }]],
      lead: ["Melodia", 52, [{ clef: "treble", standard: true, tab: null }]],
      guitar: ["Violão", 24, [{ clef: "treble8vb", standard: true, tab: GUITAR_TAB }]],
      bass: ["Baixo", 33, [{ clef: "bass", standard: true, tab: BASS_TAB }]],
      drums: ["Bateria", 0, [{ clef: "percussion", standard: true, tab: null }]],
    };
    const [name, program, staves] = defs[inst.kind]!;
    score.parts.push({
      name: inst.name?.trim() || name,
      program,
      percussion: inst.kind === "drums",
      staves,
      measures: Array.from({ length: t.measures }, () => emptyMeasure(score, key, staves.length, cap)),
      spanners: [],
    });
  }
  return score;
}

function tabFor(midi: number, tab: { tuning: number[]; capo: number; frets: number }) {
  // corda mais aguda possível com traste válido (1 = mais aguda)
  for (let i = 0; i < tab.tuning.length; i++) {
    const fret = midi - tab.tuning[i]! - tab.capo;
    if (fret >= 0 && fret <= tab.frets) return { string: i + 1, fret };
  }
  return null;
}

function editVoice(score: Score, loc: VoiceLoc, e: VoiceEdit, fail: MockCtx["fail"]) {
  const part = score.parts[loc.part] ?? fail("notation.part_not_found", "parte não encontrada");
  const m = part.measures[loc.measure] ?? fail("notation.measure_not_found", "compasso não encontrado");
  if (loc.voice === m.voices.length && e.kind === "insert") m.voices.push({ staff: 0, events: [] });
  const v = m.voices[loc.voice] ?? fail("notation.voice_not_found", "voz não encontrada");
  const ev = (i: number) => v.events[i] ?? fail("notation.event_not_found", `evento ${i} não encontrado`);
  const note = (i: number, n: number) => {
    const x = ev(i);
    if (x.kind.type !== "chord") fail("notation.not_a_chord", "o evento é uma pausa");
    return x.kind.notes[n] ?? fail("notation.note_not_found", "nota não encontrada");
  };
  const tab = part.staves[v.staff]?.tab ?? null;
  const withTab = (p: Pitch): ScoreNote => ({ pitch: p, tie: false, tab: tab ? tabFor(midiOf(p), tab) : null, techniques: [] });
  switch (e.kind) {
    case "insert":
      if (e.index > v.events.length) fail("notation.event_not_found", "posição inválida");
      v.events.splice(e.index, 0, normalizeEvent(score, e.event, withTab));
      break;
    case "replace": {
      const id = ev(e.index).id;
      v.events[e.index] = { ...normalizeEvent(score, e.event, withTab), id };
      break;
    }
    case "delete":
      ev(e.index);
      v.events.splice(e.index, 1);
      break;
    case "setDuration":
      ev(e.index).duration = e.duration;
      break;
    case "setPitch": {
      const n = note(e.index, e.note);
      n.pitch = e.pitch;
      if (tab) n.tab = tabFor(midiOf(e.pitch), tab);
      break;
    }
    case "addNote": {
      const x = ev(e.index);
      if (x.kind.type === "rest") x.kind = { type: "chord", notes: [] };
      if (x.kind.type === "chord") x.kind.notes.push(e.note);
      break;
    }
    case "removeNote": {
      const x = ev(e.index);
      note(e.index, e.note);
      if (x.kind.type === "chord") x.kind.notes.splice(e.note, 1);
      if (x.kind.type === "chord" && x.kind.notes.length === 0) x.kind = { type: "rest" };
      break;
    }
    case "addArticulation": {
      const x = ev(e.index);
      if (!x.articulations.includes(e.articulation)) x.articulations.push(e.articulation);
      break;
    }
    case "removeArticulation": {
      const x = ev(e.index);
      x.articulations = x.articulations.filter((a) => a !== e.articulation);
      break;
    }
    case "setDynamic":
      ev(e.index).dynamic = e.dynamic;
      break;
    case "setTie":
      note(e.index, e.note).tie = e.tie;
      break;
    case "setTab": {
      const n = note(e.index, e.note);
      if (e.tab && tab) {
        const midi = tab.tuning[e.tab.string - 1];
        if (midi === undefined || e.tab.fret > tab.frets) fail("notation.invalid_tab", "corda ou traste inválido");
        n.tab = e.tab;
        n.pitch = spell(midi + tab.capo + e.tab.fret, m.key.fifths);
      } else n.tab = e.tab;
      break;
    }
    case "addTechnique":
      note(e.index, e.note).techniques.push(e.technique);
      break;
    case "removeTechnique": {
      const n = note(e.index, e.note);
      n.techniques = n.techniques.filter((t) => t.kind !== e.technique.kind);
      break;
    }
    case "setLyric":
      ev(e.index).lyric = e.lyric;
      break;
    case "setGrace":
      ev(e.index).grace = e.grace;
      break;
    case "fillRests": {
      const filled = v.events.reduce((a, x) => a + (x.grace ? 0 : durFrac(x.duration)), 0);
      const left = capacityOf(score, loc.measure) - filled;
      if (left > 1e-9) for (const d of restsFor(left)) v.events.push(newEvent(score, d, { type: "rest" }));
      break;
    }
  }
}

function normalizeEvent(score: Score, e: ScoreEvent, withTab: (p: Pitch) => ScoreNote): ScoreEvent {
  const kind: ScoreEvent["kind"] = e.kind.type === "chord" ? { type: "chord", notes: e.kind.notes.map((n) => (n.tab ? n : { ...withTab(n.pitch), tie: n.tie, techniques: n.techniques })) } : { type: "rest" };
  return { ...e, id: score.nextEventId++, kind };
}

function forEachNote(score: Score, fn: (n: { pitch: Pitch; tab: { string: number; fret: number } | null }, key: KeySignature, part: ScorePart) => void, partIdx?: number) {
  score.parts.forEach((p, pi) => {
    if (partIdx !== undefined && pi !== partIdx) return;
    for (const m of p.measures) for (const v of m.voices) for (const e of v.events) if (e.kind.type === "chord") for (const n of e.kind.notes) fn(n, m.key, p);
  });
}

function editScore(score: Score, e: ScoreEdit, fail: MockCtx["fail"]) {
  switch (e.kind) {
    case "addMeasure":
      for (const p of score.parts) {
        const prev = p.measures[Math.max(0, e.index - 1)]!;
        p.measures.splice(e.index, 0, emptyMeasure(score, prev.key, p.staves.length, capacityOf(score, Math.max(0, e.index - 1))));
      }
      break;
    case "removeMeasure":
      if ((score.parts[0]?.measures.length ?? 0) <= 1) fail("notation.last_measure", "A partitura precisa de ao menos um compasso.");
      for (const p of score.parts) p.measures.splice(e.index, 1);
      break;
    case "setKey":
      for (const p of score.parts) {
        const old = p.measures[e.measure]?.key;
        for (let i = e.measure; i < p.measures.length; i++) {
          if (i > e.measure && JSON.stringify(p.measures[i]!.key) !== JSON.stringify(old)) break;
          p.measures[i]!.key = e.key;
        }
      }
      break;
    case "setMeter":
      for (const p of score.parts) {
        const m = p.measures[e.measure] ?? fail("notation.measure_not_found", "compasso não encontrado");
        m.meter = e.meter;
      }
      if (e.measure === 0 && e.meter) score.tempo.meters[0] = { tick: 0, ...e.meter };
      break;
    case "setRepeat":
      for (const p of score.parts) Object.assign(p.measures[e.measure]!, { repeatStart: e.start, repeatEnd: e.end });
      break;
    case "transpose": {
      const part = e.scope.scope === "part" ? e.scope.target : undefined;
      forEachNote(
        score,
        (n, key, p) => {
          const midi = midiOf(n.pitch) + e.semitones;
          if (midi < 0 || midi > 127) fail("notation.pitch_out_of_range", "nota fora da faixa MIDI");
          n.pitch = spell(midi, key.fifths);
          const tab = p.staves[0]?.tab;
          if (n.tab && tab) n.tab = tabFor(midi, tab);
        },
        part,
      );
      break;
    }
    case "setHeader":
      score.title = e.title;
      score.composer = e.composer;
      break;
    case "setTempo":
      score.tempo.tempos = [...score.tempo.tempos.filter((t) => t.tick !== e.tick), { tick: e.tick, bpm: e.bpm }].sort((a, b) => a.tick - b.tick);
      break;
    case "addPart":
      score.parts.splice(e.index, 0, e.part);
      break;
    case "removePart":
      score.parts.splice(e.index, 1);
      break;
    case "addSpanner":
      score.parts[e.part]?.spanners.push(e.spanner);
      break;
    case "removeSpanner":
      score.parts[e.part]?.spanners.splice(e.index, 1);
      break;
  }
}

function validate(score: Score, scale: number[] | null): ScoreReport {
  const r: ScoreReport = { measureIssues: [], outOfScale: [] };
  const gcd = (a: number, b: number): number => (b ? gcd(b, a % b) : a);
  const frac = (x: number) => {
    const den = 128 * 3;
    const num = Math.round(x * den);
    const g = gcd(Math.abs(num), den) || 1;
    return { num: num / g, den: den / g };
  };
  score.parts.forEach((p, part) =>
    p.measures.forEach((m, measure) => {
      const cap = capacityOf(score, measure);
      m.voices.forEach((v, voice) => {
        if (voice > 0 && v.events.length === 0) return;
        const filled = v.events.reduce((a, x) => a + (x.grace ? 0 : durFrac(x.duration)), 0);
        if (filled < cap - 1e-9) r.measureIssues.push({ loc: { part, measure, voice }, issue: { kind: "underfilled", missing: frac(cap - filled) } });
        else if (filled > cap + 1e-9) r.measureIssues.push({ loc: { part, measure, voice }, issue: { kind: "overfilled", excess: frac(filled - cap) } });
        const allowed = scale ?? diatonic(m.key);
        v.events.forEach((e, index) => {
          if (e.kind.type !== "chord" || p.percussion) return;
          e.kind.notes.forEach((n, noteIndex) => {
            if (!allowed.includes(midiOf(n.pitch) % 12)) r.outOfScale.push({ event: { voice: { part, measure, voice }, index }, noteIndex, pitch: n.pitch });
          });
        });
      });
    }),
  );
  return r;
}

const TYPE_XML: Record<NoteValue, string> = { whole: "whole", half: "half", quarter: "quarter", eighth: "eighth", sixteenth: "16th", thirtySecond: "32nd", sixtyFourth: "64th", oneTwentyEighth: "128th" };
const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** MusicXML 4.0 partwise mínimo (o núcleo real usa `infra-notation-io`). */
export function scoreToMusicXml(score: Score): string {
  const div = 480;
  const parts = score.parts.map((p, i) => `<score-part id="P${i + 1}"><part-name>${esc(p.name)}</part-name></score-part>`).join("");
  const body = score.parts
    .map((p, pi) => {
      const measures = p.measures
        .map((m, mi) => {
          const meter = meterAt(score, mi);
          const attrs =
            mi === 0
              ? `<attributes><divisions>${div}</divisions><key><fifths>${m.key.fifths}</fifths><mode>${m.key.mode}</mode></key><time><beats>${meter.numerator}</beats><beat-type>${meter.denominator}</beat-type></time><staves>${p.staves.length}</staves>${p.staves
                  .map((s, si) => {
                    const clef = { treble: ["G", 2], treble8vb: ["G", 2], bass: ["F", 4], alto: ["C", 3], tenor: ["C", 4], percussion: ["percussion", 2] }[s.clef] as [string, number];
                    const tuning = s.tab ? [...s.tab.tuning].reverse().map((m, li) => { const p = spell(m, 0); return `<staff-tuning line="${li + 1}"><tuning-step>${p.step}</tuning-step>${p.alter ? `<tuning-alter>${p.alter}</tuning-alter>` : ""}<tuning-octave>${p.octave}</tuning-octave></staff-tuning>`; }).join("") : "";
                    const tab = s.tab ? `<staff-details number="${si + 1}"><staff-lines>${s.tab.tuning.length}</staff-lines>${tuning}</staff-details>` : "";
                    return `<clef number="${si + 1}"><sign>${clef[0]}</sign><line>${clef[1]}</line>${s.clef === "treble8vb" ? "<clef-octave-change>-1</clef-octave-change>" : ""}</clef>${tab}`;
                  })
                  .join("")}</attributes>`
              : m.meter
                ? `<attributes><time><beats>${m.meter.numerator}</beats><beat-type>${m.meter.denominator}</beat-type></time></attributes>`
                : "";
          let pos = 0;
          const voices = m.voices
            .map((v, vi) => {
              const back = vi > 0 && pos > 0 ? `<backup><duration>${pos}</duration></backup>` : "";
              pos = 0;
              const notes = v.events
                .map((e) => {
                  const dur = Math.round(durFrac(e.duration) * 4 * div);
                  pos += dur;
                  const common = `<duration>${dur}</duration><voice>${vi + 1}</voice><type>${TYPE_XML[e.duration.value]}</type>${"<dot/>".repeat(e.duration.dots)}<staff>${v.staff + 1}</staff>`;
                  const dyn = e.dynamic ? `<direction placement="below"><direction-type><dynamics><${e.dynamic}/></dynamics></direction-type><staff>${v.staff + 1}</staff></direction>` : "";
                  if (e.kind.type === "rest") return `${dyn}<note><rest/>${common}</note>`;
                  return (
                    dyn +
                    e.kind.notes
                      .map((n, ni) => {
                        const arts = ni === 0 && e.articulations.length ? `<articulations>${e.articulations.map((a) => (a === "marcato" ? "<strong-accent/>" : a === "fermata" ? "" : `<${a}/>`)).join("")}</articulations>` : "";
                        const tech = n.tab ? `<technical><string>${n.tab.string}</string><fret>${n.tab.fret}</fret></technical>` : "";
                        const tie = n.tie ? `<tie type="start"/>` : "";
                        const notations = arts || tech || n.tie ? `<notations>${n.tie ? `<tied type="start"/>` : ""}${arts}${tech}</notations>` : "";
                        return `<note>${ni > 0 ? "<chord/>" : ""}<pitch><step>${n.pitch.step}</step>${n.pitch.alter ? `<alter>${n.pitch.alter}</alter>` : ""}<octave>${n.pitch.octave}</octave></pitch>${tie}${common}${notations}</note>`;
                      })
                      .join("")
                  );
                })
                .join("");
              return back + notes;
            })
            .join("");
          return `<measure number="${mi + 1}">${attrs}${voices}</measure>`;
        })
        .join("");
      return `<part id="P${pi + 1}">${measures}</part>`;
    })
    .join("");
  return `<?xml version="1.0" encoding="UTF-8"?><score-partwise version="4.0"><work><work-title>${esc(score.title)}</work-title></work><identification><creator type="composer">${esc(score.composer)}</creator></identification><part-list>${parts}</part-list>${body}</score-partwise>`;
}

// ------------------------------------------------------------------------------------ letra

const VOWELS = "aeiouáàâãéêíóôõúüy";
/** Separação silábica aproximada (CV); o núcleo real usa as regras do `ctx-sync`. */
export function mockSyllables(word: string): string[] {
  if (word.includes("-")) {
    const parts = word.split("-").filter(Boolean);
    return parts.map((p, i) => (i < parts.length - 1 ? `${p}-` : p));
  }
  const isV = (c: string | undefined) => !!c && VOWELS.includes(c.toLowerCase());
  const out: string[] = [];
  let cur = "";
  for (let i = 0; i < word.length; i++) {
    const c = word[i]!;
    cur += c;
    if (isV(c) && !isV(word[i + 1])) {
      const next = word[i + 1];
      const after = word[i + 2];
      const rest = word.slice(i + 1);
      if (!rest.split("").some((x) => isV(x))) {
        cur += rest;
        i = word.length;
      } else if (next && !isV(next) && after && !isV(after) && !"lrh".includes(after.toLowerCase())) {
        cur += next;
        i++;
      }
      out.push(cur);
      cur = "";
    }
  }
  if (cur) {
    if (out.length) out[out.length - 1] += cur;
    else out.push(cur);
  }
  return out;
}

function unitSyllables(lines: LyricLine[], unit: LyricUnit): [number, number, number][] {
  const out: [number, number, number][] = [];
  lines.forEach((l, li) =>
    l.words.forEach((w, wi) =>
      w.syllables.forEach((_, si) => {
        if (unit.kind === "line" && unit.line === li) out.push([li, wi, si]);
        if (unit.kind === "word" && unit.line === li && unit.word === wi) out.push([li, wi, si]);
        if (unit.kind === "syllable" && unit.line === li && unit.word === wi && unit.syllable === si) out.push([li, wi, si]);
      }),
    ),
  );
  return out;
}

function setSpan(lines: LyricLine[], unit: LyricUnit, start: number, end: number, fail: MockCtx["fail"]) {
  const syl = unitSyllables(lines, unit);
  if (syl.length === 0) fail("sync.unit_not_found", "Unidade de letra não encontrada.");
  const weights = syl.map(([l, w, s]) => Math.max(1, lines[l]!.words[w]!.syllables[s]!.text.length));
  const total = weights.reduce((a, b) => a + b, 0);
  let t = start;
  syl.forEach(([l, w, s], i) => {
    const e = i === syl.length - 1 ? end : Math.max(t + 1, Math.round(t + ((end - start) * weights[i]!) / total));
    lines[l]!.words[w]!.syllables[s]!.span = { start: t, end: e };
    t = e;
  });
}

function unitsAt(lines: LyricLine[], g: "syllable" | "word" | "line"): LyricUnit[] {
  const out: LyricUnit[] = [];
  lines.forEach((l, line) => {
    if (g === "line") return void out.push({ kind: "line", line });
    l.words.forEach((w, word) => {
      if (g === "word") return void out.push({ kind: "word", line, word });
      w.syllables.forEach((_, syllable) => out.push({ kind: "syllable", line, word, syllable }));
    });
  });
  return out;
}

// --------------------------------------------------------------------------------- afinação

const NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

function wavBytes(seconds: number, sr: number, hz: number): ArrayBuffer {
  const n = Math.round(seconds * sr);
  const buf = new ArrayBuffer(44 + n * 2);
  const d = new DataView(buf);
  const str = (o: number, s: string) => [...s].forEach((c, i) => d.setUint8(o + i, c.charCodeAt(0)));
  str(0, "RIFF");
  d.setUint32(4, 36 + n * 2, true);
  str(8, "WAVE");
  str(12, "fmt ");
  d.setUint32(16, 16, true);
  d.setUint16(20, 1, true);
  d.setUint16(22, 1, true);
  d.setUint32(24, sr, true);
  d.setUint32(28, sr * 2, true);
  d.setUint16(32, 2, true);
  d.setUint16(34, 16, true);
  str(36, "data");
  d.setUint32(40, n * 2, true);
  for (let i = 0; i < n; i++) d.setInt16(44 + i * 2, Math.round(0.2 * 32767 * Math.sin((2 * Math.PI * hz * i) / sr) * Math.min(1, (n - i) / 2000)), true);
  return buf;
}

// ------------------------------------------------------------------------------- comandos

export function musicHandlers(ctx: MockCtx): { handlers: Record<string, (a: Record<string, any>) => unknown>; binary: (cmd: string, a: Record<string, unknown>) => ArrayBuffer | null } {
  const { s, mutate, view, fail, sr, uuid } = ctx;
  const store = (v: SessionView = s()): MusicStore => ((v as MusicSession)._music ??= { scores: [], maps: {} });
  const sync = (v: SessionView) => {
    const m = store(v);
    v.scores = m.scores.map(({ id, score }) => ({ id, title: score.title, composer: score.composer, parts: score.parts.map((p) => p.name), measures: score.parts[0]?.measures.length ?? 0 }));
    v.tuning.maps = Object.values(m.maps).map((x) => ({ mapId: x.id, sourceId: x.source, noteCount: x.notes.length, editCount: Object.keys(x.edits).length, scale: x.scale }));
  };
  const scoreOf = (v: SessionView, id: string) => store(v).scores.find((x) => x.id === id)?.score ?? fail("notation.score_not_found", "Partitura não encontrada.");
  const addScore = (label: string, score: Score) => {
    const id = uuid();
    const view = mutate(label, null, (v) => {
      store(v).scores.push({ id, score });
      sync(v);
    });
    return { scoreId: id, view };
  };
  const lyricsOf = (v: SessionView) => v.lyrics.lines;
  let tuner: { channel: number; started: number } | null = null;

  const handlers: Record<string, (a: Record<string, any>) => unknown> = {
    score_list: () => view().scores,
    score_create: (a) => addScore("Nova partitura", scoreFromTemplate(a.req as ScoreTemplate, fail)),
    score_create_from_chords: (a) => {
      const sc = scoreFromTemplate({ title: a.req.title, bpm: s().tempo.tempos[0]?.bpm ?? 120, meterNumerator: a.req.meterNumerator, meterDenominator: a.req.meterDenominator, keyFifths: a.req.keyFifths, keyMode: a.req.keyMode, measures: Math.max(1, a.req.slots.length), instruments: [{ kind: "piano" }] }, fail);
      return addScore("Partitura dos acordes", sc);
    },
    score_get: (a) => structuredClone(scoreOf(s(), a.scoreId)),
    score_musicxml: (a) => scoreToMusicXml(scoreOf(s(), a.scoreId)),
    score_delete: (a) =>
      mutate("Remover partitura", null, (v) => {
        scoreOf(v, a.scoreId);
        store(v).scores = store(v).scores.filter((x) => x.id !== a.scoreId);
        sync(v);
      }),
    score_edit_voice: (a) =>
      mutate("Editar partitura", a.req.gesture, (v) => {
        const sc = scoreOf(v, a.req.scoreId);
        const backup = structuredClone(sc);
        try {
          editVoice(sc, a.req.loc, a.req.edit, fail);
        } catch (e) {
          Object.assign(sc, backup);
          throw e;
        }
        sync(v);
      }),
    score_edit: (a) =>
      mutate("Editar partitura", null, (v) => {
        const sc = scoreOf(v, a.req.scoreId);
        const backup = structuredClone(sc);
        try {
          editScore(sc, a.req.edit, fail);
        } catch (e) {
          Object.assign(sc, backup);
          throw e;
        }
        sync(v);
      }),
    score_import: (a) => addScore("Importar partitura", scoreFromTemplate({ title: String(a.path).split(/[\\/]/).pop()!.replace(/\.[^.]+$/, ""), bpm: 120, meterNumerator: 4, meterDenominator: 4, measures: 4, instruments: [{ kind: "lead" }] }, fail)),
    score_import_bytes: (a) => {
      const bytes = new Uint8Array(a.req.bytes as number[]);
      const head = new TextDecoder().decode(bytes.slice(0, 4096));
      const known = head.startsWith("MThd") || head.startsWith("PK") || head.includes("<score-partwise") || head.includes("<GPIF");
      if (!known) fail("notation_io.unsupported", "formato não suportado: formato desconhecido");
      const r = addScore("Importar partitura", scoreFromTemplate({ title: String(a.req.name).replace(/\.[^.]+$/, ""), bpm: 120, meterNumerator: 4, meterDenominator: 4, measures: 4, instruments: [{ kind: head.startsWith("PK") ? "guitar" : "lead" }] }, fail));
      return { ...r, warnings: ["Importação simulada: conteúdo não interpretado no modo de demonstração."] };
    },
    score_export: (a) => {
      scoreOf(s(), a.req.scoreId);
      return null;
    },
    score_render_to_track: (a) =>
      mutate("Renderizar partitura", null, (v) => {
        const sc = scoreOf(v, a.req.scoreId);
        const len = sr() * 4;
        const srcId = uuid();
        v.sources.push({ id: srcId, contentHash: "b".repeat(64), relPath: `audio/rendered/${srcId}.wav`, originalName: `${sc.title}.wav`, channels: 2, lengthFrames: len });
        const clip = { id: uuid(), sourceId: srcId, sourceName: `${sc.title}.wav`, start: a.req.atTick ?? 0, startFrame: 0, offsetFrames: 0, lengthFrames: len, endTick: 0, gain: 0 };
        const existing = a.req.trackId ? v.tracks.find((t) => t.id === a.req.trackId) : null;
        if (existing) existing.clips.push(clip);
        else
          v.tracks.push({
            id: uuid(),
            name: sc.title.slice(0, 64),
            clips: [clip],
            strip: { volume: 0, pan: 0, mute: false, solo: false, inserts: [], sends: [], output: { kind: "master" } },
            recording: { armed: false, input: null, monitor: "off", takes: [], activeTake: null },
            audible: true,
            meterSlot: 0,
          });
      }),
    score_validate: (a) => validate(scoreOf(s(), a.req.scoreId), a.req.scale ?? null),
    // letra
    lyrics_set_text: (a) =>
      mutate("Importar letra", null, (v) => {
        const text = String(a.text);
        v.lyrics.lines = text.trim()
          ? text
              .split(/\r?\n/)
              .filter((l) => l.trim())
              .map((l) => ({ words: l.trim().split(/\s+/).map((w) => ({ syllables: mockSyllables(w).map((t) => ({ text: t, span: null })) })) }))
          : [];
      }),
    lyrics_set_span: (a) => {
      if (!(a.req.start >= 0 && a.req.start < a.req.end)) fail("api.invalid_argument", "intervalo inválido (início ≥ 0 e menor que o fim)");
      return mutate("Tempo da letra", a.req.gesture, (v) => setSpan(lyricsOf(v), a.req.unit, a.req.start, a.req.end, fail));
    },
    lyrics_clear_span: (a) =>
      mutate("Limpar tempo da letra", null, (v) => {
        for (const [l, w, sy] of unitSyllables(lyricsOf(v), a.unit)) lyricsOf(v)[l]!.words[w]!.syllables[sy]!.span = null;
      }),
    lyrics_shift: (a) =>
      mutate("Deslocar letra", a.req.gesture, (v) => {
        const key = (r: { line: number; word: number; syllable: number }) => r.line * 1e6 + r.word * 1e3 + r.syllable;
        const lo = key(a.req.from);
        const hi = key(a.req.to);
        lyricsOf(v).forEach((l, li) =>
          l.words.forEach((w, wi) =>
            w.syllables.forEach((sy, si) => {
              const k = key({ line: li, word: wi, syllable: si });
              if (sy.span && k >= lo && k <= hi) {
                if (sy.span.start + a.req.delta < 0) fail("sync.negative_time", "O deslocamento levaria a letra para antes do início.");
                sy.span = { start: sy.span.start + a.req.delta, end: sy.span.end + a.req.delta };
              }
            }),
          ),
        );
      }),
    lyrics_apply_tap_sync: (a) => {
      const taps = a.req.taps as number[];
      const units = unitsAt(lyricsOf(s()), a.req.granularity).slice(a.req.from ?? 0);
      if (taps.length === 0) fail("sync.no_taps", "Nenhum toque registrado.");
      if (taps.length > units.length) fail("sync.too_many_taps", "Mais toques do que unidades de letra.");
      for (let i = 1; i < taps.length; i++) if (taps[i]! <= taps[i - 1]!) fail("sync.tap_out_of_order", "Toques fora de ordem.");
      if (a.req.end <= taps[taps.length - 1]!) fail("sync.tap_out_of_order", "O fim precisa vir depois do último toque.");
      return mutate("Sincronizar por toque", null, (v) => taps.forEach((t, i) => setSpan(lyricsOf(v), units[i]!, t, taps[i + 1] ?? a.req.end, fail)));
    },
    lyrics_export: () => {
      if (!s().lyrics.lines.some((l) => l.words.some((w) => w.syllables.some((x) => x.span)))) fail("app.nothing_to_export", "A letra não tem trechos sincronizados para exportar.");
      return null;
    },
    lyrics_import: () =>
      mutate("Importar letra", null, (v) => {
        v.lyrics.lines = [{ words: ["Le-", "tra"].map((t, i) => ({ syllables: [{ text: t, span: { start: i * PPQ, end: (i + 1) * PPQ } }] })) }];
      }),
    // afinação
    tuning_analyze: (a) =>
      mutate("Analisar afinação", null, (v) => {
        const src = v.sources.find((x) => x.id === a.sourceId) ?? fail("project.source_not_found", "fonte de áudio não encontrada");
        const n = 6;
        const len = Math.floor(src.lengthFrames / n);
        const midis = [60.32, 62.1, 63.78, 65.2, 67.41, 64.88];
        const notes: DetectedNote[] = midis.map((midi, i) => ({ id: uuid(), startFrame: i * len + Math.round(len * 0.05), endFrame: (i + 1) * len - Math.round(len * 0.1), midi }));
        const prev = store(v).maps[src.id];
        store(v).maps[src.id] = { id: prev?.id ?? uuid(), source: src.id, notes, edits: {}, scale: prev?.scale ?? { enabled: false, key: 0, allowed: [0, 2, 4, 5, 7, 9, 11], retuneSpeedMs: 50, humanize: 0 } };
        sync(v);
      }),
    tuning_get: (a) => structuredClone(store().maps[a.sourceId] ?? fail("tuning.map_not_found", "A fonte ainda não foi analisada.")),
    tuning_edit_note: (a) =>
      mutate("Editar nota (afinação)", a.req.gesture, (v) => {
        const m = store(v).maps[a.req.sourceId] ?? fail("tuning.map_not_found", "A fonte ainda não foi analisada.");
        if (!m.notes.some((n) => n.id === a.req.noteId)) fail("tuning.note_not_found", "Nota não encontrada.");
        const e = a.req.edit as NoteEdit;
        if (Math.abs(e.centsOffset) > 1200) fail("tuning.invalid_cents_offset", "Ajuste em cents fora de ±1200.");
        const neutral = e.target === null && e.centsOffset === 0 && e.flatten === 0 && e.formantSemitones === 0;
        if (neutral) delete m.edits[a.req.noteId];
        else m.edits[a.req.noteId] = e;
        sync(v);
      }),
    tuning_set_scale: (a) =>
      mutate("Escala da correção", null, (v) => {
        const m = store(v).maps[a.req.sourceId] ?? fail("tuning.map_not_found", "A fonte ainda não foi analisada.");
        if (!a.req.scale.allowed.includes(a.req.scale.key)) fail("tuning.invalid_scale", "A escala precisa conter a tônica.");
        m.scale = a.req.scale;
        sync(v);
      }),
    tuning_set_reference: (a) => {
      if (!(a.a4Hz >= 415 && a.a4Hz <= 466)) fail("tuning.invalid_reference", "Lá4 deve estar entre 415 e 466 Hz.");
      return mutate("Referência do Lá4", null, (v) => (v.tuning.a4Hz = a.a4Hz));
    },
    tuning_apply: (a) =>
      mutate("Aplicar correção de afinação", null, (v) => {
        const m = store(v).maps[a.req.sourceId] ?? fail("tuning.map_not_found", "A fonte ainda não foi analisada.");
        if (Object.keys(m.edits).length === 0 && !m.scale.enabled) fail("tuning.nothing_to_apply", "Nenhuma correção para aplicar.");
        const clip = v.tracks.flatMap((t) => t.clips).find((c) => c.id === a.req.clipId) ?? fail("project.clip_not_found", "clipe não encontrado");
        const src = v.sources.find((x) => x.id === a.req.sourceId)!;
        const id = uuid();
        const name = src.originalName.replace(/(\.[^.]+)?$/, " (afinado)$1");
        v.sources.push({ ...src, id, relPath: `audio/rendered/${id}.wav`, originalName: name });
        Object.assign(clip, { id: uuid(), sourceId: id, sourceName: name });
      }),
    tuning_clear: (a) =>
      mutate("Limpar afinação", null, (v) => {
        delete store(v).maps[a.sourceId];
        sync(v);
      }),
    tuner_start: (a) => {
      if (!(a.inputChannel >= 0 && a.inputChannel < 2)) fail("app.input_unavailable", "Canal de entrada indisponível.");
      tuner = { channel: a.inputChannel, started: Date.now() };
      return null;
    },
    tuner_read: () => {
      const on = tuner ?? fail("app.tuner_off", "O afinador está desligado.");
      const t = (Date.now() - on.started) / 1000;
      const cents = Math.round(35 * Math.sin(t * 1.3) * 10) / 10;
      const note = 69;
      const a4 = s().tuning.a4Hz;
      return { note, name: NAMES[note % 12]!, octave: Math.floor(note / 12) - 1, cents, frequencyHz: a4 * Math.pow(2, cents / 1200), confidence: 0.95 };
    },
    tuner_stop: () => {
      tuner = null;
      return null;
    },
    clip_time_stretch: (a) => {
      const target = a.req.targetBpm ?? s().tempo.tempos[0]?.bpm ?? 120;
      const ratio = a.req.sourceBpm / target;
      if (!(a.req.sourceBpm >= 20 && a.req.sourceBpm <= 999) || !(ratio >= 0.25 && ratio <= 4)) fail("sync.invalid_ratio", "Razão de time-stretch fora de 0,25–4.");
      return mutate("Time-stretch do clipe", null, (v) => {
        const clip = v.tracks.flatMap((t) => t.clips).find((c) => c.id === a.req.clipId) ?? fail("project.clip_not_found", "clipe não encontrado");
        const src = v.sources.find((x) => x.id === clip.sourceId)!;
        const id = uuid();
        const lengthFrames = Math.round(clip.lengthFrames * ratio);
        const name = src.originalName.replace(/(\.[^.]+)?$/, ` (${Math.round(target)} BPM)$1`);
        v.sources.push({ ...src, id, relPath: `audio/rendered/${id}.wav`, originalName: name, lengthFrames });
        Object.assign(clip, { id: uuid(), sourceId: id, sourceName: name, offsetFrames: 0, lengthFrames });
      });
    },
  };

  const binary = (cmd: string, a: Record<string, unknown>): ArrayBuffer | null => {
    if (cmd !== "score_audition") return null;
    const sc = scoreOf(s(), String(a.scoreId));
    const pitches: Pitch[] = [];
    forEachNote(sc, (n) => pitches.push(n.pitch));
    const first = pitches[0] ?? fail("app.empty_score", "A partitura não tem notas para tocar.");
    return wavBytes(0.4, 22_050, 440 * Math.pow(2, (midiOf(first) - 69) / 12));
  };
  return { handlers, binary };
}
