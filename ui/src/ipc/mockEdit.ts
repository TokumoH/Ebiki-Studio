// Transporte simulado — Fase 3 (edição): mesmas regras visíveis do núcleo (ctx-project/domain/edit.rs).
import type { Clip, FadeShape, SessionView, Track } from "./types";
import { encodeSpectrogram } from "./spectrogram";

export interface EditMockCtx {
  s: () => SessionView;
  mutate: (label: string, gesture: number | null, fn: (v: SessionView) => void) => SessionView;
  fail: (code: string, message: string) => never;
  sr: () => number;
  uuid: () => string;
  tickToFrame: (t: number) => number;
  frameToTick: (f: number) => number;
}

/** Campos da Fase 3 de um clipe novo (sem processamento). */
export function clipEditDefaults(sourceFrames: number): Pick<Clip, "sourceFrames" | "fadeInFrames" | "fadeOutFrames" | "fadeInShape" | "fadeOutShape" | "polarityInverted" | "reversed"> {
  return { sourceFrames, fadeInFrames: 0, fadeOutFrames: 0, fadeInShape: "linear", fadeOutShape: "linear", polarityInverted: false, reversed: false };
}

/** Reduz os fades para caber na duração (o de entrada tem prioridade), como `ClipEdit::clamped_to`. */
export function clampFades(c: Pick<Clip, "fadeInFrames" | "fadeOutFrames">, length: number) {
  const fadeInFrames = Math.min(c.fadeInFrames, length);
  return { fadeInFrames, fadeOutFrames: Math.min(c.fadeOutFrames, length - fadeInFrames) };
}

export function editHandlers(ctx: EditMockCtx) {
  const { mutate, fail, tickToFrame } = ctx;

  const find = (v: SessionView, id: string): { track: Track; clip: Clip; index: number } => {
    for (const track of v.tracks) {
      const index = track.clips.findIndex((c) => c.id === id);
      if (index >= 0) return { track, clip: track.clips[index]!, index };
    }
    return fail("project.clip_not_found", "clipe não encontrado");
  };
  const checkFades = (c: Clip) => {
    if (c.fadeInFrames < 0 || c.fadeOutFrames < 0 || c.fadeInFrames + c.fadeOutFrames > c.lengthFrames)
      fail("project.invalid_fade", "fades inválidos: fade de entrada + fade de saída não podem exceder a duração do clipe");
  };
  const startFrame = (c: Clip) => tickToFrame(c.start);

  const handlers: Record<string, (a: Record<string, any>) => unknown> = {
    clip_trim: (a) =>
      mutate("Cortar clipe", a.gesture ?? null, (v) => {
        const { clip: c } = find(v, a.clipId);
        const len = Math.round(a.lengthFrames);
        if (a.start < 0) fail("api.invalid_argument", "início negativo");
        if (len <= 0) fail("project.clip_out_of_bounds", "trecho do clipe fora dos limites da fonte de áudio");
        const shift = tickToFrame(a.start) - startFrame(c);
        const off = c.reversed ? c.offsetFrames + c.lengthFrames - (shift + len) : c.offsetFrames + shift;
        if (off < 0 || off + len > c.sourceFrames) fail("project.clip_out_of_bounds", "trecho do clipe fora dos limites da fonte de áudio");
        Object.assign(c, { start: a.start, offsetFrames: off, lengthFrames: len, ...clampFades(c, len) });
      }),
    clip_split: (a) =>
      mutate("Dividir clipe", null, (v) => {
        const { track, clip: c, index } = find(v, a.clipId);
        const f = tickToFrame(a.atTick) - startFrame(c);
        if (a.atTick < 0 || f <= 0 || f >= c.lengthFrames) fail("project.invalid_split", "ponto de divisão fora do clipe");
        const rest = c.lengthFrames - f;
        const second: Clip = {
          ...c,
          id: ctx.uuid(),
          start: a.atTick,
          offsetFrames: c.reversed ? c.offsetFrames : c.offsetFrames + f,
          lengthFrames: rest,
          fadeInFrames: 0,
          fadeOutFrames: Math.min(c.fadeOutFrames, rest),
        };
        Object.assign(c, { offsetFrames: c.reversed ? c.offsetFrames + rest : c.offsetFrames, lengthFrames: f, fadeInFrames: Math.min(c.fadeInFrames, f), fadeOutFrames: 0 });
        track.clips.splice(index + 1, 0, second);
      }),
    clip_set_fades: (a) =>
      mutate("Fades do clipe", a.gesture ?? null, (v) => {
        const { clip: c } = find(v, a.clipId);
        const next = { ...c, ...a.fades };
        checkFades(next);
        Object.assign(c, a.fades);
      }),
    clip_set_polarity: (a) => mutate("Inverter fase", null, (v) => (find(v, a.clipId).clip.polarityInverted = Boolean(a.inverted))),
    clip_set_reversed: (a) => mutate("Reverter clipe", null, (v) => (find(v, a.clipId).clip.reversed = Boolean(a.reversed))),
    clip_crossfade: (a) =>
      mutate("Crossfade", null, (v) => {
        const A = find(v, a.aId);
        const B = find(v, a.bId);
        if (A.track.id !== B.track.id || a.aId === a.bId) fail("project.clips_do_not_overlap", "os clipes precisam estar na mesma faixa e se sobrepor");
        const [x, y] = startFrame(A.clip) <= startFrame(B.clip) ? [A.clip, B.clip] : [B.clip, A.clip];
        const [xs, xe, ys, ye] = [startFrame(x), startFrame(x) + x.lengthFrames, startFrame(y), startFrame(y) + y.lengthFrames];
        if (!(xs < ys && ys < xe && ye >= xe)) fail("project.clips_do_not_overlap", "os clipes precisam estar na mesma faixa e se sobrepor");
        const overlap = xe - ys;
        const shape = a.shape as FadeShape;
        Object.assign(x, { fadeOutFrames: overlap, fadeOutShape: shape, fadeInFrames: Math.min(x.fadeInFrames, x.lengthFrames - overlap) });
        Object.assign(y, { fadeInFrames: overlap, fadeInShape: shape, fadeOutFrames: Math.min(y.fadeOutFrames, y.lengthFrames - overlap) });
      }),
    clip_normalize: (a) =>
      mutate("Ganho do clipe", null, (v) => {
        const target = a.targetDbfs ?? -1;
        if (!(target >= -60 && target <= 0)) fail("api.invalid_argument", "Alvo de normalização deve estar entre −60 e 0 dBFS.");
        // o áudio simulado tem pico 0,75 (ver waveform_peaks em mock.ts)
        find(v, a.clipId).clip.gain = Math.round((target - 20 * Math.log10(0.75)) * 100) / 100;
      }),
  };

  /** Espectrograma sintético: uma "melodia" que sobe e desce + ruído grave. */
  function binary(cmd: string, args: Record<string, unknown>): ArrayBuffer | null {
    if (cmd !== "source_spectrogram") return null;
    const src = ctx.s().sources.find((x) => x.id === args.sourceId) ?? fail("app.source_unavailable", "Áudio indisponível.");
    const p = (args.params as { fftSize: number; hop: number; bins: number } | null) ?? { fftSize: 2048, hop: 512, bins: 128 };
    const columns = Math.ceil(src.lengthFrames / p.hop);
    const data = new Uint8Array(columns * p.bins);
    for (let c = 0; c < columns; c++) {
      const line = Math.round(p.bins * (0.45 + 0.2 * Math.sin(c / 40)));
      for (let b = 0; b < p.bins; b++) {
        const d = Math.abs(b - line);
        const base = Math.max(0, 150 - b * 1.2);
        data[c * p.bins + b] = Math.min(255, Math.max(base, d < 3 ? 230 - d * 30 : 0));
      }
    }
    return encodeSpectrogram({ bins: p.bins, fftSize: p.fftSize, hop: p.hop, sampleRate: ctx.sr(), totalFrames: src.lengthFrames, data });
  }

  return { handlers, binary };
}
