// Cálculos de grade musical a partir do mapa de tempo (puros, testáveis).
import type { TempoMap } from "../../ipc/types";

export const PPQ = 960;

export interface GridLine {
  tick: number;
  bar: number;
  beat: number;
  isBar: boolean;
}

/** Linhas de compasso/tempo entre `from` e `to` (ticks), respeitando mudanças de fórmula. */
export function gridLines(tempo: TempoMap, from: number, to: number, maxLines = 2000): GridLine[] {
  const out: GridLine[] = [];
  const meters = tempo.meters.length ? tempo.meters : [{ tick: 0, numerator: 4, denominator: 4 }];
  let bar = 1;
  for (let i = 0; i < meters.length && out.length < maxLines; i++) {
    const m = meters[i]!;
    const segEnd = meters[i + 1]?.tick ?? Number.POSITIVE_INFINITY;
    const beatTicks = (PPQ * 4) / m.denominator;
    const barTicks = beatTicks * m.numerator;
    for (let t = m.tick; t < segEnd && t <= to && out.length < maxLines; t += barTicks, bar++) {
      for (let b = 0; b < m.numerator; b++) {
        const tick = t + b * beatTicks;
        if (tick >= segEnd) break;
        if (tick >= from && tick <= to) out.push({ tick, bar, beat: b + 1, isBar: b === 0 });
      }
    }
  }
  return out;
}

/** Arredonda para o tempo (beat) mais próximo. */
export function snapToBeat(tempo: TempoMap, tick: number): number {
  const lines = gridLines(tempo, Math.max(0, tick - PPQ * 8), tick + PPQ * 8);
  let best = Math.max(0, tick);
  let dist = Number.POSITIVE_INFINITY;
  for (const l of lines) {
    const d = Math.abs(l.tick - tick);
    if (d < dist) {
      dist = d;
      best = l.tick;
    }
  }
  return best;
}
