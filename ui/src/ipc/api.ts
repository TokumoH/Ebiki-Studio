// API tipada: uma função por comando do núcleo. Respostas são validadas pelo contrato (zod).
import { z } from "zod";
import * as S from "./schema";
import { transport } from "./transport";
import type { DeviceMonitor, ExportSettings, StripEdit, StripTarget, InputAssignment, MonitorMode, Metronome } from "./types";
import type { Granularity, LyricUnit, LyricsFileFormat, NoteEdit, ScaleCorrection, ScoreEdit, ScoreExportFormat, ScoreTemplate, SyllableRef, VoiceEdit, VoiceLoc } from "./types";

export class ApiFailure extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

function toFailure(e: unknown): ApiFailure {
  const p = S.ApiError.safeParse(e);
  if (p.success) return new ApiFailure(p.data.code, p.data.message);
  return new ApiFailure("ui.unexpected", e instanceof Error ? e.message : String(e));
}

async function call<T>(schema: z.ZodType<T>, cmd: string, args?: Record<string, unknown>): Promise<T> {
  let raw: unknown;
  try {
    raw = await transport().invoke(cmd, args);
  } catch (e) {
    throw toFailure(e);
  }
  const p = schema.safeParse(raw);
  if (!p.success) throw new ApiFailure("ui.contract", `Resposta inesperada do núcleo para "${cmd}": ${p.error.issues[0]?.message ?? ""}`);
  return p.data;
}

const Session = S.SessionView;
const Void = z.union([z.null(), z.undefined()]).transform(() => undefined);

export const api = {
  // áudio
  audioDevices: () => call(S.DeviceList, "audio_devices"),
  audioStatus: () => call(S.AudioStatus, "audio_status"),
  audioMonitorGet: () => call(S.DeviceMonitor.nullable(), "audio_monitor_get"),
  audioMonitorSet: (req: DeviceMonitor | null) => call(S.DeviceMonitor.nullable(), "audio_monitor_set", { req }),
  audioConfigure: (req: { outputId: string | null; inputId: string | null; bufferSize: number | null }) => call(S.AudioStatus, "audio_configure", { req }),
  // projeto
  projectCreate: (req: { dir: string; name: string; sampleRate: number; bitDepth: number }) => call(Session, "project_create", { req }),
  projectOpen: (path: string) => call(Session, "project_open", { path }),
  projectView: () => call(Session, "project_view"),
  projectSave: () => call(Session, "project_save"),
  projectClose: () => call(Void, "project_close"),
  projectRecent: () => call(S.RecentProject.array(), "project_recent"),
  projectForgetRecent: (path: string) => call(Void, "project_forget_recent", { path }),
  projectRecoveryCheck: (path: string) => call(z.boolean(), "project_recovery_check", { path }),
  projectRecover: (path: string) => call(Session, "project_recover", { path }),
  projectDiscardRecovery: (path: string) => call(Void, "project_discard_recovery", { path }),
  projectRename: (name: string) => call(Session, "project_rename", { name }),
  // histórico
  undo: () => call(Session, "history_undo"),
  redo: () => call(Session, "history_redo"),
  endGesture: () => call(Void, "history_end_gesture"),
  // faixas e clipes
  trackCreate: (name: string, index?: number) => call(Session, "track_create", { name, index: index ?? null }),
  trackDelete: (trackId: string) => call(Session, "track_delete", { trackId }),
  trackRename: (trackId: string, name: string) => call(Session, "track_rename", { trackId, name }),
  trackMove: (trackId: string, to: number) => call(Session, "track_move", { trackId, to }),
  audioImport: (paths: string[], trackId: string | null, atTick: number) => call(Session, "audio_import", { paths, trackId, atTick }),
  clipMove: (clipId: string, trackId: string, start: number, gesture?: number) => call(Session, "clip_move", { clipId, trackId, start, gesture: gesture ?? null }),
  clipDelete: (clipId: string) => call(Session, "clip_delete", { clipId }),
  clipSetGain: (clipId: string, gainDb: number, gesture?: number) => call(Session, "clip_set_gain", { clipId, gainDb, gesture: gesture ?? null }),
  // tempo
  tempoSet: (tick: number, bpm: number) => call(Session, "tempo_set", { tick, bpm }),
  tempoRemove: (tick: number) => call(Session, "tempo_remove", { tick }),
  meterSet: (tick: number, numerator: number, denominator: number) => call(Session, "meter_set", { tick, numerator, denominator }),
  meterRemove: (tick: number) => call(Session, "meter_remove", { tick }),
  // mixer
  mixerEdit: (target: StripTarget, edit: StripEdit, gesture?: number) => call(Session, "mixer_edit", { target, edit, gesture: gesture ?? null }),
  busCreate: (name: string) => call(Session, "bus_create", { name }),
  busDelete: (busId: string) => call(Session, "bus_delete", { busId }),
  busRename: (busId: string, name: string) => call(Session, "bus_rename", { busId, name }),
  mixerPresets: () => call(S.PresetInfo.array(), "mixer_presets"),
  mixerApplyPreset: (target: StripTarget, preset: string) => call(Session, "mixer_apply_preset", { target, preset }),
  mixAnalyze: (targetLufs: number) => call(S.MixReport, "mix_analyze", { targetLufs }),
  // gravação
  recordingTrack: (req: { trackId: string; armed?: boolean; input?: InputAssignment | null; monitor?: MonitorMode }) => call(Session, "recording_track", { req }),
  recordingMetronome: (config: Metronome) => call(Session, "recording_metronome", { config }),
  recordingPunch: (range: [number, number] | null) => call(Session, "recording_punch", { range }),
  recordingLoop: (range: [number, number] | null) => call(Session, "recording_loop", { range }),
  recordingLatency: (frames: number) => call(Session, "recording_latency", { frames }),
  takeSelect: (trackId: string, takeId: string) => call(Session, "take_select", { trackId, takeId }),
  // transporte
  play: () => call(Void, "transport_play"),
  pause: () => call(Void, "transport_pause"),
  stop: (returnToStart: boolean) => call(Session.nullable(), "transport_stop", { returnToStart }),
  seek: (tick: number) => call(Void, "transport_seek", { tick }),
  record: (atTick: number, countIn: boolean) => call(Void, "transport_record", { atTick, countIn }),
  transportState: () => call(S.TransportView, "transport_state"),
  waveform: async (sourceId: string, framesPerPeak: number): Promise<Float32Array> => {
    try {
      const buf = await transport().invokeBinary("waveform_peaks", { sourceId, framesPerPeak });
      return new Float32Array(buf);
    } catch (e) {
      throw toFailure(e);
    }
  },
  // exportação
  exportFormats: () => call(S.ExportFormat.array(), "export_formats"),
  exportMix: (path: string, settings: ExportSettings, range: [number, number] | null) => call(Void, "export_mix", { path, settings, range }),
  exportStems: (dir: string, settings: ExportSettings) => call(z.string().array(), "export_stems", { dir, settings }),
};

export type Api = typeof api;

// ---- Harmonia e bases (Fases 4–5) ----
// (import no fim do arquivo de propósito: seção só acrescentada, para facilitar o merge)
import type { BaseKey, BaseQuery, GenerateBaseRequest, HarmonyKey, InsertBaseRequest, NewUserBase, RealizeRequest, SaveGeneratedBaseRequest, UserProgressionInput } from "./types";

async function callBinary(cmd: string, args?: Record<string, unknown>): Promise<ArrayBuffer> {
  try {
    return await transport().invokeBinary(cmd, args);
  } catch (e) {
    throw toFailure(e);
  }
}

/** Comandos de composição (harmonia, bases, biblioteca e sintetizador). Áudio de audição = WAV. */
export const compositionApi = {
  // harmonia
  harmonyKeys: () => call(S.HarmonyKeysCatalog, "harmony_keys"),
  harmonyField: (key: HarmonyKey) => call(S.HarmonicField, "harmony_field", { key }),
  harmonyProgressions: (req: { key: HarmonyKey; genre?: string | null; limit?: number }) => call(S.ProgressionSuggestion.array(), "harmony_progressions", { req }),
  harmonyUserProgressions: () => call(S.ProgressionView.array(), "harmony_user_progressions"),
  harmonySuggestNext: (req: { key: HarmonyKey; history: string[]; limit?: number }) => call(S.ChordSuggestion.array(), "harmony_suggest_next", { req }),
  harmonyRealize: (req: RealizeRequest) => call(S.Realization, "harmony_realize", { req }),
  harmonyAudition: (req: RealizeRequest) => callBinary("harmony_audition", { req }),
  harmonyInsertProgression: (req: { realize: RealizeRequest; atTick: number; trackId?: string | null }) => call(S.InsertedClip, "harmony_insert_progression", { req }),
  harmonyUserProgressionAdd: (input: UserProgressionInput) => call(S.ProgressionView, "harmony_user_progression_add", { input }),
  harmonyUserProgressionEdit: (id: string, input: UserProgressionInput) => call(S.ProgressionView, "harmony_user_progression_edit", { id, input }),
  harmonyUserProgressionRemove: (id: string) => call(S.LibraryHistory, "harmony_user_progression_remove", { id }),
  // histórico da biblioteca (separado do histórico do projeto)
  libraryHistory: () => call(S.LibraryHistory, "library_history"),
  libraryUndo: () => call(S.LibraryHistory, "library_undo"),
  libraryRedo: () => call(S.LibraryHistory, "library_redo"),
  // bases
  basesOptions: () => call(S.BaseOptions, "bases_options"),
  basesSearch: (query: BaseQuery) => call(S.LibraryBase.array(), "bases_search", { query }),
  basesAddUser: (path: string, meta: NewUserBase) => call(S.LibraryBase, "bases_add_user", { path, meta }),
  basesGenerate: (req: GenerateBaseRequest) => call(S.GeneratedBase, "bases_generate", { req }),
  basesGeneratedAudition: (req: GenerateBaseRequest) => callBinary("bases_generated_audition", { req }),
  basesSaveGenerated: (req: SaveGeneratedBaseRequest) => call(S.LibraryBase, "bases_save_generated", { req }),
  basesRemove: (id: string) => call(S.LibraryHistory, "bases_remove", { id }),
  basesAudition: (id: string) => callBinary("bases_audition", { id }),
  basesMismatch: (req: { baseId: string; atTick: number; projectKey?: BaseKey | null }) => call(S.MismatchReport, "bases_mismatch", { req }),
  basesInsert: (req: InsertBaseRequest) => call(S.InsertBaseOutcome, "bases_insert", { req }),
  // sintetizador
  synthStatus: () => call(S.SynthStatus, "synth_status"),
  synthSetSoundfont: (path: string | null) => call(S.SynthStatus, "synth_set_soundfont", { path }),
};

export type CompositionApi = typeof compositionApi;

// ------------------------------------------------------------------------------------------------
// Música (Fases 6–8): partituras, letra, afinação, afinador e time-stretch (commands_music.rs).

async function binary(cmd: string, args?: Record<string, unknown>): Promise<ArrayBuffer> {
  try {
    return await transport().invokeBinary(cmd, args);
  } catch (e) {
    throw toFailure(e);
  }
}

const gest = (g?: number) => g ?? null;

export const musicApi = {
  // partituras
  scoreList: () => call(S.ScoreSummary.array(), "score_list"),
  scoreCreate: (req: ScoreTemplate) => call(S.ScoreCreated, "score_create", { req }),
  scoreCreateFromChords: (req: { title: string; meterNumerator: number; meterDenominator: number; keyFifths?: number; keyMode?: "major" | "minor"; slots: { startTick: number; durationTicks: number; notes: number[] }[] }) =>
    call(S.ScoreCreated, "score_create_from_chords", { req }),
  scoreGet: (scoreId: string) => call(S.Score, "score_get", { scoreId }),
  scoreMusicXml: (scoreId: string) => call(z.string(), "score_musicxml", { scoreId }),
  scoreDelete: (scoreId: string) => call(Session, "score_delete", { scoreId }),
  scoreEditVoice: (scoreId: string, loc: VoiceLoc, edit: VoiceEdit, gesture?: number) => call(Session, "score_edit_voice", { req: { scoreId, loc, edit, gesture: gest(gesture) } }),
  scoreEdit: (scoreId: string, edit: ScoreEdit, gesture?: number) => call(Session, "score_edit", { req: { scoreId, edit, gesture: gest(gesture) } }),
  scoreImport: (path: string) => call(S.ScoreImportResult, "score_import", { path }),
  /** Bytes lidos na UI (GP3–GP6 já convertidos para GP7 pelo alphaTab — ADR-0020). */
  scoreImportBytes: (name: string, bytes: Uint8Array) => call(S.ScoreImportResult, "score_import_bytes", { req: { name, bytes: Array.from(bytes) } }),
  scoreExport: (scoreId: string, path: string, format: ScoreExportFormat) => call(Void, "score_export", { req: { scoreId, path, format } }),
  /** WAV float 32 em memória (para `AudioContext.decodeAudioData`). */
  scoreAudition: (scoreId: string) => binary("score_audition", { scoreId }),
  scoreRenderToTrack: (scoreId: string, trackId: string | null, atTick = 0) => call(Session, "score_render_to_track", { req: { scoreId, trackId, atTick } }),
  scoreValidate: (scoreId: string, scale: number[] | null) => call(S.ScoreReport, "score_validate", { req: { scoreId, scale } }),
  // letra
  lyricsSetText: (text: string) => call(Session, "lyrics_set_text", { text }),
  lyricsSetSpan: (unit: LyricUnit, start: number, end: number, gesture?: number) => call(Session, "lyrics_set_span", { req: { unit, start, end, gesture: gest(gesture) } }),
  lyricsClearSpan: (unit: LyricUnit) => call(Session, "lyrics_clear_span", { unit }),
  lyricsShift: (from: SyllableRef, to: SyllableRef, delta: number, gesture?: number) => call(Session, "lyrics_shift", { req: { from, to, delta, gesture: gest(gesture) } }),
  lyricsApplyTapSync: (granularity: Granularity, from: number, taps: number[], end: number) => call(Session, "lyrics_apply_tap_sync", { req: { granularity, from, taps, end } }),
  lyricsExport: (path: string, format: LyricsFileFormat) => call(Void, "lyrics_export", { req: { path, format } }),
  lyricsImport: (path: string) => call(Session, "lyrics_import", { path }),
  // afinação
  tuningAnalyze: (sourceId: string) => call(Session, "tuning_analyze", { sourceId }),
  tuningGet: (sourceId: string) => call(S.PitchCorrectionMap, "tuning_get", { sourceId }),
  tuningEditNote: (sourceId: string, noteId: string, edit: NoteEdit, gesture?: number) => call(Session, "tuning_edit_note", { req: { sourceId, noteId, edit, gesture: gest(gesture) } }),
  tuningSetScale: (sourceId: string, scale: ScaleCorrection) => call(Session, "tuning_set_scale", { req: { sourceId, scale } }),
  tuningSetReference: (a4Hz: number) => call(Session, "tuning_set_reference", { a4Hz }),
  tuningApply: (sourceId: string, clipId: string) => call(Session, "tuning_apply", { req: { sourceId, clipId } }),
  tuningClear: (sourceId: string) => call(Session, "tuning_clear", { sourceId }),
  // afinador
  tunerStart: (inputChannel: number) => call(Void, "tuner_start", { inputChannel }),
  tunerRead: () => call(S.TunerReading.nullable(), "tuner_read"),
  tunerStop: () => call(Void, "tuner_stop"),
  // time-stretch
  clipTimeStretch: (clipId: string, sourceBpm: number, targetBpm: number | null) => call(Session, "clip_time_stretch", { req: { clipId, sourceBpm, targetBpm } }),
};

export type MusicApi = typeof musicApi;

// ------------------------------------------------------------------------------------------------
// Edição / pós-produção (Fase 3, commands_edit.rs). Seção só acrescentada, para facilitar o merge.
import type { FadeRequest, FadeShape, SpectrogramRequest } from "./types";
import { parseSpectrogram, type Spectrogram } from "./spectrogram";

export const editApi = {
  clipTrim: (clipId: string, start: number, lengthFrames: number, gesture?: number) => call(Session, "clip_trim", { clipId, start, lengthFrames, gesture: gest(gesture) }),
  clipSplit: (clipId: string, atTick: number) => call(Session, "clip_split", { clipId, atTick }),
  clipSetFades: (clipId: string, fades: FadeRequest, gesture?: number) => call(Session, "clip_set_fades", { clipId, fades, gesture: gest(gesture) }),
  clipSetPolarity: (clipId: string, inverted: boolean) => call(Session, "clip_set_polarity", { clipId, inverted }),
  clipSetReversed: (clipId: string, reversed: boolean) => call(Session, "clip_set_reversed", { clipId, reversed }),
  clipCrossfade: (aId: string, bId: string, shape: FadeShape) => call(Session, "clip_crossfade", { aId, bId, shape }),
  /** Ganho do clipe para levar o pico a `targetDbfs` (padrão −1 dBFS no núcleo). */
  clipNormalize: (clipId: string, targetDbfs?: number) => call(Session, "clip_normalize", { clipId, targetDbfs: targetDbfs ?? null }),
  sourceSpectrogram: async (sourceId: string, params?: SpectrogramRequest): Promise<Spectrogram> => parseSpectrogram(await binary("source_spectrogram", { sourceId, params: params ?? null })),
};

export type EditApi = typeof editApi;
