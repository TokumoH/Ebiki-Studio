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

// Música (Fases 6–8)
export type LyricTrack = z.infer<typeof S.LyricTrack>;
export type LyricLine = z.infer<typeof S.LyricLine>;
export type Syllable = z.infer<typeof S.Syllable>;
export type LyricSpan = z.infer<typeof S.LyricSpan>;
export type LyricUnit = z.infer<typeof S.LyricUnit>;
export type SyllableRef = z.infer<typeof S.SyllableRef>;
export type Granularity = z.infer<typeof S.Granularity>;
export type LyricsFileFormat = z.infer<typeof S.LyricsFileFormat>;
export type ScoreSummary = z.infer<typeof S.ScoreSummary>;
export type Score = z.infer<typeof S.Score>;
export type ScorePart = z.infer<typeof S.ScorePart>;
export type ScoreMeasure = z.infer<typeof S.ScoreMeasure>;
export type ScoreVoice = z.infer<typeof S.ScoreVoice>;
export type ScoreEvent = z.infer<typeof S.ScoreEvent>;
export type ScoreNote = z.infer<typeof S.ScoreNote>;
export type NoteDuration = z.infer<typeof S.NoteDuration>;
export type NoteValue = z.infer<typeof S.NoteValue>;
export type Pitch = z.infer<typeof S.Pitch>;
export type Step = z.infer<typeof S.Step>;
export type KeySignature = z.infer<typeof S.KeySignature>;
export type Articulation = z.infer<typeof S.Articulation>;
export type Dynamic = z.infer<typeof S.Dynamic>;
export type TabPosition = z.infer<typeof S.TabPosition>;
export type VoiceLoc = z.infer<typeof S.VoiceLoc>;
export type VoiceEdit = z.infer<typeof S.VoiceEdit>;
export type ScoreEdit = z.infer<typeof S.ScoreEdit>;
export type ScoreTemplate = z.infer<typeof S.ScoreTemplate>;
export type InstrumentKind = z.infer<typeof S.InstrumentKind>;
export type ScoreExportFormat = z.infer<typeof S.ScoreExportFormat>;
export type ScoreReport = z.infer<typeof S.ScoreReport>;
export type ScoreImportResult = z.infer<typeof S.ScoreImportResult>;
export type ScaleCorrection = z.infer<typeof S.ScaleCorrection>;
export type TuningView = z.infer<typeof S.TuningView>;
export type NoteEdit = z.infer<typeof S.NoteEdit>;
export type DetectedNote = z.infer<typeof S.DetectedNote>;
export type PitchCorrectionMap = z.infer<typeof S.PitchCorrectionMap>;
export type TunerReading = z.infer<typeof S.TunerReading>;
export type PitchCorrectorParams = z.infer<typeof S.PitchCorrectorParams>;
