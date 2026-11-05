import type { z } from "zod";
import type * as S from "./schema";

export type SessionView = z.infer<typeof S.SessionView>;
export type Track = z.infer<typeof S.Track>;
export type Clip = z.infer<typeof S.Clip>;
export type BusView = z.infer<typeof S.BusView>;
export type ChannelStrip = z.infer<typeof S.ChannelStrip>;
export type EffectSlot = z.infer<typeof S.EffectSlot>;
export type Effect = z.infer<typeof S.Effect>;
export type EqBand = z.infer<typeof S.EqBand>;
export type Output = z.infer<typeof S.Output>;
export type AuxSend = z.infer<typeof S.AuxSend>;
export type TempoMap = z.infer<typeof S.TempoMap>;
export type TransportView = z.infer<typeof S.TransportView>;
export type MetersView = z.infer<typeof S.MetersView>;
export type StripTarget = z.infer<typeof S.StripTarget>;
export type StripEdit = z.infer<typeof S.StripEdit>;
export type ApiError = z.infer<typeof S.ApiError>;
export type RecentProject = z.infer<typeof S.RecentProject>;
export type DeviceList = z.infer<typeof S.DeviceList>;
export type AudioStatus = z.infer<typeof S.AudioStatus>;
export type ExportFormat = z.infer<typeof S.ExportFormat>;
export type ExportSettings = z.infer<typeof S.ExportSettings>;
export type MixReport = z.infer<typeof S.MixReport>;
export type Advice = z.infer<typeof S.Advice>;
export type PresetInfo = z.infer<typeof S.PresetInfo>;
export type InputAssignment = z.infer<typeof S.InputAssignment>;
export type MonitorMode = z.infer<typeof S.MonitorMode>;
export type TrackRecording = z.infer<typeof S.TrackRecording>;
export type Metronome = SessionView["recording"]["metronome"];

// ---- Harmonia e bases ----
export type HarmonyKey = z.infer<typeof S.HarmonyKey>;
export type BaseKey = z.infer<typeof S.BaseKey>;
export type BaseMeter = z.infer<typeof S.BaseMeter>;
export type HarmonyKeysCatalog = z.infer<typeof S.HarmonyKeysCatalog>;
export type HarmonicField = z.infer<typeof S.HarmonicField>;
export type ProgressionRef = z.infer<typeof S.ProgressionRef>;
export type ProgressionView = z.infer<typeof S.ProgressionView>;
export type ProgressionSuggestion = z.infer<typeof S.ProgressionSuggestion>;
export type ChordSuggestion = z.infer<typeof S.ChordSuggestion>;
export type CompositionNoteEvent = z.infer<typeof S.CompositionNoteEvent>;
export type Realization = z.infer<typeof S.Realization>;
export type InsertedClip = z.infer<typeof S.InsertedClip>;
export type LibraryHistory = z.infer<typeof S.LibraryHistory>;
export type BaseOptions = z.infer<typeof S.BaseOptions>;
export type BaseKind = z.infer<typeof S.BaseKindTag>;
export type LibraryBase = z.infer<typeof S.LibraryBase>;
export type GeneratedBase = z.infer<typeof S.GeneratedBase>;
export type MismatchReport = z.infer<typeof S.MismatchReport>;
export type BaseDecision = z.infer<typeof S.BaseDecision>;
export type InsertBaseOutcome = z.infer<typeof S.InsertBaseOutcome>;
export type SynthStatus = z.infer<typeof S.SynthStatus>;

/** Requisições (UI → núcleo); campos opcionais seguem os padrões do Rust. */
export interface VoicingRequest {
  low?: number;
  high?: number;
  voiceLeading?: boolean;
  bass?: boolean;
  program?: number;
  bassProgram?: number;
  velocity?: number;
}
export interface RealizeRequest {
  progression: ProgressionRef;
  key: HarmonyKey;
  bpm?: number | null;
  bars?: number | null;
  voicing?: VoicingRequest;
}
export interface UserProgressionInput {
  name: string;
  mode: string;
  genres?: string[];
  beatsPerBar?: number;
  steps: string;
}
export interface BaseQuery {
  text?: string;
  bpmMin?: number;
  bpmMax?: number;
  key?: BaseKey;
  genre?: string;
  instrument?: string;
  tags?: string[];
  kind?: BaseKind;
}
export interface NewUserBase {
  name: string;
  bpm: number;
  key?: BaseKey | null;
  meter?: BaseMeter | null;
  genre?: string | null;
  instruments?: string[];
  tags?: string[];
  progression?: string | null;
}
export interface GenerateBaseRequest {
  progression: ProgressionRef;
  key: HarmonyKey;
  bpm: number;
  bars: number;
  style: string;
  drums?: string | null;
  bass?: string | null;
}
export interface SaveGeneratedBaseRequest {
  name: string;
  generation: GenerateBaseRequest;
  genre?: string | null;
  tags?: string[];
}
export interface InsertBaseRequest {
  baseId: string;
  atTick: number;
  trackId?: string | null;
  projectKey?: BaseKey | null;
  decision?: BaseDecision | null;
}
