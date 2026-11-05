import { useEffect, useReducer, useState } from "react";
import { compositionApi } from "../../ipc/api";
import type { BaseDecision, BaseKey, HarmonyKeysCatalog, LibraryBase } from "../../ipc/types";
import { useComposition } from "../../core/composition";
import { useLive } from "../../core/live";
import { errorMessage, useSession } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog, Field } from "../../shared/components";
import { decodeKey, encodeKey } from "./filters";
import { describeMismatch, insertFlow } from "./insertFlow";
import { allBaseKeys, keyLabel } from "./labels";

/**
 * Inserir base no cursor. Pergunta "Manter" × "Adaptar" sempre que BPM/tom divergem (RF-4.5).
 * O projeto não guarda tom: o usuário escolhe (padrão = último tom da Harmonia).
 */
export function InsertBaseDialog({ base, catalog, onClose }: { base: LibraryBase | null; catalog: HarmonyKeysCatalog | null; onClose: () => void }) {
  const harmonyKey = useComposition((s) => s.key);
  const selectedTrackId = useSession((s) => s.selectedTrackId);
  const tracks = useSession((s) => s.session?.tracks);
  const [flow, dispatch] = useReducer(insertFlow, { step: "idle" });
  const [projectKey, setProjectKey] = useState<BaseKey | null>(null);
  const [onSelected, setOnSelected] = useState(false);
  const [atTick, setAtTick] = useState(0);

  useEffect(() => {
    if (!base) return;
    // posição do cursor no momento em que o diálogo abre
    setAtTick(Math.max(0, useLive.getState().transport?.tick ?? 0));
    setProjectKey({ tonic: harmonyKey.tonic, mode: harmonyKey.isMinor ? "minor" : "major" });
    dispatch({ type: "open", baseId: base.id });
  }, [base?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // (re)calcula a divergência quando abre ou o tom do projeto muda
  useEffect(() => {
    if (!base || flow.step !== "checking") return;
    let alive = true;
    compositionApi.basesMismatch({ baseId: base.id, atTick, projectKey }).then(
      (report) => alive && dispatch({ type: "report", report }),
      (e) => alive && dispatch({ type: "fail", message: errorMessage(e) }),
    );
    return () => {
      alive = false;
    };
  }, [base, flow.step, projectKey, atTick]);

  // executa a inserção quando há uma decisão
  useEffect(() => {
    if (!base || flow.step !== "inserting") return;
    const decision = flow.decision;
    const trackId = onSelected ? selectedTrackId : null;
    compositionApi.basesInsert({ baseId: base.id, atTick, trackId, projectKey, decision }).then(
      (outcome) => {
        dispatch({ type: "outcome", outcome });
        if (outcome.status === "inserted") {
          useSession.getState().select({ trackId: outcome.clip.trackId, clipId: outcome.clip.clipId });
          useSession.getState().setSession(outcome.clip.view);
          notify.success(outcome.decision === "adapt" ? `Base "${base.name}" inserida adaptada ao projeto.` : `Base "${base.name}" inserida.`);
          onClose();
        }
      },
      (e) => dispatch({ type: "fail", message: errorMessage(e) }),
    );
  }, [flow.step]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!base) return null;
  const choose = (decision: BaseDecision | null) => dispatch({ type: "choose", decision });
  const report = flow.step === "ready" || flow.step === "inserting" ? flow.report : null;
  const working = flow.step === "checking" || flow.step === "inserting";
  const selectedName = tracks?.find((t) => t.id === selectedTrackId)?.name;

  return (
    <Dialog
      title={`Inserir "${base.name}"`}
      open
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          {report?.needsDecision ? (
            <>
              <Button disabled={working} onClick={() => choose("keep")}>
                Manter original
              </Button>
              <Button variant="primary" disabled={working} onClick={() => choose("adapt")}>
                Adaptar ao projeto
              </Button>
            </>
          ) : (
            <Button variant="primary" disabled={working || !report} onClick={() => choose(null)}>
              Inserir
            </Button>
          )}
        </>
      }
    >
      <p className="hint">
        Base: {base.bpm.toLocaleString("pt-BR")} BPM · {keyLabel(base.key, catalog)} · {base.meter.numerator}/{base.meter.denominator}
      </p>
      <div className="row">
        <Field label="Tom do projeto">
          <select
            className="select"
            aria-label="Tom do projeto"
            value={encodeKey(projectKey)}
            onChange={(e) => {
              setProjectKey(decodeKey(e.target.value) ?? null);
              dispatch({ type: "recheck" });
            }}
          >
            <option value="">Não informado (compara só o BPM)</option>
            {allBaseKeys().map((k) => (
              <option key={encodeKey(k)} value={encodeKey(k)}>
                {keyLabel(k, catalog)}
              </option>
            ))}
          </select>
        </Field>
        <label className="check">
          <input type="checkbox" checked={onSelected} disabled={!selectedName} onChange={(e) => setOnSelected(e.target.checked)} />
          {selectedName ? `Na faixa selecionada (${selectedName})` : "Na faixa selecionada"}
        </label>
      </div>
      {flow.step === "checking" ? <p className="hint">Comparando com o projeto…</p> : null}
      {report ? (
        <div className={report.needsDecision ? "mismatch mismatch--warn" : "mismatch"} role="status" aria-label="Divergência">
          <strong>{report.needsDecision ? "A base diverge do projeto:" : "A base combina com o projeto."}</strong>
          <ul>
            {describeMismatch(report, base.bpm).map((l) => (
              <li key={l}>{l}</li>
            ))}
          </ul>
          {report.needsDecision ? <p className="hint">Como inserir? "Manter" usa a base como está; "Adaptar" ajusta andamento e tom ao projeto.</p> : null}
        </div>
      ) : null}
      {flow.step === "inserting" ? <p className="hint">Inserindo…</p> : null}
      {flow.step === "error" ? <p className="field-error">{flow.message}</p> : null}
    </Dialog>
  );
}
