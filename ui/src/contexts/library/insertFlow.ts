// Regras puras do fluxo "inserir base" com divergência de BPM/tom (RF-4.5: sempre perguntar).
import type { BaseDecision, InsertBaseOutcome, MismatchReport } from "../../ipc/types";

export type InsertFlow =
  | { step: "idle" }
  | { step: "checking"; baseId: string }
  | { step: "ready"; baseId: string; report: MismatchReport }
  | { step: "inserting"; baseId: string; report: MismatchReport; decision: BaseDecision | null }
  | { step: "done"; baseId: string; decision: BaseDecision }
  | { step: "error"; baseId: string; message: string };

export type FlowEvent =
  | { type: "open"; baseId: string }
  | { type: "recheck" }
  | { type: "report"; report: MismatchReport }
  | { type: "choose"; decision: BaseDecision | null }
  | { type: "outcome"; outcome: InsertBaseOutcome }
  | { type: "fail"; message: string }
  | { type: "close" };

const baseOf = (s: InsertFlow): string | null => (s.step === "idle" ? null : s.baseId);

export function insertFlow(s: InsertFlow, e: FlowEvent): InsertFlow {
  switch (e.type) {
    case "open":
      return { step: "checking", baseId: e.baseId };
    case "close":
      return { step: "idle" };
    case "recheck": {
      const id = baseOf(s);
      return id && s.step !== "inserting" && s.step !== "done" ? { step: "checking", baseId: id } : s;
    }
    case "report":
      return s.step === "checking" ? { step: "ready", baseId: s.baseId, report: e.report } : s;
    case "choose":
      if (s.step !== "ready") return s;
      // com divergência, inserir exige uma decisão explícita (nunca decide sozinho)
      if (s.report.needsDecision && e.decision === null) return s;
      return { step: "inserting", baseId: s.baseId, report: s.report, decision: e.decision };
    case "outcome":
      if (s.step !== "inserting") return s;
      if (e.outcome.status === "needsDecision") return { step: "ready", baseId: s.baseId, report: e.outcome.report };
      return { step: "done", baseId: s.baseId, decision: e.outcome.decision };
    case "fail": {
      const id = baseOf(s);
      return id ? { step: "error", baseId: id, message: e.message } : s;
    }
  }
}

const fmt = (n: number, d = 2) => n.toLocaleString("pt-BR", { maximumFractionDigits: d });

/** Linhas legíveis do relatório de divergência. */
export function describeMismatch(r: MismatchReport, baseBpm: number): string[] {
  const lines: string[] = [];
  const projectBpm = r.bpmRatio > 0 ? baseBpm / r.bpmRatio : baseBpm;
  if (Math.abs(r.bpmRatio - 1) > 1e-6) {
    lines.push(`Andamento: base ${fmt(baseBpm)} BPM → projeto ${fmt(projectBpm)} BPM (×${fmt(r.bpmRatio, 3)})`);
  } else {
    lines.push(`Andamento igual (${fmt(baseBpm)} BPM)`);
  }
  if (r.semitoneShift === null) lines.push("Tom: não comparado (tom da base ou do projeto desconhecido)");
  else if (r.semitoneShift === 0) lines.push("Tom: mesmo centro tonal");
  else {
    const n = Math.abs(r.semitoneShift);
    lines.push(`Tom: ${n} ${n > 1 ? "semitons" : "semitom"} ${r.semitoneShift > 0 ? "acima" : "abaixo"} para chegar ao projeto`);
  }
  if (r.modeDiffers) lines.push("Modo diferente (maior × menor): a adaptação mira a relativa");
  return lines;
}
