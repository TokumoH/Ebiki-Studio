// API tipada: uma função por comando do núcleo. Respostas são validadas pelo contrato (zod).
import { z } from "zod";
import * as S from "./schema";
import { transport } from "./transport";
import type { ExportSettings, StripEdit, StripTarget, InputAssignment, MonitorMode, Metronome } from "./types";

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
