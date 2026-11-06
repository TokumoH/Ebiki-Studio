import { useEffect, useMemo, useRef, useState, type PointerEvent as RPointerEvent, type KeyboardEvent as RKeyboardEvent } from "react";
import "./tuning.css";
import { api, musicApi } from "../../ipc/api";
import type { NoteEdit, PitchCorrectionMap, ScaleCorrection } from "../../ipc/types";
import { endGesture, errorMessage, newGesture, run, useSession } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog, Field, Toggle } from "../../shared/components";
import { NEUTRAL_EDIT, NOTE_NAMES_PT, centsOff, centsToAngle, dragEdit, effectiveMidi, isInTune, midiName, pitchRange, scaleClasses, scaleKind } from "./tuningMath";

/** Afinação: afinador em tempo real + editor de afinação por clipe + time-stretch. */
export function TuningPanel() {
  const session = useSession((s) => s.session);
  if (!session) return null;
  return (
    <div className="tuning">
      <Tuner />
      <PitchEditor />
    </div>
  );
}

// ------------------------------------------------------------------------------- afinador

function Tuner() {
  const a4 = useSession((s) => s.session?.tuning.a4Hz ?? 440);
  const [channels, setChannels] = useState(2);
  const [channel, setChannel] = useState(0);
  const [on, setOn] = useState(false);
  const [reading, setReading] = useState<{ name: string; octave: number; cents: number; frequencyHz: number; note: number } | null>(null);
  const [ref, setRef] = useState(a4);
  useEffect(() => setRef(a4), [a4]);
  useEffect(() => {
    void api.audioStatus().then((s) => setChannels(Math.max(1, s.inputChannels))).catch(() => undefined);
  }, []);
  useEffect(() => {
    if (!on) return;
    let alive = true;
    const id = setInterval(() => {
      void musicApi
        .tunerRead()
        .then((r) => alive && setReading(r))
        .catch((e: unknown) => {
          if (!alive) return;
          setOn(false);
          notify.error(errorMessage(e));
        });
    }, 50);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [on]);
  useEffect(() => () => void musicApi.tunerStop().catch(() => undefined), []);

  const toggle = async () => {
    if (on) {
      setOn(false);
      setReading(null);
      await run(() => musicApi.tunerStop());
    } else if (await run(() => musicApi.tunerStart(channel))) setOn(true);
  };
  const angle = reading ? centsToAngle(reading.cents) : 0;
  const tuned = reading ? isInTune(reading.cents) : false;
  return (
    <section className="tuner" aria-label="Afinador">
      <h3>Afinador</h3>
      <div className="row">
        <Field label="Entrada">
          <select className="select" aria-label="Canal de entrada do afinador" value={channel} disabled={on} onChange={(e) => setChannel(Number(e.target.value))}>
            {Array.from({ length: channels }, (_, i) => (
              <option key={i} value={i}>
                Entrada {i + 1}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Lá4 (Hz)">
          <input
            className="input num"
            type="number"
            min={415}
            max={466}
            step={0.5}
            value={ref}
            aria-label="Referência do Lá4"
            onChange={(e) => setRef(Number(e.target.value))}
            onBlur={() => ref !== a4 && void run(() => musicApi.tuningSetReference(ref)).then((ok) => !ok && setRef(a4))}
            onKeyDown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()}
          />
        </Field>
        <Toggle pressed={on} onChange={() => void toggle()} aria-label="Ligar afinador">
          {on ? "Desligar" : "Ligar"}
        </Toggle>
      </div>
      <div className={`tuner-dial${tuned ? " tuner-dial--ok" : ""}`} role="meter" aria-label="Desvio em cents" aria-valuemin={-50} aria-valuemax={50} aria-valuenow={reading?.cents ?? 0}>
        <svg viewBox="0 0 200 110" aria-hidden>
          <path d="M 20 100 A 80 80 0 0 1 180 100" className="dial-arc" />
          {[-50, -25, 0, 25, 50].map((c) => {
            const a = (centsToAngle(c) * Math.PI) / 180;
            return <line key={c} x1={100 + 72 * Math.sin(a)} y1={100 - 72 * Math.cos(a)} x2={100 + 82 * Math.sin(a)} y2={100 - 82 * Math.cos(a)} className={c === 0 ? "dial-tick dial-tick--zero" : "dial-tick"} />;
          })}
          <g style={{ transform: `rotate(${angle}deg)`, transformOrigin: "100px 100px", transition: "transform 80ms linear" }}>
            <line x1={100} y1={100} x2={100} y2={26} className="dial-needle" />
          </g>
          <circle cx={100} cy={100} r={5} className="dial-hub" />
        </svg>
        <div className="tuner-note" data-testid="tuner-note">
          {reading ? (
            <>
              <strong>
                {reading.name}
                <sub>{reading.octave}</sub>
              </strong>
              <span>{NOTE_NAMES_PT[reading.note % 12]}</span>
            </>
          ) : (
            <strong>—</strong>
          )}
        </div>
        <div className="tuner-readout mono">{reading ? `${reading.cents > 0 ? "+" : ""}${reading.cents.toFixed(1)} cents · ${reading.frequencyHz.toFixed(1)} Hz` : on ? "sem sinal" : "desligado"}</div>
      </div>
    </section>
  );
}

// ----------------------------------------------------------------------- editor de afinação

const PX_SEMI = 14;
const PX_SEC = 90;

function PitchEditor() {
  const session = useSession((s) => s.session);
  const clipId = useSession((s) => s.selectedClipId);
  const [map, setMap] = useState<PitchCorrectionMap | null>(null);
  const [sel, setSel] = useState<string | null>(null);
  const [drag, setDrag] = useState<{ id: string; edit: NoteEdit } | null>(null);
  const [stretching, setStretching] = useState(false);
  const canvas = useRef<HTMLCanvasElement>(null);
  const clip = session?.tracks.flatMap((t) => t.clips).find((c) => c.id === clipId) ?? null;
  const source = clip ? session?.sources.find((s) => s.id === clip.sourceId) : undefined;
  const summary = clip ? session?.tuning.maps.find((m) => m.sourceId === clip.sourceId) : undefined;
  const sr = session?.project.sampleRate ?? 48_000;

  // aplicar/time-stretch/desfazer trocam o clipe (id novo, mesma faixa e posição): segue o clipe
  const last = useRef<{ id: string; trackId: string; start: number } | null>(null);
  useEffect(() => {
    if (!session) return;
    if (clipId) {
      const t = session.tracks.find((x) => x.clips.some((c) => c.id === clipId));
      const c = t?.clips.find((x) => x.id === clipId);
      if (t && c) last.current = { id: c.id, trackId: t.id, start: c.start };
      return;
    }
    const prev = last.current;
    if (!prev || session.tracks.some((t) => t.clips.some((c) => c.id === prev.id))) return;
    const again = session.tracks.find((t) => t.id === prev.trackId)?.clips.find((c) => c.start === prev.start);
    if (again) useSession.getState().select({ trackId: prev.trackId, clipId: again.id });
  }, [session, clipId]);

  useEffect(() => {
    if (!clip || !summary) return setMap(null);
    let live = true;
    void musicApi
      .tuningGet(clip.sourceId)
      .then((m) => live && setMap(m))
      .catch(() => live && setMap(null));
    return () => {
      live = false;
    };
  }, [session, clip?.sourceId, summary?.mapId]); // eslint-disable-line react-hooks/exhaustive-deps

  const edits = useMemo(() => ({ ...(map?.edits ?? {}), ...(drag ? { [drag.id]: drag.edit } : {}) }), [map, drag]);
  const range = useMemo(() => pitchRange(map?.notes ?? [], edits), [map, edits]);
  const width = Math.max(400, Math.ceil(((source?.lengthFrames ?? sr) / sr) * PX_SEC));
  const height = (range.high - range.low + 1) * PX_SEMI;
  const y = (midi: number) => (range.high - midi) * PX_SEMI;

  // grade do piano-roll (canvas): linhas por semitom, teclas pretas sombreadas
  useEffect(() => {
    const c = canvas.current;
    const ctx = c?.getContext("2d");
    if (!c || !ctx) return;
    c.width = width;
    c.height = height;
    const css = getComputedStyle(c);
    ctx.clearRect(0, 0, width, height);
    for (let m = range.low; m <= range.high; m++) {
      const black = [1, 3, 6, 8, 10].includes(m % 12);
      ctx.fillStyle = black ? css.getPropertyValue("--bg-sunken") || "#101214" : css.getPropertyValue("--bg") || "#15171a";
      ctx.fillRect(0, y(m), width, PX_SEMI);
      ctx.fillStyle = css.getPropertyValue("--line") || "#2f353c";
      ctx.fillRect(0, y(m) + PX_SEMI - (m % 12 === 0 ? 2 : 1), width, m % 12 === 0 ? 2 : 1);
    }
    ctx.fillStyle = css.getPropertyValue("--line-strong") || "#3d444d";
    for (let sec = 1; sec * PX_SEC < width; sec++) ctx.fillRect(sec * PX_SEC, 0, 1, height);
  }, [width, height, range.low, range.high]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!session) return null;
  if (!clip || !source) {
    return (
      <section className="pitch-editor" aria-label="Editor de afinação">
        <h3>Editor de afinação</h3>
        <p className="hint">Selecione um clipe de áudio no arranjo para analisar e corrigir a afinação.</p>
      </section>
    );
  }

  const sendEdit = (noteId: string, edit: NoteEdit, gesture?: number) => run(() => musicApi.tuningEditNote(clip.sourceId, noteId, edit, gesture));
  const scale = map?.scale;
  const setScale = (patch: Partial<ScaleCorrection>) => {
    if (!scale) return;
    const next = { ...scale, ...patch };
    if (!next.allowed.includes(next.key)) next.allowed = [...next.allowed, next.key].sort((a, b) => a - b);
    void run(() => musicApi.tuningSetScale(clip.sourceId, next));
  };
  const snapAll = async () => {
    if (!map) return;
    const g = newGesture();
    for (const n of map.notes) if (!(await sendEdit(n.id, { ...(map.edits[n.id] ?? NEUTRAL_EDIT), target: Math.round(n.midi), centsOffset: 0 }, g))) break;
    await endGesture();
  };

  const onNotePointerDown = (e: RPointerEvent<HTMLDivElement>, id: string) => {
    if (!map) return;
    const note = map.notes.find((n) => n.id === id)!;
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    setSel(id);
    const startY = e.clientY;
    let last: NoteEdit | null = null;
    const move = (ev: PointerEvent) => {
      last = dragEdit(note, map.edits[id], (startY - ev.clientY) / PX_SEMI, ev.shiftKey);
      setDrag({ id, edit: last });
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      if (last) void sendEdit(id, last).then(() => setDrag(null));
      else setDrag(null);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  };
  const onNoteKey = (e: RKeyboardEvent<HTMLDivElement>, id: string) => {
    if (!map || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    e.preventDefault();
    e.stopPropagation();
    const note = map.notes.find((n) => n.id === id)!;
    const dir = e.key === "ArrowUp" ? 1 : -1;
    void sendEdit(id, dragEdit(note, map.edits[id], e.shiftKey ? dir * 0.1 : dir, e.shiftKey));
  };

  const selNote = map?.notes.find((n) => n.id === sel) ?? null;
  const selEdit = selNote ? map?.edits[selNote.id] : undefined;
  const kind = scale ? scaleKind(scale.key, scale.allowed) : "major";
  return (
    <section className="pitch-editor" aria-label="Editor de afinação">
      <div className="row">
        <h3 title={source.originalName}>Editor de afinação — {clip.sourceName}</h3>
      </div>
      <div className="row">
        <Button variant={map ? "default" : "primary"} onClick={() => void run(() => musicApi.tuningAnalyze(clip.sourceId), { busy: true })}>
          {map ? "Reanalisar" : "Analisar clipe"}
        </Button>
        <Button disabled={!map} onClick={() => void snapAll()}>
          Corrigir tudo
        </Button>
        <Button variant="primary" disabled={!map} onClick={() => void run(() => musicApi.tuningApply(clip.sourceId, clip.id), { busy: true, success: "Correção aplicada (nova fonte de áudio; Ctrl+Z desfaz)." })}>
          Aplicar
        </Button>
        <Button disabled={!map} onClick={() => void run(() => musicApi.tuningClear(clip.sourceId))}>
          Limpar
        </Button>
        <Button onClick={() => setStretching(true)}>Time-stretch…</Button>
      </div>
      {map && scale ? (
        <div className="row scale-row" aria-label="Correção por escala">
          <Toggle pressed={scale.enabled} onChange={(v) => setScale({ enabled: v })} aria-label="Correção por escala">
            Escala
          </Toggle>
          <select className="select sel-sm" aria-label="Tônica da escala" value={scale.key} onChange={(e) => setScale({ key: Number(e.target.value), allowed: scaleClasses(Number(e.target.value), kind === "custom" ? "major" : kind) })}>
            {NOTE_NAMES_PT.map((n, i) => (
              <option key={n} value={i}>
                {n}
              </option>
            ))}
          </select>
          <select className="select sel-sm" aria-label="Tipo de escala" value={kind} onChange={(e) => e.target.value !== "custom" && setScale({ allowed: scaleClasses(scale.key, e.target.value as "major") })}>
            <option value="major">Maior</option>
            <option value="minor">Menor natural</option>
            <option value="chromatic">Cromática</option>
            {kind === "custom" ? <option value="custom">Personalizada</option> : null}
          </select>
          <label className="mini">
            retune
            <input className="input num" type="number" min={0} max={1000} value={scale.retuneSpeedMs} aria-label="Velocidade de correção (ms)" onChange={(e) => setScale({ retuneSpeedMs: Math.max(0, Math.min(1000, Number(e.target.value))) })} /> ms
          </label>
          <label className="mini">
            humanizar
            <input type="range" min={0} max={1} step={0.05} value={scale.humanize} aria-label="Humanizar" onChange={(e) => setScale({ humanize: Number(e.target.value) })} />
          </label>
        </div>
      ) : null}
      {map ? (
        <>
          <div className="roll" data-testid="pitch-roll" aria-label={`${map.notes.length} notas detectadas`}>
            <div className="roll-keys" style={{ height }}>
              {Array.from({ length: range.high - range.low + 1 }, (_, i) => range.high - i).map((m) => (
                <div key={m} style={{ height: PX_SEMI }} className={m % 12 === 0 ? "roll-key roll-key--c" : "roll-key"}>
                  {m % 12 === 0 || m === range.high ? midiName(m) : ""}
                </div>
              ))}
            </div>
            <div className="roll-scroll">
              <div className="roll-area" style={{ width, height }}>
                <canvas ref={canvas} className="roll-grid" style={{ width, height }} aria-hidden />
                {map.notes.map((n, i) => {
                  const x = (n.startFrame / sr) * PX_SEC;
                  const w = Math.max(4, ((n.endFrame - n.startFrame) / sr) * PX_SEC);
                  const eff = effectiveMidi(n, edits[n.id]);
                  const edited = !!edits[n.id];
                  return (
                    <div key={n.id}>
                      <div className="roll-detected" style={{ left: x, top: y(n.midi) + 3, width: w, height: PX_SEMI - 6 }} aria-hidden />
                      <div
                        className="roll-note"
                        role="slider"
                        tabIndex={0}
                        aria-label={`Nota ${i + 1}: ${midiName(n.midi)} ${centsOff(n.midi) >= 0 ? "+" : ""}${centsOff(n.midi)} cents`}
                        aria-valuemin={0}
                        aria-valuemax={127}
                        aria-valuenow={Math.round(eff * 100) / 100}
                        aria-valuetext={edited ? `corrigida para ${midiName(eff)}${centsOff(eff) ? ` ${centsOff(eff) > 0 ? "+" : ""}${centsOff(eff)} cents` : ""}` : "sem correção"}
                        aria-selected={sel === n.id}
                        data-edited={edited || undefined}
                        style={{ left: x, top: y(eff) + 1, width: w, height: PX_SEMI - 2 }}
                        onPointerDown={(e) => onNotePointerDown(e, n.id)}
                        onFocus={() => setSel(n.id)}
                        onKeyDown={(e) => onNoteKey(e, n.id)}
                      />
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
          <div className="row note-detail" data-testid="note-detail">
            {selNote ? (
              <>
                <span className="mono">
                  {midiName(selNote.midi)} {centsOff(selNote.midi) >= 0 ? "+" : ""}
                  {centsOff(selNote.midi)} c → {midiName(effectiveMidi(selNote, selEdit))}
                  {centsOff(effectiveMidi(selNote, selEdit)) ? ` ${centsOff(effectiveMidi(selNote, selEdit))} c` : ""}
                </span>
                <label className="mini">
                  achatar
                  <input type="range" min={0} max={1} step={0.05} value={selEdit?.flatten ?? 0} aria-label="Achatar vibrato" onChange={(e) => void sendEdit(selNote.id, { ...(selEdit ?? NEUTRAL_EDIT), flatten: Number(e.target.value) })} />
                </label>
                <Button size="sm" disabled={!selEdit} onClick={() => void sendEdit(selNote.id, NEUTRAL_EDIT)}>
                  Remover correção
                </Button>
              </>
            ) : (
              <span className="hint">Arraste uma nota ↕ (semitons; Shift = cents) ou use ↑/↓ com a nota focada.</span>
            )}
          </div>
        </>
      ) : (
        <p className="hint">Clique em “Analisar clipe” para detectar as notas (YIN) de {source.originalName}.</p>
      )}
      <TimeStretchDialog open={stretching} onClose={() => setStretching(false)} clipId={clip.id} projectBpm={session.tempo.tempos[0]?.bpm ?? 120} />
    </section>
  );
}

function TimeStretchDialog({ open, onClose, clipId, projectBpm }: { open: boolean; onClose: () => void; clipId: string; projectBpm: number }) {
  const [sourceBpm, setSourceBpm] = useState(projectBpm);
  const [useProject, setUseProject] = useState(true);
  const [target, setTarget] = useState(projectBpm);
  const goal = useProject ? projectBpm : target;
  const ratio = sourceBpm / goal;
  const valid = sourceBpm >= 20 && sourceBpm <= 999 && goal >= 20 && goal <= 999 && ratio >= 0.25 && ratio <= 4;
  const apply = async () => {
    if (await run(() => musicApi.clipTimeStretch(clipId, sourceBpm, useProject ? null : target), { busy: true, success: "Clipe ajustado ao andamento." })) onClose();
  };
  return (
    <Dialog
      title="Time-stretch do clipe"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" disabled={!valid || Math.abs(ratio - 1) < 1e-6} onClick={() => void apply()}>
            Aplicar time-stretch
          </Button>
        </>
      }
    >
      <Field label="Andamento original do áudio (BPM)">
        <input className="input" type="number" min={20} max={999} step={0.01} value={sourceBpm} aria-label="BPM de origem" onChange={(e) => setSourceBpm(Number(e.target.value))} />
      </Field>
      <label className="checkbox">
        <input type="checkbox" checked={useProject} onChange={(e) => setUseProject(e.target.checked)} /> Usar o andamento do projeto ({projectBpm.toFixed(2)} BPM)
      </label>
      {!useProject ? (
        <Field label="Andamento de destino (BPM)">
          <input className="input" type="number" min={20} max={999} step={0.01} value={target} aria-label="BPM de destino" onChange={(e) => setTarget(Number(e.target.value))} />
        </Field>
      ) : null}
      <p className="hint">
        Duração × {Number.isFinite(ratio) ? ratio.toFixed(3) : "—"} (WSOLA, altura preservada; gera uma fonte nova e pode ser desfeito). {valid ? "" : "A razão precisa ficar entre 0,25 e 4."}
      </p>
    </Dialog>
  );
}
