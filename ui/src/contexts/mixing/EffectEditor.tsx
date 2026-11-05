import { useRef } from "react";
import type { Effect, EffectSlot, EqBand, StripTarget } from "../../ipc/types";
import { api } from "../../ipc/api";
import { run, newGesture, endGesture } from "../../core/session";
import { EFFECT_LABELS, PARAMS } from "./effectsMeta";
import { Button, Toggle } from "../../shared/components";

const EQ_KINDS: { value: EqBand["kind"]; label: string }[] = [
  { value: "highPass", label: "Passa-altas" },
  { value: "lowShelf", label: "Shelf graves" },
  { value: "peak", label: "Pico" },
  { value: "highShelf", label: "Shelf agudos" },
  { value: "lowPass", label: "Passa-baixas" },
];

/** Editor genérico de parâmetros; cada arrasto de slider é um único desfazer (gesto). */
export function EffectEditor({ target, slot, onClose }: { target: StripTarget; slot: EffectSlot; onClose: () => void }) {
  const gesture = useRef<number | null>(null);
  const send = (effect: Effect) => {
    gesture.current ??= newGesture();
    void run(() => api.mixerEdit(target, { op: "setEffect", id: slot.id, effect }, gesture.current ?? undefined));
  };
  const finish = () => {
    gesture.current = null;
    void endGesture();
  };
  const e = slot.effect;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <strong style={{ flex: 1 }}>{EFFECT_LABELS[e.type] ?? e.type}</strong>
        <Toggle pressed={slot.bypass} onChange={(v) => void run(() => api.mixerEdit(target, { op: "bypass", id: slot.id, value: v }))} aria-label="Bypass">
          Bypass
        </Toggle>
        <Button size="sm" variant="danger" onClick={() => void run(() => api.mixerEdit(target, { op: "removeEffect", id: slot.id })).then(onClose)}>
          Remover
        </Button>
      </div>
      {e.type === "eq" && "bands" in e.params ? (
        <div className="effect-editor">
          {(e.params.bands as EqBand[]).map((b, i) => {
            const setBand = (patch: Partial<EqBand>) => {
              const bands = (e.params.bands as EqBand[]).map((x, j) => (j === i ? { ...x, ...patch } : x));
              send({ type: "eq", params: { bands } });
            };
            return (
              <div className="eq-band" key={i}>
                <div style={{ display: "flex", gap: 4 }}>
                  <select className="select" value={b.kind} aria-label={`Tipo da banda ${i + 1}`} onChange={(ev) => setBand({ kind: ev.target.value as EqBand["kind"] })} style={{ flex: 1 }}>
                    {EQ_KINDS.map((k) => (
                      <option key={k.value} value={k.value}>
                        {k.label}
                      </option>
                    ))}
                  </select>
                  <Toggle pressed={b.enabled} onChange={(v) => setBand({ enabled: v })} aria-label={`Ativar banda ${i + 1}`}>
                    On
                  </Toggle>
                </div>
                <Slider label="Frequência" unit="Hz" min={20} max={20000} step={1} log value={b.freqHz} onChange={(v) => setBand({ freqHz: v })} onEnd={finish} />
                <Slider label="Ganho" unit="dB" min={-24} max={24} step={0.1} value={b.gainDb} onChange={(v) => setBand({ gainDb: v })} onEnd={finish} />
                <Slider label="Q" unit="" min={0.1} max={18} step={0.01} log value={b.q} onChange={(v) => setBand({ q: v })} onEnd={finish} />
              </div>
            );
          })}
        </div>
      ) : (
        <div className="effect-editor">
          {(PARAMS[e.type] ?? []).map((p) => (
            <Slider
              key={p.key}
              label={p.label}
              unit={p.unit}
              min={p.min}
              max={p.max}
              step={p.step}
              log={p.unit === "Hz" || p.unit === "ms"}
              value={Number((e.params as Record<string, unknown>)[p.key] ?? p.min)}
              onChange={(v) => send({ ...e, params: { ...e.params, [p.key]: v } } as Effect)}
              onEnd={finish}
            />
          ))}
          {e.type === "delay" && "pingPong" in e.params ? (
            <Toggle pressed={Boolean(e.params.pingPong)} onChange={(v) => send({ ...e, params: { ...e.params, pingPong: v } } as Effect)}>
              Ping-pong
            </Toggle>
          ) : null}
        </div>
      )}
    </div>
  );
}

function Slider({ label, unit, min, max, step, value, log, onChange, onEnd }: { label: string; unit: string; min: number; max: number; step: number; value: number; log?: boolean; onChange: (v: number) => void; onEnd: () => void }) {
  const toPos = (v: number) => (log ? Math.log(v / min) / Math.log(max / min) : (v - min) / (max - min));
  const fromPos = (p: number) => {
    const v = log ? min * Math.pow(max / min, p) : min + p * (max - min);
    return Math.round(v / step) * step;
  };
  return (
    <div className="param">
      <label>
        <span>{label}</span>
        <span className="mono">
          {value.toFixed(step < 1 ? (step < 0.1 ? 2 : 1) : 0)} {unit}
        </span>
      </label>
      <input type="range" min={0} max={1} step={0.001} value={toPos(value)} aria-label={label} onChange={(e) => onChange(fromPos(Number(e.target.value)))} onPointerUp={onEnd} onKeyUp={onEnd} />
    </div>
  );
}
