// Estado de visualização da timeline (zoom no tempo e altura das faixas), compartilhado entre a
// timeline e o menu Exibir. Só interface: não vai para o projeto. Funções puras exportadas para teste.
import { create } from "zustand";

export const ZOOM_MIN = 4;
export const ZOOM_MAX = 400;
export const ZOOM_DEFAULT = 40;
const STEP = 1.5;

export const HEIGHT_MIN = 40;
export const HEIGHT_MAX = 320;
export const HEIGHT_DEFAULT = 72;

/** Fator de zoom de um giro da roda: para cima aproxima; suave para trackpad (deltas pequenos). */
export function wheelFactor(deltaY: number, deltaMode: number): number {
  // deltaMode 1 = linhas (~33 px), 2 = páginas
  const px = deltaMode === 1 ? deltaY * 33 : deltaMode === 2 ? deltaY * 400 : deltaY;
  return Math.exp(-px * 0.0015);
}

/** scrollLeft que mantém o tique `tickAtMouse` na mesma posição `mouseOffset` da tela. */
export function anchoredScrollLeft(tickAtMouse: number, pxPerTick: number, mouseOffset: number): number {
  return Math.max(0, tickAtMouse * pxPerTick - mouseOffset);
}

export const clampHeight = (h: number) => Math.round(Math.min(HEIGHT_MAX, Math.max(HEIGHT_MIN, h)));

/** Índice da faixa na posição vertical `y` (a partir do topo das faixas), com alturas variáveis. */
export function trackIndexAt(heights: readonly number[], y: number): number {
  if (heights.length === 0) return 0;
  let acc = 0;
  for (let i = 0; i < heights.length; i++) {
    acc += heights[i]!;
    if (y < acc) return i;
  }
  return heights.length - 1;
}

interface TimelineViewState {
  /** Pixels por tempo (semínima). */
  pxPerBeat: number;
  /** Altura das faixas sem altura própria (Alt+roda escala todas). */
  baseHeight: number;
  /** Altura própria por faixa (arrastar a borda do cabeçalho). */
  heights: Record<string, number>;
  zoomIn: () => void;
  zoomOut: () => void;
  zoomBy: (factor: number) => void;
  resetZoom: () => void;
  heightOf: (trackId: string) => number;
  setHeight: (trackId: string, h: number) => void;
  scaleHeights: (factor: number) => void;
}

export const useTimelineView = create<TimelineViewState>((set, get) => ({
  pxPerBeat: ZOOM_DEFAULT,
  baseHeight: HEIGHT_DEFAULT,
  heights: {},
  zoomIn: () => get().zoomBy(STEP),
  zoomOut: () => get().zoomBy(1 / STEP),
  zoomBy: (factor) => set((s) => ({ pxPerBeat: Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, s.pxPerBeat * factor)) })),
  resetZoom: () => set({ pxPerBeat: ZOOM_DEFAULT, baseHeight: HEIGHT_DEFAULT, heights: {} }),
  heightOf: (trackId) => get().heights[trackId] ?? get().baseHeight,
  setHeight: (trackId, h) => set((s) => ({ heights: { ...s.heights, [trackId]: clampHeight(h) } })),
  scaleHeights: (factor) =>
    set((s) => ({
      baseHeight: clampHeight(s.baseHeight * factor),
      heights: Object.fromEntries(Object.entries(s.heights).map(([k, v]) => [k, clampHeight(v * factor)])),
    })),
}));
