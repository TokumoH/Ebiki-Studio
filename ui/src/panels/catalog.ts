// Catálogo dos painéis encaixáveis (metadados sem React): fonte única de ids, títulos e áreas.
// Usado pelo registro (que associa o componente), pelo menu "Painéis" e pelos testes E2E.

export type PanelArea = "main" | "bottom" | "side";

export interface PanelMeta {
  id: string;
  title: string;
  area: PanelArea;
}

export const PANEL_CATALOG: readonly PanelMeta[] = [
  { id: "timeline", title: "Arranjo", area: "main" },
  { id: "mixer", title: "Mixer", area: "bottom" },
  { id: "recording", title: "Gravação", area: "bottom" },
  { id: "tempo", title: "Mapa de tempo", area: "bottom" },
  { id: "advisor", title: "Análise da mixagem", area: "bottom" },
  // Fases 6–8 (música)
  { id: "score", title: "Partitura", area: "bottom" },
  { id: "lyrics", title: "Letra", area: "bottom" },
  { id: "tuning", title: "Afinação", area: "bottom" },
  // Fases 4–5 (harmonia e bases)
  { id: "harmony", title: "Harmonia", area: "bottom" },
  { id: "bases", title: "Bases", area: "bottom" },
  // Fase 3 (edição / pós-produção)
  { id: "clip", title: "Clipe", area: "bottom" },
];
