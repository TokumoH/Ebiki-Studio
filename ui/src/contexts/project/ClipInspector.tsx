import { useEffect, useRef, useState } from "react";
import { api, editApi } from "../../ipc/api";
import type { Clip, FadeShape } from "../../ipc/types";
import { run, useSession, newGesture, endGesture } from "../../core/session";
import { useLive } from "../../core/live";
import { Button, EmptyState, Field, Segmented, Toggle } from "../../shared/components";
import { canSplitAt, framesToMs, msToFrames, overlappingNeighbor } from "./clipEdit";
import { crossfadeWithNeighbor, findClip, normalizeClip, setPolarity, setReversed, splitAtCursor } from "./clipActions";

const SHAPES: { value: FadeShape; label: string }[] = [
  { value: "linear", label: "Linear" },
  { value: "equalPower", label: "Equal power" },
];

/** Campo numérico em ms que só envia ao confirmar (Enter/perder o foco). */
function MsInput({ label, frames, sampleRate, onCommit }: { label: string; frames: number; sampleRate: number; onCommit: (frames: number) => void }) {
  const shown = Math.round(framesToMs(frames, sampleRate));
  const [text, setText] = useState(String(shown));
  useEffect(() => setText(String(shown)), [shown]);
  const commit = () => {
    const v = Number(text.replace(",", "."));
    if (Number.isFinite(v) && v >= 0 && Math.round(v) !== shown) onCommit(msToFrames(v, sampleRate));
    else setText(String(shown));
  };
  return (
    <Field label={label}>
      <input
        className="input"
        inputMode="decimal"
        aria-label={label}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
        }}
      />
    </Field>
  );
}

/** Ganho do clipe (−24..+12 dB): arrastar é um único desfazer. */
function GainSlider({ clip }: { clip: Clip }) {
  const [db, setDb] = useState(clip.gain);
  const gesture = useRef<number | null>(null);
  useEffect(() => setDb(clip.gain), [clip.gain]);
  const send = (v: number) => {
    gesture.current ??= newGesture();
    const g = gesture.current;
    void run(() => api.clipSetGain(clip.id, v, g));
  };
  const finish = () => {
    if (gesture.current !== null) {
      gesture.current = null;
      void endGesture();
    }
  };
  return (
    <Field label={`Ganho do clipe: ${db.toFixed(1)} dB`}>
      <input
        type="range"
        min={-24}
        max={12}
        step={0.5}
        value={db}
        aria-label="Ganho do clipe"
        aria-valuetext={`${db.toFixed(1)} dB`}
        onChange={(e) => {
          const v = Number(e.target.value);
          setDb(v);
          send(v);
        }}
        onPointerUp={finish}
        onKeyUp={finish}
        onBlur={finish}
        onDoubleClick={() => {
          setDb(0);
          void run(() => api.clipSetGain(clip.id, 0));
        }}
      />
    </Field>
  );
}

/** Painel "Clipe": ganho, fades, fase, reverso e ações de edição do clipe selecionado. */
export function ClipInspector() {
  const session = useSession((s) => s.session);
  const selectedClipId = useSession((s) => s.selectedClipId);
  const tick = useLive((s) => Math.round(s.transport?.tick ?? 0));
  const found = findClip(session, selectedClipId);
  if (!session || !found) {
    return (
      <div className="clip-inspector">
        <EmptyState title="Nenhum clipe selecionado">Clique num clipe da linha do tempo para editar ganho, fades, fase e reverso.</EmptyState>
      </div>
    );
  }
  const { clip, track } = found;
  const sr = session.project.sampleRate;
  const fades = { fadeInFrames: clip.fadeInFrames, fadeOutFrames: clip.fadeOutFrames, fadeInShape: clip.fadeInShape, fadeOutShape: clip.fadeOutShape };
  const setFades = (patch: Partial<typeof fades>) => void run(() => editApi.clipSetFades(clip.id, { ...fades, ...patch }));
  const overlap = overlappingNeighbor(track, clip.id);
  return (
    <div className="clip-inspector" aria-label={`Edição do clipe ${clip.sourceName}`} role="group">
      <section className="clip-inspector-section">
        <h3 className="clip-inspector-title" title={clip.sourceName}>
          {clip.sourceName}
        </h3>
        <p className="clip-inspector-meta">
          {track.name} · {(clip.lengthFrames / sr).toFixed(2)} s
        </p>
        <GainSlider clip={clip} />
        <div className="clip-inspector-row">
          <Button size="sm" onClick={() => void normalizeClip(clip.id)} title="Ajusta o ganho para o pico chegar a −1 dBFS (o arquivo não é alterado)">
            Normalizar
          </Button>
          <Toggle pressed={clip.polarityInverted} onChange={(v) => void setPolarity(clip.id, v)} title="Inverter fase (polaridade)">
            Inverter fase
          </Toggle>
          <Toggle pressed={clip.reversed} onChange={(v) => void setReversed(clip.id, v)} title="Tocar o clipe de trás para frente">
            Reverter
          </Toggle>
        </div>
      </section>
      <section className="clip-inspector-section">
        <MsInput label="Fade de entrada (ms)" frames={clip.fadeInFrames} sampleRate={sr} onCommit={(f) => setFades({ fadeInFrames: f })} />
        <Segmented label="Curva do fade de entrada" value={clip.fadeInShape} options={SHAPES} onChange={(v) => setFades({ fadeInShape: v })} />
        <MsInput label="Fade de saída (ms)" frames={clip.fadeOutFrames} sampleRate={sr} onCommit={(f) => setFades({ fadeOutFrames: f })} />
        <Segmented label="Curva do fade de saída" value={clip.fadeOutShape} options={SHAPES} onChange={(v) => setFades({ fadeOutShape: v })} />
      </section>
      <section className="clip-inspector-section">
        <div className="clip-inspector-row">
          <Button size="sm" disabled={!canSplitAt(clip, tick)} onClick={() => void splitAtCursor(clip.id)} title="Dividir no cursor (S)">
            Dividir no cursor
          </Button>
          <Button size="sm" disabled={!overlap} onClick={() => void crossfadeWithNeighbor(clip.id)} title={overlap ? `Crossfade com ${overlap.sourceName}` : "Sobreponha outro clipe nesta faixa"}>
            Crossfade
          </Button>
        </div>
      </section>
    </div>
  );
}
