// Cache de picos por fonte (evita pedir ao núcleo a cada redesenho).
import { api } from "../../ipc/api";

const cache = new Map<string, Promise<Float32Array>>();

export function peaksFor(sourceId: string, framesPerPeak: number): Promise<Float32Array> {
  const key = `${sourceId}:${framesPerPeak}`;
  let p = cache.get(key);
  if (!p) {
    p = api.waveform(sourceId, framesPerPeak).catch((e) => {
      cache.delete(key);
      throw e;
    });
    cache.set(key, p);
  }
  return p;
}

export function clearWaveformCache() {
  cache.clear();
}
