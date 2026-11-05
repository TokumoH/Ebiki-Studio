// Composição: cada painel encaixável junta componentes de um ou mais contextos.
// Novos painéis entram aqui (único ponto de registro).
import type { ComponentType } from "react";
import { Timeline } from "../contexts/project/Timeline";
import { TempoPanel } from "../contexts/project/TempoPanel";
import { MixerPanel } from "../contexts/mixing/MixerPanel";
import { AdvisorPanel } from "../contexts/mixing/AdvisorPanel";
import { RecordingPanel } from "../contexts/recording/RecordingPanel";

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
];

// ---- Harmonia e bases (Fases 4–5) ----
// (imports no fim de propósito: seção só acrescentada, para facilitar o merge)
import { HarmonyPanel } from "../contexts/harmony/HarmonyPanel";
import { BasesPanel } from "../contexts/library/BasesPanel";

PANELS.push(
  { id: "harmony", title: "Harmonia", area: "bottom", component: HarmonyPanel },
  { id: "bases", title: "Bases", area: "bottom", component: BasesPanel },
);
