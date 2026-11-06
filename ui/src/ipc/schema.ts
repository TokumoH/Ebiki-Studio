// Contrato IPC (espelha os DTOs serde do núcleo Rust). Os tipos TS são inferidos dos esquemas zod,
// e os esquemas são validados contra fixtures geradas pelo Rust (contract.test.ts).
import { z } from "zod";

export const Id = z.string().uuid();

export const EqBand = z.object({
  kind: z.enum(["highPass", "lowShelf", "peak", "highShelf", "lowPass"]),
  enabled: z.boolean(),
  freqHz: z.number(),
  gainDb: z.number(),
  q: z.number(),
});

export const Effect = z.union([
  z.object({ type: z.literal("eq"), params: z.object({ bands: z.array(EqBand) }) }),
  z.object({
    type: z.literal("compressor"),
    params: z.object({ thresholdDb: z.number(), ratio: z.number(), attackMs: z.number(), releaseMs: z.number(), kneeDb: z.number(), makeupDb: z.number() }),
  }),
  z.object({
    type: z.literal("gate"),
    params: z.object({ thresholdDb: z.number(), attackMs: z.number(), holdMs: z.number(), releaseMs: z.number(), rangeDb: z.number() }),
  }),
  z.object({ type: z.literal("limiter"), params: z.object({ ceilingDb: z.number(), inputGainDb: z.number(), releaseMs: z.number() }) }),
  z.object({
    type: z.literal("reverb"),
    params: z.object({ roomSize: z.number(), damping: z.number(), width: z.number(), predelayMs: z.number(), mix: z.number() }),
  }),
  z.object({
    type: z.literal("delay"),
    params: z.object({ timeMs: z.number(), feedback: z.number(), highCutHz: z.number(), mix: z.number(), pingPong: z.boolean() }),
  }),
  z.object({ type: z.literal("saturation"), params: z.object({ driveDb: z.number(), tone: z.number(), mix: z.number() }) }),
  z.object({ type: z.literal("chorus"), params: z.object({ rateHz: z.number(), depthMs: z.number(), mix: z.number() }) }),
  z.object({ type: z.literal("deEsser"), params: z.object({ freqHz: z.number(), thresholdDb: z.number(), reductionDb: z.number() }) }),
  z.object({ type: z.literal("pitchCorrector"), params: z.lazy(() => PitchCorrectorParams) }),
  // efeitos adicionados pelo núcleo em versões futuras chegam aqui sem quebrar a UI
  z.object({ type: z.string(), params: z.record(z.string(), z.unknown()) }),
]);

export const EffectSlot = z.object({ id: Id, bypass: z.boolean(), effect: Effect });
export const Output = z.discriminatedUnion("kind", [z.object({ kind: z.literal("master") }), z.object({ kind: z.literal("bus"), id: Id })]);
export const AuxSend = z.object({ id: Id, bus: Id, level: z.number(), preFader: z.boolean() });

export const ChannelStrip = z.object({
  volume: z.number(),
  pan: z.number(),
  mute: z.boolean(),
  solo: z.boolean(),
  inserts: z.array(EffectSlot),
  sends: z.array(AuxSend),
  output: Output,
});

export const InputAssignment = z.object({ left: z.number().int(), right: z.number().int().nullable() });
export const MonitorMode = z.enum(["off", "whenArmed", "always"]);
export const Take = z.object({ id: Id, sourceId: Id, start: z.number().int(), lengthFrames: z.number().int(), number: z.number().int() });
export const TrackRecording = z.object({
  armed: z.boolean(),
  input: InputAssignment.nullable(),
  monitor: MonitorMode,
  takes: z.array(Take),
  activeTake: Id.nullable(),
});

/** Forma da curva de fade (Fase 3). */
export const FadeShape = z.enum(["linear", "equalPower"]);

export const Clip = z.object({
  id: Id,
  sourceId: Id,
  sourceName: z.string(),
  start: z.number().int(),
  startFrame: z.number().int(),
  offsetFrames: z.number().int(),
  lengthFrames: z.number().int(),
  endTick: z.number().int(),
  gain: z.number(),
  // Fase 3 — edição não destrutiva
  sourceFrames: z.number().int().nonnegative(),
  fadeInFrames: z.number().int().nonnegative(),
  fadeOutFrames: z.number().int().nonnegative(),
  fadeInShape: FadeShape,
  fadeOutShape: FadeShape,
  polarityInverted: z.boolean(),
  reversed: z.boolean(),
});

/** Fades enviados em `clip_set_fades`. */
export const FadeRequest = z.object({
  fadeInFrames: z.number().int().nonnegative(),
  fadeOutFrames: z.number().int().nonnegative(),
  fadeInShape: FadeShape,
  fadeOutShape: FadeShape,
});

/** Parâmetros de `source_spectrogram` (resposta binária `AMSG`, ver ipc/spectrogram.ts). */
export const SpectrogramRequest = z.object({
  fftSize: z.number().int(),
  hop: z.number().int(),
  bins: z.number().int(),
});

export const Track = z.object({
  id: Id,
  name: z.string(),
  clips: z.array(Clip),
  strip: ChannelStrip,
  recording: TrackRecording,
  audible: z.boolean(),
  meterSlot: z.number().int(),
});

export const BusView = z.object({ id: Id, name: z.string(), strip: ChannelStrip, audible: z.boolean(), meterSlot: z.number().int() });

export const TempoMap = z.object({
  tempos: z.array(z.object({ tick: z.number().int(), bpm: z.number() })),
  meters: z.array(z.object({ tick: z.number().int(), numerator: z.number().int(), denominator: z.number().int() })),
});

const TickRange = z.tuple([z.number().int(), z.number().int()]);

export const SessionView = z.object({
  project: z.object({ id: Id, name: z.string(), root: z.string(), sampleRate: z.number().int(), bitDepth: z.number().int() }),
  tempo: TempoMap,
  tracks: z.array(Track),
  buses: z.array(BusView),
  master: ChannelStrip,
  sources: z.array(
    z.object({ id: Id, contentHash: z.string(), relPath: z.string(), originalName: z.string(), channels: z.number().int(), lengthFrames: z.number().int() }),
  ),
  recording: z.object({
    metronome: z.object({ enabled: z.boolean(), volume: z.number(), subdivision: z.number().int(), accentFirstBeat: z.boolean(), countInBars: z.number().int() }),
    punch: TickRange.nullable(),
    loopRange: TickRange.nullable(),
    latencyCompensationFrames: z.number().int(),
  }),
  history: z.object({ canUndo: z.boolean(), canRedo: z.boolean(), undoLabel: z.string().nullable(), redoLabel: z.string().nullable(), dirty: z.boolean() }),
  contentEndFrame: z.number().int(),
  // Fases 6–8 (esquemas na seção "Música" ao fim do arquivo; lazy por ordem de declaração)
  lyrics: z.lazy(() => LyricTrack),
  scores: z.lazy(() => ScoreSummary.array()),
  tuning: z.lazy(() => TuningView),
});

export const TransportView = z.object({
  state: z.enum(["stopped", "playing", "paused", "countin", "recording"]),
  frame: z.number().int(),
  tick: z.number().int(),
  seconds: z.number(),
  barBeat: z.object({ bar: z.number().int(), beat: z.number().int(), tickInBeat: z.number().int() }),
  bpm: z.number(),
  recordStart: z.number().int().nullable(),
});

export const MetersView = z.object({
  master: z.tuple([z.number(), z.number()]),
  tracks: z.array(z.object({ id: Id, peak: z.tuple([z.number(), z.number()]) })),
  buses: z.array(z.object({ id: Id, peak: z.tuple([z.number(), z.number()]) })),
  /** Picos por canal de entrada do dispositivo (vazio sem entrada). */
  inputs: z.array(z.number()).default([]),
  xruns: z.number().int(),
  recordOverflow: z.number().int(),
});

export const StripTarget = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("track"), id: Id }),
  z.object({ kind: z.literal("bus"), id: Id }),
  z.object({ kind: z.literal("master") }),
]);

export const StripEdit = z.discriminatedUnion("op", [
  z.object({ op: z.literal("volume"), value: z.number() }),
  z.object({ op: z.literal("pan"), value: z.number() }),
  z.object({ op: z.literal("mute"), value: z.boolean() }),
  z.object({ op: z.literal("solo"), value: z.boolean() }),
  z.object({ op: z.literal("addEffect"), slot: EffectSlot, index: z.number().int().nullable() }),
  z.object({ op: z.literal("removeEffect"), id: Id }),
  z.object({ op: z.literal("moveEffect"), id: Id, to: z.number().int() }),
  z.object({ op: z.literal("setEffect"), id: Id, effect: Effect }),
  z.object({ op: z.literal("bypass"), id: Id, value: z.boolean() }),
  z.object({ op: z.literal("output"), value: Output }),
  z.object({ op: z.literal("addSend"), send: AuxSend }),
  z.object({ op: z.literal("removeSend"), id: Id }),
  z.object({ op: z.literal("sendLevel"), id: Id, value: z.number() }),
]);

export const ApiError = z.object({ code: z.string(), message: z.string() });

export const RecentProject = z.object({ path: z.string(), name: z.string(), openedAt: z.number().int() });
export const DeviceInfo = z.object({ id: z.string(), name: z.string(), isDefault: z.boolean() });
export const DeviceList = z.object({ host: z.string(), outputs: z.array(DeviceInfo), inputs: z.array(DeviceInfo) });
export const AudioStatus = z.object({
  mode: z.string(),
  outputName: z.string().nullable(),
  inputName: z.string().nullable(),
  outputId: z.string().nullable().default(null),
  inputId: z.string().nullable().default(null),
  sampleRate: z.number().int(),
  bufferSize: z.number().int().nullable(),
  inputChannels: z.number().int(),
  estimatedLatencyMs: z.number().nullable(),
  error: z.string().nullable(),
});
/** "Ouvir entrada": canal (0-based) ou par estéreo, ganho em dB. */
export const DeviceMonitor = z.object({ left: z.number().int().min(0), right: z.number().int().min(0).nullable(), gainDb: z.number() });
export const ExportFormat = z.enum(["wav", "aiff", "flac", "mp3", "ogg", "aac"]);
export const ExportSettings = z.object({
  format: ExportFormat,
  bitDepth: z.number().int(),
  mp3Kbps: z.number().int().nullable(),
  vorbisQuality: z.number(),
  normalize: z.boolean(),
});
export const Advice = z.object({ severity: z.enum(["info", "warning", "critical"]), code: z.string(), channel: z.string().nullable(), message: z.string() });
export const MixReport = z.object({
  analysis: z.object({
    master: z.object({ peakDb: z.number(), truePeakDb: z.number(), rmsDb: z.number(), integratedLufs: z.number().nullable(), correlation: z.number(), clippedSamples: z.number() }),
    channels: z.array(z.object({ name: z.string(), peakDb: z.number(), rmsDb: z.number(), clippedSamples: z.number() })),
    targetLufs: z.number(),
  }),
  advice: z.array(Advice),
});
export const PresetInfo = z.object({ id: z.string(), label: z.string() });

// ---- Harmonia e bases (Fases 4–5; spec fase-5 §11.5) ----
// Enumerações do núcleo (modos, gêneros, padrões) ficam como `string`: valores novos não quebram a UI.

const PitchClassNum = z.number().int().min(0).max(11);
export const HarmonyKey = z.object({ tonic: PitchClassNum, mode: z.string() });
export const BaseKey = z.object({ tonic: PitchClassNum, mode: z.enum(["major", "minor"]) });
export const BaseMeter = z.object({ numerator: z.number().int(), denominator: z.number().int() });

export const HarmonyKeysCatalog = z.object({
  tonics: z.array(z.object({ value: PitchClassNum, sharp: z.string(), flat: z.string() })),
  modes: z.array(z.object({ mode: z.string(), code: z.string(), namePt: z.string(), isMinor: z.boolean() })),
  genres: z.array(z.object({ genre: z.string(), code: z.string(), namePt: z.string() })),
});

const ChordLabel = z.object({ symbol: z.string(), roman: z.string() });
export const HarmonicField = z.object({
  key: HarmonyKey,
  label: z.string(),
  preferFlats: z.boolean(),
  degrees: z.array(z.object({ degree: z.number().int(), triad: ChordLabel, seventh: ChordLabel })),
});

export const ProgressionRef = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("builtin"), slug: z.string() }),
  z.object({ kind: z.literal("user"), id: z.string() }),
  z.object({ kind: z.literal("roman"), steps: z.string(), mode: z.string(), beatsPerBar: z.number().int().optional() }),
  z.object({ kind: z.literal("cifra"), chords: z.string(), beatsPerBar: z.number().int().optional() }),
]);

export const ProgressionView = z.object({
  ref: ProgressionRef,
  name: z.string(),
  origin: z.enum(["builtin", "user"]),
  mode: z.string(),
  genres: z.array(z.string()),
  universal: z.boolean(),
  beatsPerBar: z.number().int(),
  stepsText: z.string(),
  roman: z.array(z.string()),
  beats: z.array(z.number().int()),
});
export const ProgressionSuggestion = z.object({ progression: ProgressionView, score: z.number(), cifra: z.array(z.string()) });
export const ChordSuggestion = z.object({ symbol: z.string(), roman: z.string(), score: z.number(), reason: z.string(), reasonLabel: z.string() });

export const CompositionNoteEvent = z.object({
  startTick: z.number().int(),
  durationTicks: z.number().int(),
  channel: z.number().int(),
  program: z.number().int(),
  key: z.number().int(),
  velocity: z.number().int(),
});

export const Realization = z.object({
  name: z.string(),
  key: HarmonyKey,
  bpm: z.number(),
  beatsPerBar: z.number().int(),
  totalTicks: z.number().int(),
  chords: z.array(
    z.object({ symbol: z.string(), startTick: z.number().int(), durationTicks: z.number().int(), notes: z.array(z.number().int()), bass: z.number().int().nullable() }),
  ),
  events: z.array(CompositionNoteEvent),
});

export const InsertedClip = z.object({
  trackId: Id,
  clipId: Id,
  sourceId: Id,
  relPath: z.string(),
  frames: z.number().int(),
  view: SessionView,
});

export const LibraryHistory = z.object({ canUndo: z.boolean(), canRedo: z.boolean(), undoLabel: z.string().nullable(), redoLabel: z.string().nullable() });

const PatternOption = z.object({ value: z.string(), code: z.string(), namePt: z.string(), meterNumerator: z.number().int().nullable() });
export const BaseOptions = z.object({
  styles: z.array(PatternOption),
  drums: z.array(PatternOption),
  basses: z.array(PatternOption),
  instruments: z.array(PatternOption),
});

export const BaseKindTag = z.enum(["bundledCc0", "generated", "userAudio"]);
export const LibraryBaseKind = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("generated"), progression: z.string(), bars: z.number().int(), style: z.string(), drums: z.string(), bass: z.string() }),
  z.object({ kind: z.literal("userAudio"), path: z.string(), progression: z.string().nullable() }),
  z.object({ kind: z.literal("bundledCc0"), path: z.string(), progression: z.string().nullable() }),
]);
export const LibraryBase = z.object({
  id: Id,
  name: z.string(),
  kind: LibraryBaseKind,
  bpm: z.number(),
  key: BaseKey.nullable(),
  meter: BaseMeter,
  genre: z.string().nullable(),
  instruments: z.array(z.string()),
  tags: z.array(z.string()),
});

export const GeneratedBase = z.object({
  bpm: z.number(),
  meter: BaseMeter,
  totalTicks: z.number().int(),
  recipeProgression: z.string(),
  drums: z.string(),
  bass: z.string(),
  events: z.array(CompositionNoteEvent),
});

export const MismatchReport = z.object({ bpmRatio: z.number(), semitoneShift: z.number().int().nullable(), modeDiffers: z.boolean(), needsDecision: z.boolean() });
export const BaseDecision = z.enum(["keep", "adapt"]);
export const InsertBaseOutcome = z.discriminatedUnion("status", [
  z.object({ status: z.literal("needsDecision"), report: MismatchReport }),
  z.object({ status: z.literal("inserted"), report: MismatchReport, decision: BaseDecision, clip: InsertedClip }),
]);

export const SynthStatus = z.object({ engine: z.enum(["soundfont", "simple"]), soundfontPath: z.string().nullable(), bankName: z.string().nullable(), presetCount: z.number().int() });

// ------------------------------------------------------------------------------------------------
// Música (Fases 6–8): letra sincronizada, partituras e afinação. Contrato: specs
// 2026-11-05-fase-7-notacao.md e 2026-11-05-fase-6-8-sincronizacao-e-afinacao.md (seção 10).

const Int = z.number().int();

// --- letra (ctx-sync)
export const LyricSpan = z.object({ start: Int, end: Int });
export const Syllable = z.object({ text: z.string(), span: LyricSpan.nullable() });
export const LyricWord = z.object({ syllables: z.array(Syllable) });
export const LyricLine = z.object({ words: z.array(LyricWord) });
export const LyricTrack = z.object({ id: Id, lines: z.array(LyricLine) });
export const Granularity = z.enum(["syllable", "word", "line"]);
export const SyllableRef = z.object({ line: Int, word: Int, syllable: Int });
export const LyricUnit = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("syllable"), line: Int, word: Int, syllable: Int }),
  z.object({ kind: z.literal("word"), line: Int, word: Int }),
  z.object({ kind: z.literal("line"), line: Int }),
]);
export const LyricsFileFormat = z.discriminatedUnion("format", [
  z.object({ format: z.literal("lrc"), granularity: Granularity }),
  z.object({ format: z.literal("srt") }),
]);

// --- partituras (ctx-notation)
export const ScoreSummary = z.object({ id: Id, title: z.string(), composer: z.string(), parts: z.array(z.string()), measures: Int });
export const NoteValue = z.enum(["whole", "half", "quarter", "eighth", "sixteenth", "thirtySecond", "sixtyFourth", "oneTwentyEighth"]);
export const Tuplet = z.object({ actual: Int, normal: Int });
export const NoteDuration = z.object({ value: NoteValue, dots: Int, tuplet: Tuplet.nullable() });
export const Step = z.enum(["C", "D", "E", "F", "G", "A", "B"]);
export const Pitch = z.object({ step: Step, alter: Int, octave: Int });
export const KeyMode = z.enum(["major", "minor"]);
export const KeySignature = z.object({ fifths: Int, mode: KeyMode });
export const ScoreMeter = z.object({ numerator: Int, denominator: Int });
export const TabPosition = z.object({ string: Int, fret: Int });
export const Technique = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("hammerOn") }),
  z.object({ kind: z.literal("pullOff") }),
  z.object({ kind: z.literal("slide") }),
  z.object({ kind: z.literal("bend"), cents: Int }),
  z.object({ kind: z.literal("vibrato") }),
  z.object({ kind: z.literal("palmMute") }),
  z.object({ kind: z.literal("harmonic"), harmonic: z.enum(["natural", "artificial"]) }),
  z.object({ kind: z.literal("letRing") }),
]);
export const ScoreNote = z.object({ pitch: Pitch, tie: z.boolean(), tab: TabPosition.nullable(), techniques: z.array(Technique) });
export const EventKind = z.discriminatedUnion("type", [z.object({ type: z.literal("rest") }), z.object({ type: z.literal("chord"), notes: z.array(ScoreNote) })]);
export const Articulation = z.enum(["staccato", "accent", "tenuto", "marcato", "fermata"]);
export const Dynamic = z.enum(["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"]);
export const GraceKind = z.enum(["acciaccatura", "appoggiatura"]);
export const ScoreLyric = z.object({ text: z.string(), syllabic: z.enum(["single", "begin", "middle", "end"]) });
export const ScoreEvent = z.object({
  id: Int,
  duration: NoteDuration,
  kind: EventKind,
  articulations: z.array(Articulation),
  dynamic: Dynamic.nullable(),
  grace: GraceKind.nullable(),
  lyric: ScoreLyric.nullable(),
});
export const ScoreVoice = z.object({ staff: Int, events: z.array(ScoreEvent) });
export const ScoreMeasure = z.object({ key: KeySignature, meter: ScoreMeter.nullable(), repeatStart: z.boolean(), repeatEnd: Int.nullable(), voices: z.array(ScoreVoice) });
export const Clef = z.enum(["treble", "treble8vb", "bass", "alto", "tenor", "percussion"]);
export const TabConfig = z.object({ tuning: z.array(Int), capo: Int, frets: Int });
export const Staff = z.object({ clef: Clef, standard: z.boolean(), tab: TabConfig.nullable() });
export const Spanner = z.object({ kind: z.enum(["slur", "crescendo", "diminuendo"]), start: Int, end: Int });
export const ScorePart = z.object({ name: z.string(), program: Int, percussion: z.boolean(), staves: z.array(Staff), measures: z.array(ScoreMeasure), spanners: z.array(Spanner) });
export const Score = z.object({ title: z.string(), composer: z.string(), tempo: TempoMap, parts: z.array(ScorePart), nextEventId: Int });
export const VoiceLoc = z.object({ part: Int, measure: Int, voice: Int });

/** Edições dentro de uma voz (`score_edit_voice`). */
export const VoiceEdit = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("insert"), index: Int, event: ScoreEvent }),
  z.object({ kind: z.literal("delete"), index: Int }),
  z.object({ kind: z.literal("replace"), index: Int, event: ScoreEvent }),
  z.object({ kind: z.literal("setDuration"), index: Int, duration: NoteDuration }),
  z.object({ kind: z.literal("setPitch"), index: Int, note: Int, pitch: Pitch }),
  z.object({ kind: z.literal("addNote"), index: Int, note: ScoreNote }),
  z.object({ kind: z.literal("removeNote"), index: Int, note: Int }),
  z.object({ kind: z.literal("addArticulation"), index: Int, articulation: Articulation }),
  z.object({ kind: z.literal("removeArticulation"), index: Int, articulation: Articulation }),
  z.object({ kind: z.literal("setDynamic"), index: Int, dynamic: Dynamic.nullable() }),
  z.object({ kind: z.literal("setTie"), index: Int, note: Int, tie: z.boolean() }),
  z.object({ kind: z.literal("setTab"), index: Int, note: Int, tab: TabPosition.nullable() }),
  z.object({ kind: z.literal("addTechnique"), index: Int, note: Int, technique: Technique }),
  z.object({ kind: z.literal("removeTechnique"), index: Int, note: Int, technique: Technique }),
  z.object({ kind: z.literal("setLyric"), index: Int, lyric: ScoreLyric.nullable() }),
  z.object({ kind: z.literal("setGrace"), index: Int, grace: GraceKind.nullable() }),
  z.object({ kind: z.literal("fillRests") }),
]);

export const TransposeScope = z.discriminatedUnion("scope", [
  z.object({ scope: z.literal("score") }),
  z.object({ scope: z.literal("part"), target: Int }),
  z.object({ scope: z.literal("events"), target: z.array(Int) }),
]);

/** Edições estruturais (`score_edit`). */
export const ScoreEdit = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("addMeasure"), index: Int }),
  z.object({ kind: z.literal("removeMeasure"), index: Int }),
  z.object({ kind: z.literal("setKey"), measure: Int, key: KeySignature }),
  z.object({ kind: z.literal("setMeter"), measure: Int, meter: ScoreMeter.nullable() }),
  z.object({ kind: z.literal("setRepeat"), measure: Int, start: z.boolean(), end: Int.nullable() }),
  z.object({ kind: z.literal("addPart"), index: Int, part: ScorePart }),
  z.object({ kind: z.literal("removePart"), index: Int }),
  z.object({ kind: z.literal("addSpanner"), part: Int, spanner: Spanner }),
  z.object({ kind: z.literal("removeSpanner"), part: Int, index: Int }),
  z.object({ kind: z.literal("transpose"), scope: TransposeScope, semitones: Int }),
  z.object({ kind: z.literal("setHeader"), title: z.string(), composer: z.string() }),
  z.object({ kind: z.literal("setTempo"), tick: Int, bpm: z.number() }),
]);

export const InstrumentKind = z.enum(["piano", "lead", "guitar", "bass", "drums"]);
export const ScoreTemplate = z.object({
  title: z.string(),
  composer: z.string().optional(),
  bpm: z.number(),
  meterNumerator: Int,
  meterDenominator: Int,
  keyFifths: Int.optional(),
  keyMode: KeyMode.optional(),
  measures: Int,
  instruments: z.array(z.object({ kind: InstrumentKind, name: z.string().nullable().optional() })),
});
export const ScoreCreated = z.object({ scoreId: Id, view: SessionView });
export const ScoreImportResult = z.object({ scoreId: Id, warnings: z.array(z.string()), view: SessionView });
export const ScoreExportFormat = z.enum(["musicXml", "mxl", "midi"]);
const Frac = z.object({ num: Int, den: Int });
export const ScoreReport = z.object({
  measureIssues: z.array(
    z.object({
      loc: VoiceLoc,
      issue: z.discriminatedUnion("kind", [z.object({ kind: z.literal("underfilled"), missing: Frac }), z.object({ kind: z.literal("overfilled"), excess: Frac })]),
    }),
  ),
  outOfScale: z.array(z.object({ event: z.object({ voice: VoiceLoc, index: Int }), noteIndex: Int, pitch: Pitch })),
});

// --- afinação (ctx-tuning)
export const ScaleCorrection = z.object({ enabled: z.boolean(), key: Int, allowed: z.array(Int), retuneSpeedMs: z.number(), humanize: z.number() });
export const TuningView = z.object({
  a4Hz: z.number(),
  maps: z.array(z.object({ mapId: Id, sourceId: Id, noteCount: Int, editCount: Int, scale: ScaleCorrection })),
});
export const NoteEdit = z.object({ target: Int.nullable(), centsOffset: z.number(), flatten: z.number(), formantSemitones: z.number() });
export const DetectedNote = z.object({ id: Id, startFrame: Int, endFrame: Int, midi: z.number() });
export const PitchCorrectionMap = z.object({ id: Id, source: Id, notes: z.array(DetectedNote), edits: z.record(z.string(), NoteEdit), scale: ScaleCorrection });
export const TunerReading = z.object({ note: Int, name: z.string(), octave: Int, cents: z.number(), frequencyHz: z.number(), confidence: z.number() });
export const PitchCorrectorParams = z.object({ enabled: z.boolean(), keyRoot: Int, scaleMask: Int, retuneMs: z.number(), humanize: z.number(), a4Hz: z.number() });
