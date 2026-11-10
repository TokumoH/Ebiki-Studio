import { useState } from "react";
import { api } from "../../ipc/api";
import { run, useSession } from "../../core/session";
import { Button, Field } from "../../shared/components";
import { barBeatAt, barStartAt, barToTick, tickToBar } from "../../core/bars";
import { formatBarBeat } from "../../core/format";

/** Tabela de eventos do mapa de tempo (o evento do início não pode ser removido). */
function EventTable<T extends { tick: number }>({ rows, valueLabel, value, what, fmtBar, onRemove }: { rows: T[]; valueLabel: string; value: (r: T) => string; what: string; fmtBar: (tick: number) => string; onRemove: (tick: number) => void }) {
  return (
    <table>
      <thead>
        <tr>
          <th>Compasso</th>
          <th>{valueLabel}</th>
          <th />
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <tr key={r.tick}>
            <td className="mono">{fmtBar(r.tick)}</td>
            <td className="mono">{value(r)}</td>
            <td>
              {r.tick > 0 ? (
                <Button size="sm" variant="ghost" aria-label={`Remover ${what} no compasso ${fmtBar(r.tick)}`} onClick={() => onRemove(r.tick)}>
                  ✕
                </Button>
              ) : null}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Editor do mapa de tempo: mudanças de andamento e de fórmula de compasso (por compasso). */
export function TempoPanel() {
  const session = useSession((s) => s.session);
  const [bpmBar, setBpmBar] = useState(1);
  const [bpm, setBpm] = useState(120);
  const [meterBar, setMeterBar] = useState(1);
  const [num, setNum] = useState(4);
  const [den, setDen] = useState(4);
  if (!session) return null;
  const tempo = session.tempo;
  const meters = tempo.meters;
  // mesma numeração da régua e do loop: cada mudança de fórmula inicia um compasso novo;
  // evento fora do início do compasso (ex.: andamento importado de MIDI) mostra compasso.tempo.tique
  const fmtBar = (tick: number) => (barStartAt(tempo, tick) === tick ? String(tickToBar(tempo, tick)) : formatBarBeat(barBeatAt(tempo, tick)));
  return (
    <div className="tempo-panel">
      <section>
        <h3>Andamento</h3>
        <EventTable
          rows={session.tempo.tempos}
          valueLabel="BPM"
          value={(t) => t.bpm.toFixed(2)}
          what="andamento"
          fmtBar={fmtBar}
          onRemove={(tick) => void run(() => api.tempoRemove(tick))}
        />
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(() => api.tempoSet(barToTick(tempo, bpmBar), bpm));
          }}
        >
          <Field label="Compasso" tip="tempo.bar">
            <input className="input" type="number" min={1} value={bpmBar} onChange={(e) => setBpmBar(Math.max(1, Number(e.target.value)))} style={{ width: 80 }} />
          </Field>
          <Field label="BPM" tip="tempo.bpm">
            <input className="input" type="number" min={20} max={999} step={0.01} value={bpm} onChange={(e) => setBpm(Number(e.target.value))} style={{ width: 90 }} />
          </Field>
          <Button type="submit" data-tip="tempo.setBpm">Definir</Button>
        </form>
      </section>
      <section>
        <h3>Fórmula de compasso</h3>
        <EventTable
          rows={meters}
          valueLabel="Fórmula"
          value={(m) => `${m.numerator}/${m.denominator}`}
          what="fórmula"
          fmtBar={fmtBar}
          onRemove={(tick) => void run(() => api.meterRemove(tick))}
        />
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(() => api.meterSet(barToTick(tempo, meterBar), num, den));
          }}
        >
          <Field label="Compasso" tip="tempo.bar">
            <input className="input" type="number" min={1} value={meterBar} onChange={(e) => setMeterBar(Math.max(1, Number(e.target.value)))} style={{ width: 80 }} />
          </Field>
          <Field label="Tempos" tip="tempo.numerator">
            <input className="input" type="number" min={1} max={32} value={num} onChange={(e) => setNum(Number(e.target.value))} style={{ width: 70 }} />
          </Field>
          <Field label="Figura" tip="tempo.denominator">
            <select className="select" value={den} onChange={(e) => setDen(Number(e.target.value))}>
              {[1, 2, 4, 8, 16, 32].map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          </Field>
          <Button type="submit" data-tip="tempo.setMeter">Definir</Button>
        </form>
      </section>
    </div>
  );
}
