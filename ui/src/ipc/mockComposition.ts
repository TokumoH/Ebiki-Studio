// Simulação dos comandos de composição (harmonia, bases, biblioteca, sintetizador) para o modo
// navegador/E2E. Envolve o transporte simulado do projeto: inserções viram `audio_import` no núcleo
// simulado (um único passo desfazível com `history_undo`, como no Rust). Não sintetiza música:
// a audição devolve um WAV curto e válido.
import type { Transport } from "./transport";
import type {
  BaseKey,
  ChordSuggestion,
  GenerateBaseRequest,
  HarmonicField,
  HarmonyKey,
  InsertBaseOutcome,
  InsertedClip,
  LibraryBase,
  LibraryHistory,
  MismatchReport,
  ProgressionRef,
  ProgressionSuggestion,
  ProgressionView,
  RealizeRequest,
  SessionView,
  SynthStatus,
  UserProgressionInput,
} from "./types";
import keysCatalog from "./__fixtures__/composition-keys.json";
import baseOptions from "./__fixtures__/composition-base-options.json";

const PPQ = 960;
const uuid = () => crypto.randomUUID();

function fail(code: string, message: string): never {
  throw { code, message };
}

// ------------------------------------------------------------------ teoria mínima

const MAJOR = [0, 2, 4, 5, 7, 9, 11];
const HARM = [0, 2, 3, 5, 7, 8, 11];
const MELO = [0, 2, 3, 5, 7, 9, 11];
const rotate = (s: number[], r: number) => {
  const out = [...s.slice(r), ...s.slice(0, r)];
  return out.map((x) => (x - out[0]! + 12) % 12);
};
const MODE_ORDER = keysCatalog.modes.map((m) => m.mode);
function scaleOf(mode: string): number[] {
  const i = Math.max(0, MODE_ORDER.indexOf(mode));
  const family = [MAJOR, HARM, MELO][Math.floor(i / 7)] ?? MAJOR;
  return rotate(family, i % 7);
}
const isMinorMode = (mode: string) => keysCatalog.modes.find((m) => m.mode === mode)?.isMinor ?? false;

const SHARP = keysCatalog.tonics.map((t) => t.sharp);
const FLAT = keysCatalog.tonics.map((t) => t.flat);
function preferFlats(k: HarmonyKey): boolean {
  const major = isMinorMode(k.mode) ? (k.tonic + 3) % 12 : k.tonic;
  return [1, 3, 5, 8, 10].includes(major);
}
const noteName = (pc: number, flats: boolean) => (flats ? FLAT : SHARP)[((pc % 12) + 12) % 12]!;

const NUMERALS = ["I", "II", "III", "IV", "V", "VI", "VII"];

function quality(scale: number[], i: number, seventh: boolean) {
  const at = (n: number) => (scale[(i + n) % 7]! - scale[i]! + 12) % 12;
  const third = at(2);
  const fifth = at(4);
  const sev = at(6);
  let triad = "";
  let mark = "";
  if (third === 3 && fifth === 6) [triad, mark] = ["dim", "°"];
  else if (third === 4 && fifth === 8) [triad, mark] = ["aug", "+"];
  else if (third === 3) triad = "m";
  if (!seventh) return { symbol: triad, roman: mark, minor: third === 3 };
  const map: Record<string, [string, string]> = {
    "4-7-11": ["maj7", "maj7"],
    "4-7-10": ["7", "7"],
    "3-7-10": ["m7", "7"],
    "3-6-10": ["m7(b5)", "ø7"],
    "3-6-9": ["dim7", "°7"],
    "3-7-11": ["m(maj7)", "(maj7)"],
    "4-8-11": ["maj7(#5)", "+maj7"],
  };
  const [symbol, roman] = map[`${third}-${fifth}-${sev}`] ?? ["7", "7"];
  return { symbol, roman, minor: third === 3 };
}

function fieldOf(key: HarmonyKey): HarmonicField {
  const scale = scaleOf(key.mode);
  const flats = preferFlats(key);
  const label = (i: number, seventh: boolean) => {
    const q = quality(scale, i, seventh);
    const acc = scale[i]! - MAJOR[i]!;
    const prefix = acc < 0 ? "b" : acc > 0 ? "#" : "";
    const num = q.minor ? NUMERALS[i]!.toLowerCase() : NUMERALS[i]!;
    return { symbol: noteName(key.tonic + scale[i]!, flats) + q.symbol, roman: prefix + num + q.roman };
  };
  const mode = keysCatalog.modes.find((m) => m.mode === key.mode);
  return {
    key,
    label: `${noteName(key.tonic, flats)} ${mode?.namePt ?? key.mode}`,
    preferFlats: flats,
    degrees: scale.map((_, i) => ({ degree: i + 1, triad: label(i, false), seventh: label(i, true) })),
  };
}

const ROMAN_RE = /^(b|#)?(VII|VI|IV|V|III|II|I|vii|vi|iv|v|iii|ii|i)([^:]*)(?::(\d+))?$/;

/** Grau romano → cifra no tom (simplificado; o núcleo é a referência). */
function romanToCifra(token: string, key: HarmonyKey): { symbol: string; beats: number } {
  const m = ROMAN_RE.exec(token.trim());
  if (!m) fail("harmony.invalid_roman", `grau romano inválido: ${token}`);
  const [, acc, num, rest = "", beats] = m;
  const idx = NUMERALS.indexOf(num!.toUpperCase());
  const offset = MAJOR[idx]! + (acc === "b" ? -1 : acc === "#" ? 1 : 0);
  const lower = num !== num!.toUpperCase();
  let suffix = rest.replace("°", "dim").replace("ø7", "m7(b5)");
  if (lower && !suffix.startsWith("dim") && !suffix.startsWith("m7(b5)")) suffix = "m" + suffix;
  return { symbol: noteName(key.tonic + offset, preferFlats(key)) + suffix, beats: beats ? Number(beats) : 4 };
}

const CIFRA_RE = /^([A-G])(#|b)?([^:/\s]*)(?:\/([A-G](?:#|b)?))?(?::(\d+))?$/;
function parseCifra(token: string): { root: number; symbol: string; beats: number } {
  const m = CIFRA_RE.exec(token.trim());
  if (!m) fail("harmony.invalid_chord", `cifra inválida: ${token}`);
  const base = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 }[m[1] as "C"];
  const root = (base + (m[2] === "#" ? 1 : m[2] === "b" ? -1 : 0) + 12) % 12;
  return { root, symbol: token.split(":")[0]!, beats: m[5] ? Number(m[5]) : 4 };
}

// ------------------------------------------------------------------ catálogo simulado

interface CatalogEntry {
  slug: string;
  name: string;
  mode: string;
  genres: string[];
  universal: boolean;
  steps: string;
}

const CATALOG: CatalogEntry[] = [
  { slug: "pop-axis", name: "Eixo pop (I–V–vi–IV)", mode: "ionian", genres: ["pop", "rock", "sertanejo"], universal: true, steps: "I:4 V:4 vi:4 IV:4" },
  { slug: "doo-wop", name: "Anos 50 / doo-wop (I–vi–IV–V)", mode: "ionian", genres: ["pop", "rnbSoul", "ballad"], universal: true, steps: "I:4 vi:4 IV:4 V:4" },
  { slug: "pop-punk", name: "Pop melancólico (vi–IV–I–V)", mode: "ionian", genres: ["pop", "rock"], universal: false, steps: "vi:4 IV:4 I:4 V:4" },
  { slug: "ii-v-i-major", name: "ii–V–I maior", mode: "ionian", genres: ["jazz", "bossaNova", "mpb"], universal: true, steps: "ii7:4 V7:4 Imaj7:8" },
  { slug: "samba-i-vi7", name: "Samba (I–VI7–ii7–V7)", mode: "ionian", genres: ["samba", "mpb"], universal: false, steps: "I:4 VI7:4 ii7:4 V7:4" },
  { slug: "rock-bvii", name: "Rock mixolídio (I–bVII–IV–I)", mode: "ionian", genres: ["rock"], universal: false, steps: "I:4 bVII:4 IV:4 I:4" },
  { slug: "andalusian", name: "Cadência andaluza (i–bVII–bVI–V)", mode: "aeolian", genres: ["rock", "pop"], universal: false, steps: "i:4 bVII:4 bVI:4 V:4" },
  { slug: "sertanejo-i-iv-v", name: "Sertanejo (I–IV–V–I)", mode: "ionian", genres: ["sertanejo", "forro"], universal: false, steps: "I:4 IV:4 V:4 I:4" },
  { slug: "blues-basic", name: "Blues (I7–IV7–I7–V7)", mode: "ionian", genres: ["blues", "funk"], universal: false, steps: "I7:4 IV7:4 I7:4 V7:4" },
  { slug: "gospel-iv-v-iii-vi", name: "Gospel (IV–V–iii–vi)", mode: "ionian", genres: ["gospel", "ballad"], universal: false, steps: "IV:4 V:4 iii:4 vi:4" },
  { slug: "reggae-i-iv", name: "Reggae (I–IV)", mode: "ionian", genres: ["reggae"], universal: false, steps: "I:4 IV:4" },
  { slug: "minor-i-iv-v", name: "Menor (i–iv–V7–i)", mode: "aeolian", genres: ["ballad", "mpb"], universal: true, steps: "i:4 iv:4 V7:4 i:4" },
];

interface UserProg {
  id: string;
  name: string;
  mode: string;
  genres: string[];
  beatsPerBar: number;
  steps: string;
}

const tokens = (steps: string) => steps.trim().split(/\s+/).filter(Boolean);

function viewOf(e: CatalogEntry | UserProg, origin: "builtin" | "user"): ProgressionView {
  const t = tokens(e.steps);
  return {
    ref: origin === "builtin" ? { kind: "builtin", slug: (e as CatalogEntry).slug } : { kind: "user", id: (e as UserProg).id },
    name: e.name,
    origin,
    mode: e.mode,
    genres: e.genres,
    universal: origin === "builtin" ? (e as CatalogEntry).universal : false,
    beatsPerBar: origin === "builtin" ? 4 : (e as UserProg).beatsPerBar,
    stepsText: e.steps,
    roman: t.map((x) => x.split(":")[0]!),
    beats: t.map((x) => Number(x.split(":")[1] ?? 4)),
  };
}

const BASE_STARTERS = (): LibraryBase[] => [
  { id: uuid(), name: "Pop básico em Dó", kind: { kind: "generated", progression: "I:4 V:4 vi:4 IV:4", bars: 8, style: "popStrum", drums: "popRock", bass: "rootFifth" }, bpm: 100, key: { tonic: 0, mode: "major" }, meter: { numerator: 4, denominator: 4 }, genre: "pop", instruments: ["drums", "bass", "acousticGuitar"], tags: ["inicial"] },
  { id: uuid(), name: "Bossa em Ré (ii–V–I)", kind: { kind: "generated", progression: "ii7:4 V7:4 Imaj7:8", bars: 8, style: "bossaComping", drums: "bossa", bass: "bossaRootFifth" }, bpm: 130, key: { tonic: 2, mode: "major" }, meter: { numerator: 4, denominator: 4 }, genre: "bossa_nova", instruments: ["drums", "bass", "nylonGuitar"], tags: ["inicial", "suave"] },
  { id: uuid(), name: "Valsa em Sol", kind: { kind: "generated", progression: "I:3 IV:3 V:3 I:3", bars: 8, style: "waltz", drums: "waltz", bass: "waltzRootFifth" }, bpm: 90, key: { tonic: 7, mode: "major" }, meter: { numerator: 3, denominator: 4 }, genre: "mpb", instruments: ["drums", "bass", "piano"], tags: ["inicial"] },
  { id: uuid(), name: "Loop de bateria 96", kind: { kind: "userAudio", path: "loop-96.wav", progression: "i iv" }, bpm: 96, key: { tonic: 9, mode: "minor" }, meter: { numerator: 4, denominator: 4 }, genre: "pop", instruments: ["drums"], tags: ["loop"] },
];

/** WAV PCM 16 bits estéreo curto e válido (tom de 440 Hz com fade), para a WebAudio decodificar. */
export function tinyWav(seconds = 0.25, rate = 8000): ArrayBuffer {
  const frames = Math.max(1, Math.round(seconds * rate));
  const buf = new ArrayBuffer(44 + frames * 4);
  const dv = new DataView(buf);
  const str = (o: number, s: string) => [...s].forEach((c, i) => dv.setUint8(o + i, c.charCodeAt(0)));
  str(0, "RIFF");
  dv.setUint32(4, 36 + frames * 4, true);
  str(8, "WAVE");
  str(12, "fmt ");
  dv.setUint32(16, 16, true);
  dv.setUint16(20, 1, true);
  dv.setUint16(22, 2, true);
  dv.setUint32(24, rate, true);
  dv.setUint32(28, rate * 4, true);
  dv.setUint16(32, 4, true);
  dv.setUint16(34, 16, true);
  str(36, "data");
  dv.setUint32(40, frames * 4, true);
  for (let i = 0; i < frames; i++) {
    const env = Math.min(1, i / 200, (frames - i) / 200);
    const v = Math.round(Math.sin((2 * Math.PI * 440 * i) / rate) * 0.2 * env * 32767);
    dv.setInt16(44 + i * 4, v, true);
    dv.setInt16(46 + i * 4, v, true);
  }
  return buf;
}

interface LibSnapshot {
  user: UserProg[];
  bases: LibraryBase[];
}

/** Acrescenta os comandos de composição a um transporte simulado do projeto. */
export function withCompositionMock(inner: Transport): Transport {
  let lib: LibSnapshot = { user: [], bases: BASE_STARTERS() };
  const undo: { label: string; snap: LibSnapshot }[] = [];
  const redo: { label: string; snap: LibSnapshot }[] = [];
  let synth: SynthStatus = { engine: "simple", soundfontPath: null, bankName: null, presetCount: 0 };

  const history = (): LibraryHistory => ({ canUndo: undo.length > 0, canRedo: redo.length > 0, undoLabel: undo.at(-1)?.label ?? null, redoLabel: redo.at(-1)?.label ?? null });
  function change<T>(label: string, fn: (l: LibSnapshot) => T): T {
    const before = structuredClone(lib);
    const r = fn(lib);
    undo.push({ label, snap: before });
    redo.length = 0;
    return r;
  }

  const session = () => inner.invoke<SessionView>("project_view");
  const projectBpm = async () => (await session()).tempo.tempos[0]?.bpm ?? 120;

  function resolve(ref: ProgressionRef, key: HarmonyKey): { name: string; chords: { symbol: string; beats: number }[] } {
    switch (ref.kind) {
      case "builtin": {
        const e = CATALOG.find((c) => c.slug === ref.slug) ?? fail("harmony.unknown_progression", `progressão não encontrada: ${ref.slug}`);
        return { name: e.name, chords: tokens(e.steps).map((t) => romanToCifra(t, key)) };
      }
      case "user": {
        const u = lib.user.find((x) => x.id === ref.id) ?? fail("harmony.unknown_progression", "progressão do usuário não encontrada");
        return { name: u.name, chords: tokens(u.steps).map((t) => romanToCifra(t, key)) };
      }
      case "roman":
        if (!ref.steps.trim()) fail("harmony.invalid_progression", "progressão vazia");
        return { name: "Progressão", chords: tokens(ref.steps).map((t) => romanToCifra(t, key)) };
      case "cifra":
        if (!ref.chords.trim()) fail("harmony.invalid_progression", "progressão vazia");
        return { name: "Sequência", chords: tokens(ref.chords).map((t) => parseCifra(t)) };
    }
  }

  function validateUser(input: UserProgressionInput, ignoreId?: string) {
    const name = input.name.trim();
    if (!name || [...name].length > 64) fail("harmony.invalid_name", "nome inválido: deve ter de 1 a 64 caracteres");
    if (lib.user.some((u) => u.id !== ignoreId && u.name.toLowerCase() === name.toLowerCase())) fail("harmony.duplicate_name", `já existe uma progressão chamada "${name}"`);
    if (!tokens(input.steps).length) fail("harmony.invalid_progression", "informe ao menos um grau");
    tokens(input.steps).forEach((t) => romanToCifra(t, { tonic: 0, mode: input.mode }));
    return { name, mode: input.mode, genres: input.genres ?? [], beatsPerBar: input.beatsPerBar ?? 4, steps: input.steps.trim() };
  }

  async function insertAudio(name: string, trackId: string | null | undefined, atTick: number): Promise<InsertedClip> {
    if (!(atTick >= 0)) fail("composition.invalid_position", "Posição negativa.");
    const before = await session();
    const known = new Set(before.tracks.flatMap((t) => t.clips.map((c) => c.id)));
    const view = await inner.invoke<SessionView>("audio_import", { paths: [`/simulado/${name}.wav`], trackId: trackId ?? null, atTick });
    for (const t of view.tracks)
      for (const c of t.clips)
        if (!known.has(c.id)) {
          const src = view.sources.find((s) => s.id === c.sourceId);
          return { trackId: t.id, clipId: c.id, sourceId: c.sourceId, relPath: `audio/generated/${src?.id ?? uuid()}.wav`, frames: c.lengthFrames, view };
        }
    fail("app.internal", "clipe não inserido");
  }

  function mismatchOf(base: LibraryBase, bpm: number, projectKey: BaseKey | null | undefined): MismatchReport {
    const bpmRatio = base.bpm / bpm;
    let semitoneShift: number | null = null;
    let modeDiffers = false;
    if (base.key && projectKey) {
      modeDiffers = base.key.mode !== projectKey.mode;
      // modos diferentes: mira a relativa do tom do projeto
      const target = !modeDiffers ? projectKey.tonic : projectKey.mode === "major" ? projectKey.tonic + 9 : projectKey.tonic + 3;
      let d = (((target - base.key.tonic) % 12) + 12) % 12;
      if (d > 5) d -= 12;
      semitoneShift = d;
    }
    const needsDecision = Math.abs(bpmRatio - 1) > 1e-9 || (semitoneShift ?? 0) !== 0 || modeDiffers;
    return { bpmRatio, semitoneShift, modeDiffers, needsDecision };
  }

  function generate(req: GenerateBaseRequest) {
    if (!(req.bpm >= 20 && req.bpm <= 999)) fail("library.invalid_bpm", `BPM fora do intervalo 20–999: ${req.bpm}`);
    if (!(req.bars >= 1 && req.bars <= 256)) fail("composition.invalid_bars", "Compassos devem estar entre 1 e 256.");
    const style = baseOptions.styles.find((s) => s.value === req.style) ?? fail("library.invalid_style", "estilo desconhecido");
    const numerator = style.meterNumerator ?? 4;
    const chords = resolve(req.progression, req.key).chords;
    const drums = req.drums ?? (numerator === 3 ? "waltz" : "popRock");
    const bass = req.bass ?? (numerator === 3 ? "waltzRootFifth" : "rootFifth");
    const barTicks = numerator * PPQ;
    const events = Array.from({ length: req.bars * numerator }, (_, i) => ({ startTick: i * PPQ, durationTicks: PPQ / 2, channel: 9, program: 0, key: i % numerator === 0 ? 36 : 42, velocity: 90 }));
    const recipe = chords.map((c) => `${c.symbol}:${c.beats}`).join(" ");
    return { bpm: req.bpm, meter: { numerator, denominator: 4 }, totalTicks: req.bars * barTicks, recipeProgression: recipe, drums, bass, events };
  }

  const baseOf = (id: string) => lib.bases.find((b) => b.id === id) ?? fail("library.base_not_found", "base não encontrada");

  const handlers: Record<string, (a: Record<string, any>) => unknown> = {
    harmony_keys: () => keysCatalog,
    harmony_field: (a) => fieldOf(a.key),
    harmony_progressions: (a): ProgressionSuggestion[] => {
      const key: HarmonyKey = a.req.key;
      const minor = isMinorMode(key.mode);
      const genre: string | null = a.req.genre ?? null;
      const all: [CatalogEntry | UserProg, "builtin" | "user"][] = [...CATALOG.map((c) => [c, "builtin"] as [CatalogEntry, "builtin"]), ...lib.user.map((u) => [u, "user"] as [UserProg, "user"])];
      const out: ProgressionSuggestion[] = [];
      for (const [e, origin] of all) {
        const universal = origin === "builtin" && (e as CatalogEntry).universal;
        const g = !genre ? 1 : e.genres.includes(genre) ? 1 : universal ? 0.5 : 0;
        if (g === 0) continue;
        const m = e.mode === key.mode ? 1 : isMinorMode(e.mode) === minor ? 0.5 : 0;
        if (m === 0) continue;
        const score = (0.6 * g + 0.4 * m) * 0.95 + (origin === "user" ? 0.05 : 0);
        out.push({ progression: viewOf(e, origin), score, cifra: tokens(e.steps).map((t) => romanToCifra(t, key).symbol) });
      }
      out.sort((x, y) => y.score - x.score || x.progression.name.localeCompare(y.progression.name));
      return out.slice(0, a.req.limit ?? 20);
    },
    harmony_user_progressions: () => lib.user.map((u) => viewOf(u, "user")),
    harmony_suggest_next: (a): ChordSuggestion[] => {
      const key: HarmonyKey = a.req.key;
      const f = fieldOf(key);
      const history: string[] = a.req.history ?? [];
      const triads = f.degrees.map((d) => d.triad);
      const label = (r: string) => ({ diatonic: "Acorde do campo harmônico", authenticCadence: "Cadência autêntica (V → I)", deceptiveCadence: "Cadência deceptiva (V → vi)", circleOfFifths: "Movimento por quintas" })[r] ?? r;
      const scored: { symbol: string; roman: string; score: number; reason: string }[] = [];
      const last = history.at(-1);
      if (!last) {
        triads.forEach((t, i) => scored.push({ ...t, score: i === 0 ? 1 : 0.6 - i * 0.05, reason: "diatonic" }));
      } else {
        const p = parseCifra(last);
        const rel = (p.root - key.tonic + 12) % 12;
        const dominant = rel === 7;
        triads.forEach((t, i) => {
          const root = (key.tonic + scaleOf(key.mode)[i]!) % 12;
          let score = 0.4;
          let reason = "diatonic";
          if (dominant && i === 0) [score, reason] = [1, "authenticCadence"];
          else if (dominant && i === 5) [score, reason] = [0.62, "deceptiveCadence"];
          else if ((p.root + 5) % 12 === root) [score, reason] = [0.9, "circleOfFifths"];
          if (t.symbol === p.symbol) score -= 0.1;
          scored.push({ ...t, score, reason });
        });
      }
      scored.sort((x, y) => y.score - x.score);
      const top = scored[0]?.score ?? 1;
      return scored.slice(0, a.req.limit ?? 8).map((s) => ({ symbol: s.symbol, roman: s.roman, score: s.score / top, reason: s.reason, reasonLabel: label(s.reason) }));
    },
    harmony_realize: async (a) => {
      const req: RealizeRequest = a.req;
      const r = resolve(req.progression, req.key);
      let t = 0;
      const chords = r.chords.map((c) => {
        const out = { symbol: c.symbol, startTick: t, durationTicks: c.beats * PPQ, notes: [60, 64, 67], bass: 48 };
        t += c.beats * PPQ;
        return out;
      });
      return { name: r.name, key: req.key, bpm: req.bpm ?? (await projectBpm().catch(() => 120)), beatsPerBar: 4, totalTicks: t, chords, events: [] };
    },
    harmony_insert_progression: async (a) => {
      const r = resolve(a.req.realize.progression, a.req.realize.key);
      return insertAudio(`Progressão ${r.name}`, a.req.trackId, a.req.atTick);
    },
    harmony_user_progression_add: (a) =>
      change("Cadastrar progressão", (l) => {
        const u = { id: uuid(), ...validateUser(a.input) };
        l.user.push(u);
        return viewOf(u, "user");
      }),
    harmony_user_progression_edit: (a) => {
      const i = lib.user.findIndex((u) => u.id === a.id);
      if (i < 0) fail("harmony.unknown_progression", "progressão do usuário não encontrada");
      const v = validateUser(a.input, a.id);
      return change("Editar progressão", (l) => {
        l.user[i] = { id: a.id, ...v };
        return viewOf(l.user[i]!, "user");
      });
    },
    harmony_user_progression_remove: (a) => {
      if (!lib.user.some((u) => u.id === a.id)) fail("harmony.unknown_progression", "progressão do usuário não encontrada");
      change("Remover progressão", (l) => (l.user = l.user.filter((u) => u.id !== a.id)));
      return history();
    },
    library_history: () => history(),
    library_undo: () => {
      const e = undo.pop();
      if (e) {
        redo.push({ label: e.label, snap: structuredClone(lib) });
        lib = e.snap;
      }
      return history();
    },
    library_redo: () => {
      const e = redo.pop();
      if (e) {
        undo.push({ label: e.label, snap: structuredClone(lib) });
        lib = e.snap;
      }
      return history();
    },
    bases_options: () => baseOptions,
    bases_search: (a) => {
      const q = a.query ?? {};
      const norm = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
      return lib.bases.filter((b) => {
        if (q.text) {
          const hay = norm([b.name, b.genre ?? "", ...b.tags, b.kind.progression ?? ""].join(" "));
          if (!hay.includes(norm(q.text))) return false;
        }
        if (q.bpmMin !== undefined && b.bpm < q.bpmMin) return false;
        if (q.bpmMax !== undefined && b.bpm > q.bpmMax) return false;
        if (q.key && (!b.key || b.key.tonic !== q.key.tonic || b.key.mode !== q.key.mode)) return false;
        if (q.genre && b.genre !== q.genre) return false;
        if (q.instrument && !b.instruments.includes(q.instrument)) return false;
        if (q.tags && !q.tags.every((t: string) => b.tags.includes(t))) return false;
        if (q.kind && b.kind.kind !== q.kind) return false;
        return true;
      });
    },
    bases_add_user: (a) => {
      const ext = String(a.path).split(".").pop()?.toLowerCase() ?? "";
      if (!["wav", "flac", "mp3", "ogg", "aiff", "aif"].includes(ext)) fail("composition.unsupported_audio", "Formato de áudio não suportado.");
      const m = a.meta;
      if (!m.name?.trim()) fail("library.invalid_name", "nome inválido");
      if (!(m.bpm >= 20 && m.bpm <= 999)) fail("library.invalid_bpm", `BPM fora do intervalo 20–999: ${m.bpm}`);
      return change("Adicionar base", (l) => {
        const id = uuid();
        const b: LibraryBase = { id, name: m.name.trim(), kind: { kind: "userAudio", path: `${id}.${ext}`, progression: m.progression ?? null }, bpm: m.bpm, key: m.key ?? null, meter: m.meter ?? { numerator: 4, denominator: 4 }, genre: m.genre ?? null, instruments: m.instruments ?? [], tags: m.tags ?? [] };
        l.bases.push(b);
        return b;
      });
    },
    bases_generate: (a) => generate(a.req),
    bases_save_generated: (a) => {
      const req = a.req;
      const g = generate(req.generation);
      if (!req.name?.trim()) fail("library.invalid_name", "nome inválido");
      return change("Salvar base gerada", (l) => {
        const b: LibraryBase = {
          id: uuid(),
          name: req.name.trim(),
          kind: { kind: "generated", progression: g.recipeProgression, bars: req.generation.bars, style: req.generation.style, drums: g.drums, bass: g.bass },
          bpm: req.generation.bpm,
          key: { tonic: req.generation.key.tonic, mode: isMinorMode(req.generation.key.mode) ? "minor" : "major" },
          meter: g.meter,
          genre: req.genre ?? null,
          instruments: ["drums", "bass"],
          tags: req.tags ?? [],
        };
        l.bases.push(b);
        return b;
      });
    },
    bases_remove: (a) => {
      baseOf(a.id);
      change("Remover base", (l) => (l.bases = l.bases.filter((b) => b.id !== a.id)));
      return history();
    },
    bases_mismatch: async (a) => mismatchOf(baseOf(a.req.baseId), await projectBpm(), a.req.projectKey),
    bases_insert: async (a): Promise<InsertBaseOutcome> => {
      const base = baseOf(a.req.baseId);
      const report = mismatchOf(base, await projectBpm(), a.req.projectKey);
      if (report.needsDecision && !a.req.decision) return { status: "needsDecision", report };
      const decision = report.needsDecision ? a.req.decision : "keep";
      const clip = await insertAudio(`Base ${base.name}`, a.req.trackId, a.req.atTick);
      return { status: "inserted", report, decision, clip };
    },
    synth_status: () => synth,
    synth_set_soundfont: (a) => {
      if (a.path === null) synth = { engine: "simple", soundfontPath: null, bankName: null, presetCount: 0 };
      else {
        if (!/\.sf2$/i.test(a.path)) fail("synth.invalid_soundfont", "Arquivo SoundFont inválido.");
        synth = { engine: "soundfont", soundfontPath: a.path, bankName: "Banco simulado", presetCount: 128 };
      }
      return synth;
    },
  };

  const BINARY = new Set(["harmony_audition", "bases_generated_audition", "bases_audition"]);

  return {
    ...inner,
    async invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
      const h = handlers[cmd];
      if (!h) return inner.invoke<T>(cmd, args);
      await new Promise((r) => setTimeout(r, 0));
      return (await h(args as Record<string, any>)) as T;
    },
    async invokeBinary(cmd, args = {}) {
      if (!BINARY.has(cmd)) return inner.invokeBinary(cmd, args);
      const a = args as Record<string, any>;
      if (cmd === "harmony_audition") resolve(a.req.progression, a.req.key);
      if (cmd === "bases_generated_audition") generate(a.req);
      if (cmd === "bases_audition") baseOf(a.id);
      return tinyWav();
    },
    async pickOpen(opts) {
      const exts = opts.filters?.flatMap((f) => f.extensions) ?? [];
      if (exts.includes("sf2")) return ["/home/usuario/Musica/GeneralUser.sf2"];
      return inner.pickOpen(opts);
    },
  };
}
