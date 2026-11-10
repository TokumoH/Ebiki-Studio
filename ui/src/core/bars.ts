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
  beatTicks: number;
}

function segments(tempo: TempoMap): Segment[] {
  const meters = [...(tempo.meters.length ? tempo.meters : [DEFAULT_METER])].sort((a, b) => a.tick - b.tick);
  if (meters[0]!.tick > 0) meters.unshift(DEFAULT_METER);
  return meters.map((m, i) => ({
    start: m.tick,
    end: meters[i + 1]?.tick ?? Number.POSITIVE_INFINITY,
    barTicks: ((PPQ * 4) / m.denominator) * m.numerator,
    beatTicks: (PPQ * 4) / m.denominator,
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

/** Tick de início do compasso `bar` (1 = primeiro), na mesma numeração da régua. */
export function barToTick(tempo: TempoMap, bar: number): number {
  let n = 1;
  const target = Math.max(1, Math.floor(bar));
  for (const seg of segments(tempo)) {
    const bars = Number.isFinite(seg.end) ? Math.ceil((seg.end - seg.start) / seg.barTicks) : Number.POSITIVE_INFINITY;
    if (target < n + bars) return seg.start + (target - n) * seg.barTicks;
    n += bars;
  }
  return 0;
}

/** Número (1 = primeiro) do compasso que contém `tick`, na mesma numeração da régua. */
export function tickToBar(tempo: TempoMap, tick: number): number {
  const t = Math.max(0, tick);
  let n = 1;
  for (const seg of segments(tempo)) {
    if (t < seg.end) return n + Math.floor((t - seg.start) / seg.barTicks);
    n += Math.ceil((seg.end - seg.start) / seg.barTicks);
  }
  return n;
}

/** Posição compasso.tempo.tique de `tick`, com o compasso na mesma numeração da régua. */
export function barBeatAt(tempo: TempoMap, tick: number): { bar: number; beat: number; tickInBeat: number } {
  const t = Math.max(0, tick);
  const seg = segmentAt(segments(tempo), t);
  const inBar = t - barStartAt(tempo, t);
  return { bar: tickToBar(tempo, t), beat: Math.floor(inBar / seg.beatTicks) + 1, tickInBeat: inBar % seg.beatTicks };
}

/** Loop do compasso `from` até o fim do compasso `to` (inclusivo; aceita a ordem invertida). */
export function loopRangeBars(tempo: TempoMap, from: number, to: number): [number, number] {
  const a = Math.max(1, Math.min(from, to));
  const b = Math.max(1, Math.max(from, to));
  return [barToTick(tempo, a), barToTick(tempo, b + 1)];
}

/** Loop do início até o fim do compasso onde termina o último clipe; `null` sem clipes. */
export function projectLoopRange(tempo: TempoMap, lastClipEnd: number): [number, number] | null {
  if (lastClipEnd <= 0) return null;
  const lastBar = tickToBar(tempo, lastClipEnd - 1);
  return [0, barToTick(tempo, lastBar + 1)];
}

/** "1–8" (primeiro e último compasso do loop) ou "3" para um compasso só. */
export function loopLabel(tempo: TempoMap, range: [number, number]): string {
  const a = tickToBar(tempo, range[0]);
  const b = tickToBar(tempo, Math.max(range[0], range[1] - 1));
  return a === b ? String(a) : `${a}–${b}`;
}
