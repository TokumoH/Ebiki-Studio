// Cache de espectrogramas por fonte (o núcleo também guarda em disco; aqui evita refazer o IPC).
import { create } from "zustand";
import { editApi } from "../../ipc/api";
import type { Spectrogram } from "../../ipc/spectrogram";

const cache = new Map<string, Promise<Spectrogram>>();

export function spectrogramFor(sourceId: string): Promise<Spectrogram> {
  let p = cache.get(sourceId);
  if (!p) {
    p = editApi.sourceSpectrogram(sourceId).catch((e) => {
      cache.delete(sourceId);
      throw e;
    });
    cache.set(sourceId, p);
  }
  return p;
}

export function clearSpectrogramCache() {
  cache.clear();
}

/** Visualização por faixa: forma de onda (padrão) ou espectrograma. Estado só da UI. */
interface ClipViewModeState {
  spectrum: Record<string, boolean>;
  toggle: (trackId: string) => void;
}

export const useClipViewMode = create<ClipViewModeState>((set) => ({
  spectrum: {},
  toggle: (trackId) => set((s) => ({ spectrum: { ...s.spectrum, [trackId]: !s.spectrum[trackId] } })),
}));

/** Cor (mapa "magma" simplificado) para um valor 0..255 do espectrograma. */
export function spectrumColor(v: number): [number, number, number] {
  const t = Math.min(1, Math.max(0, v / 255));
  const stops: [number, number, number, number][] = [
    [0, 8, 6, 28],
    [0.35, 92, 22, 110],
    [0.6, 196, 52, 82],
    [0.8, 250, 140, 40],
    [1, 252, 250, 190],
  ];
  for (let i = 1; i < stops.length; i++) {
    const [t1, r1, g1, b1] = stops[i]!;
    const [t0, r0, g0, b0] = stops[i - 1]!;
    if (t <= t1) {
      const k = (t - t0) / (t1 - t0);
      return [Math.round(r0 + (r1 - r0) * k), Math.round(g0 + (g1 - g0) * k), Math.round(b0 + (b1 - b0) * k)];
    }
  }
  return [252, 250, 190];
}
