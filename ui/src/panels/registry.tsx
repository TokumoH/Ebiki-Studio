// Composição: cada painel encaixável junta componentes de um ou mais contextos.
// Novos painéis entram aqui (único ponto de registro).
import type { ComponentType } from "react";
import { Timeline } from "../contexts/project/Timeline";
import { TempoPanel } from "../contexts/project/TempoPanel";
import { MixerPanel } from "../contexts/mixing/MixerPanel";
import { AdvisorPanel } from "../contexts/mixing/AdvisorPanel";
import { RecordingPanel } from "../contexts/recording/RecordingPanel";
import { ScorePanel } from "../contexts/notation/ScorePanel";
import { LyricsPanel } from "../contexts/sync/LyricsPanel";
import { TuningPanel } from "../contexts/tuning/TuningPanel";

export type PanelArea = "main" | "bottom" | "side";

export interface PanelDef {
  id: string;
  title: string;
  area: PanelArea;
  component: ComponentType;
}

export const PANELS: PanelDef[] = [
  { id: "timeline", title: "Arranjo", area: "main", component: Timeline },
  { id: "mixer", title: "Mixer", area: "bottom", component: MixerPanel },
  { id: "recording", title: "Gravação", area: "bottom", component: RecordingPanel },
  { id: "tempo", title: "Mapa de tempo", area: "bottom", component: TempoPanel },
  { id: "advisor", title: "Análise da mixagem", area: "bottom", component: AdvisorPanel },
  // Fases 6–8 (música)
  { id: "score", title: "Partitura", area: "bottom", component: ScorePanel },
  { id: "lyrics", title: "Letra", area: "bottom", component: LyricsPanel },
  { id: "tuning", title: "Afinação", area: "bottom", component: TuningPanel },
];
