// Composição: cada painel encaixável junta componentes de um ou mais contextos.
// Novo painel: metadados em `catalog.ts` + componente em `COMPONENTS` (o teste do registro cobra os dois).
import type { ComponentType } from "react";
import { Timeline } from "../contexts/project/Timeline";
import { TempoPanel } from "../contexts/project/TempoPanel";
import { MixerPanel } from "../contexts/mixing/MixerPanel";
import { AdvisorPanel } from "../contexts/mixing/AdvisorPanel";
import { RecordingPanel } from "../contexts/recording/RecordingPanel";
import { ScorePanel } from "../contexts/notation/ScorePanel";
import { LyricsPanel } from "../contexts/sync/LyricsPanel";
import { TuningPanel } from "../contexts/tuning/TuningPanel";
import { HarmonyPanel } from "../contexts/harmony/HarmonyPanel";
import { BasesPanel } from "../contexts/library/BasesPanel";
import { ClipInspector } from "../contexts/project/ClipInspector";
import { PANEL_CATALOG, type PanelMeta } from "./catalog";

export type { PanelArea } from "./catalog";

export interface PanelDef extends PanelMeta {
  component: ComponentType;
}

export const COMPONENTS: Record<string, ComponentType> = {
  timeline: Timeline,
  mixer: MixerPanel,
  recording: RecordingPanel,
  tempo: TempoPanel,
  advisor: AdvisorPanel,
  score: ScorePanel,
  lyrics: LyricsPanel,
  tuning: TuningPanel,
  harmony: HarmonyPanel,
  bases: BasesPanel,
  clip: ClipInspector,
};

export const PANELS: PanelDef[] = PANEL_CATALOG.flatMap((p) => {
  const component = COMPONENTS[p.id];
  return component ? [{ ...p, component }] : [];
});
