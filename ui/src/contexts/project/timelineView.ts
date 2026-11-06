// Estado de visualização da timeline (zoom), compartilhado entre a timeline e o menu Exibir.
import { create } from "zustand";

export const ZOOM_MIN = 4;
export const ZOOM_MAX = 400;
export const ZOOM_DEFAULT = 40;
const STEP = 1.5;

interface TimelineViewState {
  /** Pixels por tempo (semínima). */
  pxPerBeat: number;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
}

export const useTimelineView = create<TimelineViewState>((set) => ({
  pxPerBeat: ZOOM_DEFAULT,
  zoomIn: () => set((s) => ({ pxPerBeat: Math.min(ZOOM_MAX, s.pxPerBeat * STEP) })),
  zoomOut: () => set((s) => ({ pxPerBeat: Math.max(ZOOM_MIN, s.pxPerBeat / STEP) })),
  resetZoom: () => set({ pxPerBeat: ZOOM_DEFAULT }),
}));
