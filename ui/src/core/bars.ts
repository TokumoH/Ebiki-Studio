// Aritmética de compassos sobre o mapa de tempo (pura, sem dependências de apresentação).
// Mesma convenção da grade: cada mudança de fórmula inicia um compasso novo no seu tick.
import type { TempoMap } from "../ipc/types";

/** Resolução do núcleo: ticks por semínima. */
export const PPQ = 960;

const DEFAULT_METER = { tick: 0, numerator: 4, denominator: 4 };

interface Segment {
  start: number;
  end: number;
  barTicks: number;
}

function segments(tempo: TempoMap): Segment[] {
  const meters = [...(tempo.meters.length ? tempo.meters : [DEFAULT_METER])].sort((a, b) => a.tick - b.tick);
  if (meters[0]!.tick > 0) meters.unshift(DEFAULT_METER);
  return meters.map((m, i) => ({
    start: m.tick,
    end: meters[i + 1]?.tick ?? Number.POSITIVE_INFINITY,
    barTicks: ((PPQ * 4) / m.denominator) * m.numerator,
  }));
}

function segmentAt(segs: Segment[], tick: number): Segment {
  return segs.find((s) => tick >= s.start && tick < s.end) ?? segs[segs.length - 1]!;
}

/** Início (tick) do compasso que contém `tick`. */
export function barStartAt(tempo: TempoMap, tick: number): number {
  const t = Math.max(0, tick);
  const seg = segmentAt(segments(tempo), t);
  return seg.start + Math.floor((t - seg.start) / seg.barTicks) * seg.barTicks;
}

/** Intervalo de `bars` compassos a partir do compasso que contém o cursor (respeita mudanças de fórmula). */
export function loopRangeFromCursor(tempo: TempoMap, cursorTick: number, bars = 4): [number, number] {
  const segs = segments(tempo);
  const start = barStartAt(tempo, cursorTick);
  let pos = start;
  for (let i = 0; i < bars; i++) {
    const seg = segmentAt(segs, pos);
    pos = Math.min(pos + seg.barTicks, seg.end);
  }
  return [start, pos];
}
