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
  sampleRate: z.number().int(),
  bufferSize: z.number().int().nullable(),
  inputChannels: z.number().int(),
  estimatedLatencyMs: z.number().nullable(),
  error: z.string().nullable(),
});
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
