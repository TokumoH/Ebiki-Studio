import { memo, useEffect, useRef } from "react";
import type { Clip } from "../../ipc/types";
import { peaksFor } from "./waveformCache";

const MAX_CANVAS = 4096;

function draw(canvas: HTMLCanvasElement, peaks: Float32Array, clip: Clip, fpp: number) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = getComputedStyle(canvas).color || "#9fd0f5";
  const startBucket = Math.floor(clip.offsetFrames / fpp);
  const buckets = Math.max(1, Math.ceil(clip.lengthFrames / fpp));
  const mid = h / 2;
  for (let x = 0; x < w; x++) {
    const b0 = startBucket + Math.floor((x / w) * buckets);
    const b1 = Math.max(b0 + 1, startBucket + Math.floor(((x + 1) / w) * buckets));
    let mn = 0;
    let mx = 0;
    for (let b = b0; b < b1 && b * 2 + 1 < peaks.length; b++) {
      mn = Math.min(mn, peaks[b * 2]!);
      mx = Math.max(mx, peaks[b * 2 + 1]!);
    }
    ctx.fillRect(x, mid - mx * mid, 1, Math.max(1, (mx - mn) * mid));
  }
}

/** Clipe com forma de onda (canvas limitado a 4096 px e escalado). */
export const ClipView = memo(function ClipView({ clip, left, width, selected, muted, onPointerDown }: { clip: Clip; left: number; width: number; selected: boolean; muted: boolean; onPointerDown: (e: React.PointerEvent) => void }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const fpp = Math.max(64, 2 ** Math.round(Math.log2(Math.max(1, clip.lengthFrames / Math.max(1, width)))));
  useEffect(() => {
    let alive = true;
    peaksFor(clip.sourceId, fpp)
      .then((p) => {
        const c = ref.current;
        if (!alive || !c) return;
        c.width = Math.max(1, Math.min(MAX_CANVAS, Math.round(width)));
        c.height = 64;
        draw(c, p, clip, fpp);
      })
      .catch(() => {
        /* fonte indisponível: o clipe aparece sem forma de onda */
      });
    return () => {
      alive = false;
    };
  }, [clip, fpp, width]);
  return (
    <div
      className={`clip${muted ? " clip--muted" : ""}`}
      style={{ left, width: Math.max(4, width) }}
      aria-selected={selected}
      role="button"
      aria-label={`Clipe ${clip.sourceName}`}
      tabIndex={-1}
      onPointerDown={onPointerDown}
      title={`${clip.sourceName}${clip.gain !== 0 ? ` (${clip.gain.toFixed(1)} dB)` : ""}`}
    >
      <span className="clip-label">{clip.sourceName}</span>
      <canvas ref={ref} />
    </div>
  );
});
