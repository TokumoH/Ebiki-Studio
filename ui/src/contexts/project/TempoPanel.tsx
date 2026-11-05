import { useState } from "react";
import { api } from "../../ipc/api";
import { run, useSession } from "../../core/session";
import { Button, Field } from "../../shared/components";
import { PPQ } from "./grid";

/** Editor do mapa de tempo: mudanças de andamento e de fórmula de compasso (por compasso). */
export function TempoPanel() {
  const session = useSession((s) => s.session);
  const [bpmBar, setBpmBar] = useState(1);
  const [bpm, setBpm] = useState(120);
  const [meterBar, setMeterBar] = useState(1);
  const [num, setNum] = useState(4);
  const [den, setDen] = useState(4);
  if (!session) return null;
  const meters = session.tempo.meters;
  // tick do início do compasso N considerando mudanças de fórmula
  const barToTick = (bar: number) => {
    let acc = 1;
    for (let i = 0; i < meters.length; i++) {
      const m = meters[i]!;
      const barTicks = ((PPQ * 4) / m.denominator) * m.numerator;
      const next = meters[i + 1];
      const barsInSeg = next ? (next.tick - m.tick) / barTicks : Number.POSITIVE_INFINITY;
      if (bar < acc + barsInSeg) return m.tick + (bar - acc) * barTicks;
      acc += barsInSeg;
    }
    return 0;
  };
  const tickToBar = (tick: number) => {
    let acc = 1;
    for (let i = 0; i < meters.length; i++) {
      const m = meters[i]!;
      const barTicks = ((PPQ * 4) / m.denominator) * m.numerator;
      const next = meters[i + 1];
      if (!next || tick < next.tick) return acc + (tick - m.tick) / barTicks;
      acc += (next.tick - m.tick) / barTicks;
    }
    return 1;
  };
  const fmtBar = (t: number) => {
    const b = tickToBar(t);
    return Number.isInteger(b) ? String(b) : b.toFixed(2);
  };
  return (
    <div className="tempo-panel">
      <section>
        <h3>Andamento</h3>
        <table>
          <thead>
            <tr>
              <th>Compasso</th>
              <th>BPM</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {session.tempo.tempos.map((t) => (
              <tr key={t.tick}>
                <td className="mono">{fmtBar(t.tick)}</td>
                <td className="mono">{t.bpm.toFixed(2)}</td>
                <td>{t.tick > 0 ? <Button size="sm" variant="ghost" aria-label={`Remover andamento no compasso ${fmtBar(t.tick)}`} onClick={() => void run(() => api.tempoRemove(t.tick))}>✕</Button> : null}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(() => api.tempoSet(barToTick(bpmBar), bpm));
          }}
        >
          <Field label="Compasso">
            <input className="input" type="number" min={1} value={bpmBar} onChange={(e) => setBpmBar(Math.max(1, Number(e.target.value)))} style={{ width: 80 }} />
          </Field>
          <Field label="BPM">
            <input className="input" type="number" min={20} max={999} step={0.01} value={bpm} onChange={(e) => setBpm(Number(e.target.value))} style={{ width: 90 }} />
          </Field>
          <Button type="submit">Definir</Button>
        </form>
      </section>
      <section>
        <h3>Fórmula de compasso</h3>
        <table>
          <thead>
            <tr>
              <th>Compasso</th>
              <th>Fórmula</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {meters.map((m) => (
              <tr key={m.tick}>
                <td className="mono">{fmtBar(m.tick)}</td>
                <td className="mono">
                  {m.numerator}/{m.denominator}
                </td>
                <td>{m.tick > 0 ? <Button size="sm" variant="ghost" aria-label={`Remover fórmula no compasso ${fmtBar(m.tick)}`} onClick={() => void run(() => api.meterRemove(m.tick))}>✕</Button> : null}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(() => api.meterSet(barToTick(meterBar), num, den));
          }}
        >
          <Field label="Compasso">
            <input className="input" type="number" min={1} value={meterBar} onChange={(e) => setMeterBar(Math.max(1, Number(e.target.value)))} style={{ width: 80 }} />
          </Field>
          <Field label="Tempos">
            <input className="input" type="number" min={1} max={32} value={num} onChange={(e) => setNum(Number(e.target.value))} style={{ width: 70 }} />
          </Field>
          <Field label="Figura">
            <select className="select" value={den} onChange={(e) => setDen(Number(e.target.value))}>
              {[1, 2, 4, 8, 16, 32].map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          </Field>
          <Button type="submit">Definir</Button>
        </form>
      </section>
    </div>
  );
}
