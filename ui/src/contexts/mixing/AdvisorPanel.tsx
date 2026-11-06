import { useState } from "react";
import "./mixer.css";
import { api } from "../../ipc/api";
import type { MixReport } from "../../ipc/types";
import { errorMessage, useSession } from "../../core/session";
import { Button, EmptyState, Field } from "../../shared/components";
import { notify } from "../../core/toasts";

const fmt = (x: number | null | undefined, unit: string) => (x === null || x === undefined || !Number.isFinite(x) ? "—" : `${x.toFixed(1)} ${unit}`);

/** Auxílio de mixagem por regras: mede a mixagem offline e lista conselhos. */
export function AdvisorPanel() {
  const session = useSession((s) => s.session);
  const [report, setReport] = useState<MixReport | null>(null);
  const [target, setTarget] = useState(-14);
  const [busy, setBusy] = useState(false);
  if (!session) return null;
  const analyze = async () => {
    setBusy(true);
    try {
      setReport(await api.mixAnalyze(target));
    } catch (e) {
      notify.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const m = report?.analysis.master;
  return (
    <div className="advisor">
      <div style={{ display: "flex", gap: 8, alignItems: "end", flexWrap: "wrap" }}>
        <Field label="Meta de loudness">
          <select className="select" value={target} onChange={(e) => setTarget(Number(e.target.value))}>
            <option value={-14}>−14 LUFS (Spotify, YouTube)</option>
            <option value={-16}>−16 LUFS (Apple Music)</option>
            <option value={-23}>−23 LUFS (TV / EBU R128)</option>
            <option value={-9}>−9 LUFS (alto)</option>
          </select>
        </Field>
        <Button variant="primary" onClick={() => void analyze()} disabled={busy || session.contentEndFrame === 0}>
          {busy ? "Analisando…" : "Analisar mixagem"}
        </Button>
      </div>
      {!report ? (
        <EmptyState title="Análise de mixagem">A mixagem é renderizada e medida (pico, pico real/true peak, LUFS, correlação). Os conselhos seguem regras de produção profissional.</EmptyState>
      ) : (
        <>
          <div className="advisor-stats">
            <div className="stat"><span>Loudness integrado</span><strong>{fmt(m?.integratedLufs, "LUFS")}</strong></div>
            <div className="stat"><span>Pico real (true peak)</span><strong>{fmt(m?.truePeakDb, "dBTP")}</strong></div>
            <div className="stat"><span>Pico</span><strong>{fmt(m?.peakDb, "dBFS")}</strong></div>
            <div className="stat"><span>RMS</span><strong>{fmt(m?.rmsDb, "dB")}</strong></div>
            <div className="stat"><span>Correlação</span><strong>{m ? m.correlation.toFixed(2) : "—"}</strong></div>
          </div>
          {report.advice.length === 0 ? <p>Nenhum problema encontrado. 🎉</p> : null}
          {report.advice.map((a, i) => (
            <div key={i} className="advice" data-severity={a.severity}>
              {a.channel ? <strong>{a.channel}: </strong> : null}
              {a.message}
            </div>
          ))}
        </>
      )}
    </div>
  );
}
