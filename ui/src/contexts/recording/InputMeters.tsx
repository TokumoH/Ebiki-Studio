// Medidores do microfone (entrada) com a faixa-alvo verde (−18 a −6 dBFS) e os avisos de nível.
// Usados no bloco "Microfone (entrada)" do painel Gravação e nas Configurações de áudio: o mesmo
// componente, as mesmas cores e os mesmos textos nos dois lugares.
import { useRef } from "react";
import { useLive } from "../../core/live";
import { dbFraction, decayPeak, levelZone, meterFraction, peakDb, TARGET_ZONE_DB } from "./inputRoutes";
import "./recording.css";

/** Faixa verde do alvo, sobreposta ao fundo do medidor. */
function TargetZone() {
  const left = dbFraction(TARGET_ZONE_DB[0]) * 100;
  const width = (dbFraction(TARGET_ZONE_DB[1]) - dbFraction(TARGET_ZONE_DB[0])) * 100;
  return <span className="input-meter-target" aria-hidden="true" style={{ left: `${left}%`, width: `${width}%` }} />;
}

/** Picos por canal com decaimento visual (sobe na hora, desce aos poucos). */
function useHeldPeaks(channels: number): number[] {
  const inputs = useLive((s) => s.meters?.inputs);
  const shown = useRef<number[]>([]);
  shown.current = Array.from({ length: channels }, (_, i) => decayPeak(shown.current[i] ?? 0, inputs?.[i] ?? 0));
  return shown.current;
}

/** Aviso de nível a partir do maior pico entre os canais (`null` = nível bom). */
export function levelHint(maxDb: number): { kind: "low" | "clip"; text: string } | null {
  const z = levelZone(maxDb);
  if (z === "low") return { kind: "low", text: "Sinal baixo: fale no volume normal e suba o ganho até o medidor ficar na faixa verde (−18 a −6 dB)." };
  if (z === "clip") return { kind: "clip", text: "Muito alto: abaixe o ganho para não distorcer." };
  return null;
}

/** Medidores por canal (até 8) com faixa-alvo e o aviso de nível logo abaixo. */
export function InputMeters({ channels }: { channels: number }) {
  const peaks = useHeldPeaks(channels);
  const maxDb = peakDb(Math.max(0, ...peaks));
  const hint = levelHint(maxDb);
  return (
    <div className="input-meters">
      {peaks.map((p, i) => {
        const db = peakDb(p);
        const zone = levelZone(db);
        return (
          <div key={i} className="input-meter-row">
            <span className="input-meter-label">Entrada {i + 1}</span>
            <div className="input-meter" role="meter" aria-label={`Nível da entrada ${i + 1}`} aria-valuemin={-60} aria-valuemax={0} aria-valuenow={Math.round(db)} data-zone={zone}>
              <TargetZone />
              <div className={`input-meter-fill input-meter-fill--${zone}`} style={{ width: `${meterFraction(p) * 100}%` }} />
            </div>
            <span className={`input-meter-db mono${zone === "clip" ? " input-meter-db--clip" : ""}`}>{zone === "clip" ? "Muito alto" : db <= -60 ? "−∞" : `${Math.round(db)} dB`}</span>
          </div>
        );
      })}
      {hint ? (
        <p className={`input-level-hint input-level-hint--${hint.kind}`} role="status">
          {hint.text}
        </p>
      ) : (
        <p className="input-level-hint" role="status">
          Nível bom: mantenha os trechos fortes na faixa verde.
        </p>
      )}
    </div>
  );
}

/** Resumo do bloco recolhido: o maior nível entre os canais, numa barra só (com a faixa-alvo). */
export function InputSummaryMeter() {
  const inputs = useLive((s) => s.meters?.inputs);
  const peak = Math.max(0, ...(inputs ?? []));
  const db = peakDb(peak);
  return (
    <span className="input-summary-meter" role="meter" aria-label="Nível da entrada (resumo)" aria-valuemin={-60} aria-valuemax={0} aria-valuenow={Math.round(db)} data-zone={levelZone(db)}>
      <TargetZone />
      <span className={`input-summary-fill input-meter-fill--${levelZone(db)}`} style={{ width: `${meterFraction(peak) * 100}%` }} />
    </span>
  );
}
