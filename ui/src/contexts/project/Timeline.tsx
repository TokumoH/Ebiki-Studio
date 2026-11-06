import { useCallback, useMemo, useRef, useState } from "react";
import "./timeline.css";
import { api, editApi } from "../../ipc/api";
import type { Clip, SessionView, Track } from "../../ipc/types";
import { run, useSession, newGesture, endGesture } from "../../core/session";
import { useLive } from "../../core/live";
import { Button, EmptyState, Toggle } from "../../shared/components";
import { gridLines, snapToBeat, PPQ } from "./grid";
import { ClipView, type ClipHandle } from "./ClipView";
import { ClipContextMenu, clipMenuItems } from "./ClipContextMenu";
import { useClipViewMode } from "./spectrogramCache";
import { canSplitAt, fadeFromPx, msToFrames, overlappingNeighbor, trimEndTo, trimStartTo } from "./clipEdit";
import { crossfadeWithNeighbor, findClip, normalizeClip, setPolarity, setReversed, splitAtCursor } from "./clipActions";

const TRACK_H = 72;

function TrackHeader({ track, index, selected }: { track: Track; index: number; selected: boolean }) {
  const select = useSession((s) => s.select);
  const [editing, setEditing] = useState(false);
  const spectrum = useClipViewMode((s) => Boolean(s.spectrum[track.id]));
  const toggleSpectrum = useClipViewMode((s) => s.toggle);
  const target = { kind: "track" as const, id: track.id };
  return (
    <div className="track-header" aria-selected={selected} onPointerDown={() => select({ trackId: track.id })} data-testid={`track-header-${index}`}>
      {editing ? (
        <input
          className="input"
          autoFocus
          defaultValue={track.name}
          aria-label="Nome da faixa"
          onBlur={(e) => {
            setEditing(false);
            if (e.target.value !== track.name) void run(() => api.trackRename(track.id, e.target.value));
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
            if (e.key === "Escape") setEditing(false);
          }}
        />
      ) : (
        <div className="name" onDoubleClick={() => setEditing(true)} title="Duplo clique para renomear">
          {track.name}
        </div>
      )}
      <div className="row">
        <Toggle className="toggle-m" pressed={track.strip.mute} onChange={(v) => void run(() => api.mixerEdit(target, { op: "mute", value: v }))} aria-label={`Mute ${track.name}`}>
          M
        </Toggle>
        <Toggle className="toggle-s" pressed={track.strip.solo} onChange={(v) => void run(() => api.mixerEdit(target, { op: "solo", value: v }))} aria-label={`Solo ${track.name}`}>
          S
        </Toggle>
        <Toggle
          className="toggle-r"
          pressed={track.recording.armed}
          onChange={(v) => void run(() => api.recordingTrack({ trackId: track.id, armed: v, input: track.recording.input ?? { left: 0, right: null } }))}
          aria-label={`Armar ${track.name}`}
        >
          ⏺
        </Toggle>
        <Toggle className="toggle-view" pressed={spectrum} onChange={() => toggleSpectrum(track.id)} aria-label={`Espectrograma ${track.name}`} title={spectrum ? "Mostrando espectrograma (clique para forma de onda)" : "Mostrando forma de onda (clique para espectrograma)"}>
          ≋
        </Toggle>
        <span style={{ flex: 1 }} />
        <Button size="sm" variant="ghost" icon aria-label={`Remover ${track.name}`} title="Remover faixa" onClick={() => void run(() => api.trackDelete(track.id))}>
          ✕
        </Button>
      </div>
    </div>
  );
}

interface DragState {
  clipId: string;
  startX: number;
  startY: number;
  originTick: number;
  originTrack: number;
  gesture: number;
  moved: boolean;
}

/** Arraste de uma borda/alça de clipe (cortar ou fade). */
interface HandleDrag {
  clip: Clip;
  handle: ClipHandle;
  startX: number;
  widthPx: number;
  gesture: number;
  moved: boolean;
}

export function Timeline() {
  const session = useSession((s) => s.session);
  const spectrumTracks = useClipViewMode((s) => s.spectrum);
  const [menu, setMenu] = useState<{ clipId: string; x: number; y: number } | null>(null);
  const selectedTrackId = useSession((s) => s.selectedTrackId);
  const selectedClipId = useSession((s) => s.selectedClipId);
  const select = useSession((s) => s.select);
  const tick = useLive((s) => s.transport?.tick ?? 0);
  const [pxPerBeat, setPxPerBeat] = useState(40);
  const [scroll, setScroll] = useState({ left: 0, top: 0, width: 1200 });
  const lanesRef = useRef<HTMLDivElement>(null);
  const headersRef = useRef<HTMLDivElement>(null);
  const drag = useRef<DragState | null>(null);
  const pxPerTick = pxPerBeat / PPQ;

  const totalTicks = useMemo(() => {
    if (!session) return PPQ * 64;
    const end = Math.max(0, ...session.tracks.flatMap((t) => t.clips.map((c) => c.endTick)));
    return Math.max(PPQ * 4 * 64, end + PPQ * 4 * 16);
  }, [session]);

  const visibleFrom = scroll.left / pxPerTick;
  const visibleTo = (scroll.left + scroll.width) / pxPerTick;
  const lines = useMemo(() => (session ? gridLines(session.tempo, visibleFrom - PPQ * 4, visibleTo + PPQ * 4) : []), [session, visibleFrom, visibleTo]);
  const showBeats = pxPerBeat >= 18;

  const onScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    if (headersRef.current) headersRef.current.scrollTop = el.scrollTop;
    setScroll({ left: el.scrollLeft, top: el.scrollTop, width: el.clientWidth });
  }, []);

  const xToTick = (clientX: number) => {
    const el = lanesRef.current;
    if (!el) return 0;
    return Math.max(0, (clientX - el.getBoundingClientRect().left + el.scrollLeft) / pxPerTick);
  };

  const onClipPointerDown = (s: SessionView, trackIndex: number, clipId: string, start: number) => (e: React.PointerEvent) => {
    e.stopPropagation();
    select({ clipId, trackId: s.tracks[trackIndex]?.id ?? null });
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag.current = { clipId, startX: e.clientX, startY: e.clientY, originTick: start, originTrack: trackIndex, gesture: newGesture(), moved: false };
    const el = e.currentTarget as HTMLElement;
    let pending: Promise<unknown> = Promise.resolve();
    const move = (ev: PointerEvent) => {
      const d = drag.current;
      if (!d) return;
      const dx = ev.clientX - d.startX;
      const dy = ev.clientY - d.startY;
      if (!d.moved && Math.abs(dx) < 3 && Math.abs(dy) < 3) return;
      d.moved = true;
      let t = Math.max(0, d.originTick + dx / pxPerTick);
      if (!ev.altKey) t = snapToBeat(s.tempo, t);
      const ti = Math.min(s.tracks.length - 1, Math.max(0, d.originTrack + Math.round(dy / TRACK_H)));
      const target = s.tracks[ti];
      if (!target) return;
      // serializa as chamadas do gesto (um único item de desfazer no núcleo)
      pending = pending.then(() => run(() => api.clipMove(d.clipId, target.id, Math.round(t), d.gesture)));
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      const d = drag.current;
      drag.current = null;
      if (d?.moved) void pending.then(() => endGesture());
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  };

  /** Corta (`value` = tick da borda) ou aplica fade (`value` = amostras) conforme a alça. */
  const applyHandle = (s: SessionView, c: Clip, handle: ClipHandle, value: number, gesture?: number, snap = true) => {
    if (handle === "trimStart" || handle === "trimEnd") {
      const t = snap ? snapToBeat(s.tempo, value) : value;
      const r = handle === "trimStart" ? trimStartTo(c, t) : trimEndTo(c, t);
      if (r.start === c.start && r.lengthFrames === c.lengthFrames) return Promise.resolve(true);
      return run(() => editApi.clipTrim(c.id, r.start, r.lengthFrames, gesture));
    }
    const frames = Math.max(0, Math.round(value));
    const fades = { fadeInFrames: c.fadeInFrames, fadeOutFrames: c.fadeOutFrames, fadeInShape: c.fadeInShape, fadeOutShape: c.fadeOutShape };
    if (handle === "fadeIn") fades.fadeInFrames = Math.min(frames, c.lengthFrames - c.fadeOutFrames);
    else fades.fadeOutFrames = Math.min(frames, c.lengthFrames - c.fadeInFrames);
    if (fades.fadeInFrames === c.fadeInFrames && fades.fadeOutFrames === c.fadeOutFrames) return Promise.resolve(true);
    return run(() => editApi.clipSetFades(c.id, fades, gesture));
  };

  const onHandlePointerDown = (s: SessionView, c: Clip, widthPx: number) => (handle: ClipHandle, e: React.PointerEvent) => {
    select({ clipId: c.id });
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const d: HandleDrag = { clip: c, handle, startX: e.clientX, widthPx, gesture: newGesture(), moved: false };
    let pending: Promise<unknown> = Promise.resolve();
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - d.startX;
      if (!d.moved && Math.abs(dx) < 2) return;
      d.moved = true;
      const pxPerFrame = d.widthPx / Math.max(1, c.lengthFrames);
      let value: number;
      if (handle === "trimStart") value = c.start + dx / pxPerTick;
      else if (handle === "trimEnd") value = c.endTick + dx / pxPerTick;
      else if (handle === "fadeIn") value = fadeFromPx(c, "in", c.fadeInFrames * pxPerFrame + dx, d.widthPx);
      else value = fadeFromPx(c, "out", c.fadeOutFrames * pxPerFrame - dx, d.widthPx);
      pending = pending.then(() => applyHandle(s, c, handle, value, d.gesture, !ev.altKey));
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      if (d.moved) void pending.then(() => endGesture());
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  };

  /** Teclado nas alças: cortar em semicolcheias (Shift = tempo); fades em 10 ms (Shift = 100 ms). */
  const onHandleKey = (s: SessionView, c: Clip) => (handle: ClipHandle, dir: -1 | 1, big: boolean) => {
    if (handle === "trimStart" || handle === "trimEnd") {
      const step = (big ? PPQ : PPQ / 4) * dir;
      void applyHandle(s, c, handle, (handle === "trimStart" ? c.start : c.endTick) + step, undefined, false);
      return;
    }
    // a alça de saída fica à direita: mover para a direita encurta o fade de saída
    const step = msToFrames(big ? 100 : 10, s.project.sampleRate) * dir;
    const value = handle === "fadeIn" ? c.fadeInFrames + step : c.fadeOutFrames - step;
    void applyHandle(s, c, handle, value);
  };

  const openMenu = (clipId: string, x: number, y: number) => {
    const el = lanesRef.current?.parentElement;
    const r = el?.getBoundingClientRect();
    setMenu({ clipId, x: x - (r?.left ?? 0), y: y - (r?.top ?? 0) });
  };

  if (!session) return null;
  const totalWidth = totalTicks * pxPerTick;
  const menuClip = menu ? findClip(session, menu.clipId) : null;

  return (
    <div
      className="timeline"
      style={{ ["--track-h" as string]: `${TRACK_H}px` }}
      tabIndex={0}
      aria-label="Linha do tempo"
      onKeyDown={(e) => {
        if ((e.key === "Delete" || e.key === "Backspace") && selectedClipId) {
          e.preventDefault();
          void run(() => api.clipDelete(selectedClipId));
        }
        // menu de contexto pelo teclado (tecla Menu ou Shift+F10)
        if ((e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) && selectedClipId) {
          e.preventDefault();
          const el = lanesRef.current?.querySelector<HTMLElement>(`[data-clip-id="${selectedClipId}"]`);
          const r = el?.getBoundingClientRect();
          openMenu(selectedClipId, (r?.left ?? 0) + 8, (r?.top ?? 0) + 16);
        }
      }}
    >
      <div className="timeline-corner">
        <Button size="sm" onClick={() => void run(() => api.trackCreate(`Faixa ${session.tracks.length + 1}`))} title="Nova faixa (Ctrl+T)">
          + Faixa
        </Button>
        <span style={{ flex: 1 }} />
        <Button size="sm" icon variant="ghost" aria-label="Diminuir zoom" onClick={() => setPxPerBeat((z) => Math.max(4, z / 1.5))}>
          −
        </Button>
        <Button size="sm" icon variant="ghost" aria-label="Aumentar zoom" onClick={() => setPxPerBeat((z) => Math.min(400, z * 1.5))}>
          +
        </Button>
      </div>
      <div className="ruler" aria-label="Régua (clique para posicionar)" onPointerDown={(e) => void run(() => api.seek(Math.round(snapToBeat(session.tempo, xToTick(e.clientX)))))}>
        <div style={{ position: "absolute", left: -scroll.left, top: 0, bottom: 0, width: totalWidth }}>
          {session.recording.loopRange ? (
            <div className="ruler-range ruler-range--loop" style={{ left: session.recording.loopRange[0] * pxPerTick, width: (session.recording.loopRange[1] - session.recording.loopRange[0]) * pxPerTick }} title="Loop" />
          ) : null}
          {session.recording.punch ? (
            <div className="ruler-range ruler-range--punch" style={{ left: session.recording.punch[0] * pxPerTick, width: (session.recording.punch[1] - session.recording.punch[0]) * pxPerTick }} title="Punch" />
          ) : null}
          {lines
            .filter((l) => l.isBar || showBeats)
            .map((l) => (
              <div key={l.tick} className={`ruler-mark${l.isBar ? "" : " ruler-mark--beat"}`} style={{ left: l.tick * pxPerTick }}>
                {l.isBar ? l.bar : ""}
              </div>
            ))}
          <div className="playhead" style={{ left: tick * pxPerTick }} />
        </div>
      </div>
      <div className="track-headers" ref={headersRef} data-scroll-sync="lanes">
        {session.tracks.map((t, i) => (
          <TrackHeader key={t.id} track={t} index={i} selected={t.id === selectedTrackId} />
        ))}
        <div style={{ height: 200 }} />
      </div>
      <div className="lanes" ref={lanesRef} onScroll={onScroll} onPointerDown={() => select({ clipId: null })}>
        {session.tracks.length === 0 ? (
          <EmptyState title="Projeto vazio">Importe áudio (Ctrl+I) ou crie uma faixa para começar.</EmptyState>
        ) : (
          <div className="lanes-inner" style={{ width: totalWidth, height: session.tracks.length * TRACK_H + 200 }}>
            {lines
              .filter((l) => l.isBar || showBeats)
              .map((l) => (
                <div key={l.tick} className={`grid-line${l.isBar ? "" : " grid-line--beat"}`} style={{ left: l.tick * pxPerTick }} />
              ))}
            {session.tracks.map((t, ti) => (
              <div key={t.id} className="lane" aria-selected={t.id === selectedTrackId} data-testid={`lane-${ti}`} onPointerDown={() => select({ trackId: t.id })}>
                {t.clips.map((c) => {
                  const left = c.start * pxPerTick;
                  const width = (c.endTick - c.start) * pxPerTick;
                  if (left > scroll.left + scroll.width + 200 || left + width < scroll.left - 200) return null;
                  return (
                    <ClipView
                      key={c.id}
                      clip={c}
                      left={left}
                      width={width}
                      selected={c.id === selectedClipId}
                      muted={!t.audible}
                      spectrum={Boolean(spectrumTracks[t.id])}
                      sampleRate={session.project.sampleRate}
                      onPointerDown={onClipPointerDown(session, ti, c.id, c.start)}
                      onHandlePointerDown={onHandlePointerDown(session, c, Math.max(4, width))}
                      onHandleKey={onHandleKey(session, c)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        select({ clipId: c.id, trackId: t.id });
                        openMenu(c.id, e.clientX, e.clientY);
                      }}
                    />
                  );
                })}
              </div>
            ))}
            <div className="playhead" style={{ left: tick * pxPerTick }} />
          </div>
        )}
      </div>
      {menu && menuClip ? (
        <ClipContextMenu
          x={menu.x}
          y={menu.y}
          label={`Ações do clipe ${menuClip.clip.sourceName}`}
          onClose={() => {
            setMenu(null);
            document.querySelector<HTMLElement>(".timeline")?.focus();
          }}
          items={clipMenuItems(menuClip.clip, {
            canSplit: canSplitAt(menuClip.clip, Math.round(tick)),
            hasOverlap: overlappingNeighbor(menuClip.track, menuClip.clip.id) !== null,
            split: () => void splitAtCursor(menuClip.clip.id),
            normalize: () => void normalizeClip(menuClip.clip.id),
            polarity: (v) => void setPolarity(menuClip.clip.id, v),
            reverse: (v) => void setReversed(menuClip.clip.id, v),
            crossfade: () => void crossfadeWithNeighbor(menuClip.clip.id),
            remove: () => void run(() => api.clipDelete(menuClip.clip.id)),
          })}
        />
      ) : null}
    </div>
  );
}
