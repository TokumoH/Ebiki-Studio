import { api } from "../../ipc/api";
import { run, useSession } from "../../core/session";
import { useLive } from "../../core/live";
import { formatBarBeat, formatTime } from "../../core/format";
import { Button, Toggle } from "../../shared/components";
import "./transport.css";

export function TransportBar() {
  const session = useSession((s) => s.session);
  const t = useLive((s) => s.transport);
  if (!session) return null;
  const state = t?.state ?? "stopped";
  const rolling = state === "playing" || state === "recording" || state === "countin";
  const rec = session.recording;
  const meter = session.tempo.meters[0];
  return (
    <div className="transport" role="toolbar" aria-label="Transporte">
      <Button icon variant="ghost" aria-label="Voltar ao início" title="Voltar ao início (Home)" onClick={() => void run(() => api.seek(0))}>
        ⏮
      </Button>
      {rolling ? (
        <Button icon aria-label="Pausar" title="Pausar (Espaço)" onClick={() => void run(() => api.pause())}>
          ⏸
        </Button>
      ) : (
        <Button icon aria-label="Tocar" title="Tocar (Espaço)" onClick={() => void run(() => api.play())}>
          ▶
        </Button>
      )}
      <Button icon aria-label="Parar" title="Parar (Enter volta ao início)" onClick={() => void run(() => api.stop(false))}>
        ⏹
      </Button>
      <Button
        icon
        aria-label={state === "recording" ? "Parar gravação" : "Gravar"}
        aria-pressed={state === "recording" || state === "countin"}
        className="toggle-r"
        title="Gravar (R)"
        onClick={() => void run(() => (state === "recording" || state === "countin" ? api.stop(false) : api.record(t?.tick ?? 0, rec.metronome.countInBars > 0)))}
      >
        ⏺
      </Button>
      <div className="transport-display" aria-live="off">
        <span className="mono big" data-testid="bar-beat">{t ? formatBarBeat(t.barBeat) : "001.1.000"}</span>
        <span className="mono">{formatTime(t?.seconds ?? 0)}</span>
      </div>
      <div className="transport-display">
        <span className="mono big">♩ {(t?.bpm ?? session.tempo.tempos[0]?.bpm ?? 120).toFixed(1)}</span>
        <span className="mono">
          {meter?.numerator ?? 4}/{meter?.denominator ?? 4}
        </span>
      </div>
      <Toggle pressed={rec.metronome.enabled} onChange={(v) => void run(() => api.recordingMetronome({ ...rec.metronome, enabled: v }))} title="Metrônomo (C)" aria-label="Metrônomo">
        Metrônomo
      </Toggle>
      <Toggle
        pressed={rec.loopRange !== null}
        onChange={(v) => void run(() => api.recordingLoop(v ? [0, 960 * 4 * 4] : null))}
        title="Loop (L) — padrão: compassos 1–4"
        aria-label="Loop"
      >
        Loop
      </Toggle>
      {state === "countin" ? <span className="badge-rec">Contagem…</span> : state === "recording" ? <span className="badge-rec">● Gravando</span> : null}
    </div>
  );
}
