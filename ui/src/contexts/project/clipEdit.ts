// Regras de apresentação da edição de clipes (Fase 3): conversões e limites calculados na UI para
// dar resposta imediata durante o arraste. O núcleo revalida tudo (fonte da verdade).
import type { Clip, FadeShape, Track } from "../../ipc/types";

/** Menor duração que a UI deixa um clipe ter ao cortar (amostras). */
export const MIN_CLIP_FRAMES = 64;

/** Amostras por tick ao redor do clipe (o mapa de tempo é aproximado como constante no clipe). */
export function framesPerTick(c: Pick<Clip, "lengthFrames" | "start" | "endTick">): number {
  const ticks = c.endTick - c.start;
  return ticks > 0 ? c.lengthFrames / ticks : 25;
}

/** Maior duração possível a partir do início atual na timeline (direita), dada a fonte. */
function maxLengthKeepingStart(c: Clip): number {
  // normal: a região cresce para depois do offset; reverso: cresce para antes dele
  return c.reversed ? c.offsetFrames + c.lengthFrames : c.sourceFrames - c.offsetFrames;
}

/** Arrastar a borda esquerda até `tick`: mantém o fim, muda início e duração. */
export function trimStartTo(c: Clip, tick: number): { start: number; lengthFrames: number } {
  const fpt = framesPerTick(c);
  // quanto dá para revelar à esquerda: normal = offset; reverso = o que sobra depois da região
  const reveal = c.reversed ? c.sourceFrames - c.offsetFrames - c.lengthFrames : c.offsetFrames;
  const minTick = Math.max(0, c.start - Math.floor(reveal / fpt));
  const maxTick = c.start + Math.floor((c.lengthFrames - MIN_CLIP_FRAMES) / fpt);
  const t = Math.round(Math.min(Math.max(tick, minTick), Math.max(minTick, maxTick)));
  const lengthFrames = c.lengthFrames - Math.round((t - c.start) * fpt);
  return { start: t, lengthFrames };
}

/** Arrastar a borda direita até `tick`: mantém o início, muda a duração. */
export function trimEndTo(c: Clip, tick: number): { start: number; lengthFrames: number } {
  const fpt = framesPerTick(c);
  const wanted = Math.round((tick - c.start) * fpt);
  const lengthFrames = Math.min(Math.max(MIN_CLIP_FRAMES, wanted), maxLengthKeepingStart(c));
  return { start: c.start, lengthFrames };
}

/** Fade a partir da posição do ponteiro (px a partir da borda correspondente), limitado à duração. */
export function fadeFromPx(c: Clip, kind: "in" | "out", px: number, widthPx: number): number {
  const frames = Math.round((Math.max(0, px) / Math.max(1, widthPx)) * c.lengthFrames);
  const other = kind === "in" ? c.fadeOutFrames : c.fadeInFrames;
  return Math.min(frames, c.lengthFrames - other);
}

/** Ganho de amplitude da curva para t ∈ [0, 1] (espelha shared_kernel::FadeShape::gain). */
export function fadeGain(shape: FadeShape, t: number): number {
  const x = Math.min(1, Math.max(0, Number.isNaN(t) ? 0 : t));
  return shape === "equalPower" ? Math.sin((x * Math.PI) / 2) : x;
}

/** Envelope (clipe × fades) da amostra relativa `rel`, como a engine aplica. */
export function envelopeAt(c: Clip, rel: number): number {
  let g = 1;
  if (rel < c.fadeInFrames) g *= fadeGain(c.fadeInShape, rel / c.fadeInFrames);
  const remaining = c.lengthFrames - rel;
  if (c.fadeOutFrames > 0 && remaining <= c.fadeOutFrames) g *= fadeGain(c.fadeOutShape, remaining / c.fadeOutFrames);
  return g;
}

/** Caminho SVG da curva de um fade (área acima da curva = atenuação), em px. */
export function fadePath(shape: FadeShape, kind: "in" | "out", x0: number, x1: number, h: number, steps = 24): string {
  const pts: string[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const g = fadeGain(shape, kind === "in" ? t : 1 - t);
    pts.push(`${(x0 + (x1 - x0) * t).toFixed(1)},${(h - g * h).toFixed(1)}`);
  }
  return `M${x0.toFixed(1)},0 L${pts.join(" L")} L${x1.toFixed(1)},0 Z`;
}

/** Clipe da mesma faixa que se sobrepõe parcialmente ao clipe dado (candidato a crossfade). */
export function overlappingNeighbor(track: Track, clipId: string): Clip | null {
  const c = track.clips.find((x) => x.id === clipId);
  if (!c) return null;
  const cs = c.startFrame;
  const ce = c.startFrame + c.lengthFrames;
  for (const o of track.clips) {
    if (o.id === c.id) continue;
    const os = o.startFrame;
    const oe = o.startFrame + o.lengthFrames;
    const [a0, a1, b0, b1] = cs < os ? [cs, ce, os, oe] : [os, oe, cs, ce];
    if (a0 < b0 && b0 < a1 && b1 >= a1) return o;
  }
  return null;
}

/** O tick está estritamente dentro do clipe (ponto válido para dividir)? */
export function canSplitAt(c: Pick<Clip, "start" | "endTick">, tick: number): boolean {
  return tick > c.start && tick < c.endTick;
}

export const framesToMs = (frames: number, sampleRate: number) => (frames * 1000) / sampleRate;
export const msToFrames = (ms: number, sampleRate: number) => Math.max(0, Math.round((ms * sampleRate) / 1000));
