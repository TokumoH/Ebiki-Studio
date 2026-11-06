import { memo, useEffect, useRef } from "react";
import type { Clip } from "../../ipc/types";
import type { Spectrogram } from "../../ipc/spectrogram";
import { peaksFor } from "./waveformCache";
import { spectrogramFor, spectrumColor } from "./spectrogramCache";
import { envelopeAt, fadePath } from "./clipEdit";

const MAX_CANVAS = 4096;
const CANVAS_H = 64;

/** Bordas e alças que o usuário arrasta (ou ajusta pelo teclado) num clipe. */
export type ClipHandle = "trimStart" | "trimEnd" | "fadeIn" | "fadeOut";

/** Amostra da fonte correspondente à amostra `rel` do clipe (o reverso lê de trás para frente). */
function sourceFrame(clip: Clip, rel: number): number {
  return clip.reversed ? clip.offsetFrames + clip.lengthFrames - 1 - rel : clip.offsetFrames + rel;
}

function drawWaveform(canvas: HTMLCanvasElement, peaks: Float32Array, clip: Clip, fpp: number) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = getComputedStyle(canvas).color || "#9fd0f5";
  const mid = h / 2;
  const gain = Math.pow(10, clip.gain / 20);
  const sign = clip.polarityInverted ? -1 : 1;
  for (let x = 0; x < w; x++) {
    const r0 = Math.floor((x / w) * clip.lengthFrames);
    const r1 = Math.max(r0 + 1, Math.floor(((x + 1) / w) * clip.lengthFrames));
    const a = sourceFrame(clip, r0);
    const b = sourceFrame(clip, r1 - 1);
    const b0 = Math.floor(Math.min(a, b) / fpp);
    const b1 = Math.floor(Math.max(a, b) / fpp) + 1;
    let mn = 0;
    let mx = 0;
    for (let k = b0; k < b1 && k * 2 + 1 < peaks.length; k++) {
      mn = Math.min(mn, peaks[k * 2]!);
      mx = Math.max(mx, peaks[k * 2 + 1]!);
    }
    const g = gain * envelopeAt(clip, (r0 + r1) / 2);
    let top = Math.min(1, mx * g);
    let bottom = Math.max(-1, mn * g);
    if (sign < 0) [top, bottom] = [-bottom, -top];
    ctx.fillRect(x, mid - top * mid, 1, Math.max(1, (top - bottom) * mid));
  }
}

function drawSpectrogram(canvas: HTMLCanvasElement, s: Spectrogram, clip: Clip) {
  const ctx = canvas.getContext("2d");
  if (!ctx || s.columns === 0) return;
  const w = canvas.width;
  const h = canvas.height;
  const img = ctx.createImageData(w, h);
  for (let x = 0; x < w; x++) {
    const rel = Math.floor(((x + 0.5) / w) * clip.lengthFrames);
    const col = Math.min(s.columns - 1, Math.floor(sourceFrame(clip, rel) / s.hop));
    const env = envelopeAt(clip, rel);
    for (let y = 0; y < h; y++) {
      const bin = Math.min(s.bins - 1, Math.floor((1 - (y + 0.5) / h) * s.bins));
      const [r, g, b] = spectrumColor(s.data[col * s.bins + bin]! * env);
      const i = (y * w + x) * 4;
      img.data[i] = r;
      img.data[i + 1] = g;
      img.data[i + 2] = b;
      img.data[i + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
}

export interface ClipViewProps {
  clip: Clip;
  left: number;
  width: number;
  selected: boolean;
  muted: boolean;
  /** Mostra o espectrograma em vez da forma de onda. */
  spectrum: boolean;
  sampleRate: number;
  onPointerDown: (e: React.PointerEvent) => void;
  onHandlePointerDown: (handle: ClipHandle, e: React.PointerEvent) => void;
  /** Ajuste pelo teclado: `dir` = −1/+1; `big` = passo maior (Shift). */
  onHandleKey: (handle: ClipHandle, dir: -1 | 1, big: boolean) => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

const HANDLE_LABELS: Record<ClipHandle, string> = {
  trimStart: "Cortar início",
  trimEnd: "Cortar fim",
  fadeIn: "Fade de entrada",
  fadeOut: "Fade de saída",
};

/** Clipe com forma de onda/espectrograma, curvas de fade e alças de corte e fade. */
export const ClipView = memo(function ClipView(p: ClipViewProps) {
  const { clip, left, width, selected, muted, spectrum, sampleRate } = p;
  const ref = useRef<HTMLCanvasElement>(null);
  const fpp = Math.max(64, 2 ** Math.round(Math.log2(Math.max(1, clip.lengthFrames / Math.max(1, width)))));
  useEffect(() => {
    let alive = true;
    const size = (c: HTMLCanvasElement) => {
      c.width = Math.max(1, Math.min(MAX_CANVAS, Math.round(width)));
      c.height = CANVAS_H;
    };
    const job = spectrum
      ? spectrogramFor(clip.sourceId).then((s) => {
          const c = ref.current;
          if (!alive || !c) return;
          size(c);
          drawSpectrogram(c, s, clip);
        })
      : peaksFor(clip.sourceId, fpp).then((pk) => {
          const c = ref.current;
          if (!alive || !c) return;
          size(c);
          drawWaveform(c, pk, clip, fpp);
        });
    job.catch(() => {
      /* fonte indisponível: o clipe aparece sem desenho */
    });
    return () => {
      alive = false;
    };
  }, [clip, fpp, width, spectrum]);

  const w = Math.max(4, width);
  const pxPerFrame = w / Math.max(1, clip.lengthFrames);
  const fadeInPx = clip.fadeInFrames * pxPerFrame;
  const fadeOutPx = clip.fadeOutFrames * pxPerFrame;
  const ms = (f: number) => Math.round((f * 1000) / sampleRate);
  const handle = (h: ClipHandle, className: string, style: React.CSSProperties, valueMs: number | null) => (
    <div
      className={`clip-handle ${className}`}
      style={style}
      role="slider"
      aria-label={`${HANDLE_LABELS[h]} — ${clip.sourceName}`}
      aria-valuenow={valueMs ?? undefined}
      aria-valuetext={valueMs === null ? undefined : `${valueMs} ms`}
      tabIndex={selected ? 0 : -1}
      title={HANDLE_LABELS[h]}
      data-handle={h}
      onPointerDown={(e) => {
        e.stopPropagation();
        p.onHandlePointerDown(h, e);
      }}
      onKeyDown={(e) => {
        if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
        e.preventDefault();
        e.stopPropagation();
        p.onHandleKey(h, e.key === "ArrowLeft" ? -1 : 1, e.shiftKey);
      }}
    />
  );
  const flags = [clip.reversed ? "reverso" : "", clip.polarityInverted ? "fase invertida" : ""].filter(Boolean);
  return (
    <div
      className={`clip${muted ? " clip--muted" : ""}${spectrum ? " clip--spectrum" : ""}`}
      style={{ left, width: w }}
      aria-selected={selected}
      role="button"
      aria-label={`Clipe ${clip.sourceName}`}
      data-clip-id={clip.id}
      tabIndex={-1}
      onPointerDown={p.onPointerDown}
      onContextMenu={p.onContextMenu}
      title={`${clip.sourceName}${clip.gain !== 0 ? ` (${clip.gain.toFixed(1)} dB)` : ""}${flags.length ? ` — ${flags.join(", ")}` : ""}`}
    >
      <span className="clip-label">
        {clip.sourceName}
        {clip.reversed ? <span className="clip-flag" title="Reverso"> ⇆</span> : null}
        {clip.polarityInverted ? <span className="clip-flag" title="Fase invertida"> Ø</span> : null}
      </span>
      <canvas ref={ref} />
      <svg className="clip-fades" width={w} height="100%" viewBox={`0 0 ${w} 100`} preserveAspectRatio="none" aria-hidden="true">
        {fadeInPx > 0 ? <path className="clip-fade" d={fadePath(clip.fadeInShape, "in", 0, fadeInPx, 100)} /> : null}
        {fadeOutPx > 0 ? <path className="clip-fade" d={fadePath(clip.fadeOutShape, "out", w - fadeOutPx, w, 100)} /> : null}
      </svg>
      {handle("trimStart", "clip-handle--trim clip-handle--start", {}, null)}
      {handle("trimEnd", "clip-handle--trim clip-handle--end", {}, null)}
      {handle("fadeIn", "clip-handle--fade clip-handle--fade-in", { left: Math.min(w - 8, fadeInPx) }, ms(clip.fadeInFrames))}
      {handle("fadeOut", "clip-handle--fade clip-handle--fade-out", { left: Math.max(0, w - fadeOutPx - 8) }, ms(clip.fadeOutFrames))}
    </div>
  );
});
