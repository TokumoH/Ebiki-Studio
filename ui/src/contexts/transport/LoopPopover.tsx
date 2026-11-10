// Opções de loop (mockup 10): 4 compassos a partir do cursor, projeto inteiro ou compassos X até Y.
import { useEffect, useRef, useState } from "react";
import type { SessionView } from "../../ipc/types";
import { loopLabel, loopRangeBars, loopRangeFromCursor, projectLoopRange, tickToBar } from "../../core/bars";
import { useLive } from "../../core/live";
import { Button } from "../../shared/components";
import { setLoop, useLoopUi } from "../../core/loop";

type Choice = "cursor" | "project" | "bars";

const lastClipEnd = (s: SessionView) => Math.max(0, ...s.tracks.flatMap((t) => t.clips.map((c) => c.endTick)));

export function LoopPopover({ session, anchor }: { session: SessionView; anchor: HTMLElement | null }) {
  const setOpen = useLoopUi((u) => u.setOpen);
  const range = session.recording.loopRange;
  const projectRange = projectLoopRange(session.tempo, lastClipEnd(session));
  const [choice, setChoice] = useState<Choice>("bars");
  const [from, setFrom] = useState(() => (range ? tickToBar(session.tempo, range[0]) : 1));
  const [to, setTo] = useState(() => (range ? tickToBar(session.tempo, range[1] - 1) : 4));
  const box = useRef<HTMLDivElement>(null);

  // fecha com Esc ou clique fora (o botão ▾ alterna sozinho)
  useEffect(() => {
    box.current?.querySelector<HTMLElement>("input[type=radio]:checked")?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setOpen(false);
        anchor?.focus();
      }
    };
    const down = (e: PointerEvent) => {
      const t = e.target as Node;
      if (!box.current?.contains(t) && !anchor?.contains(t)) setOpen(false);
    };
    window.addEventListener("keydown", key, true);
    window.addEventListener("pointerdown", down, true);
    return () => {
      window.removeEventListener("keydown", key, true);
      window.removeEventListener("pointerdown", down, true);
    };
  }, [anchor, setOpen]);

  const target = (): [number, number] | null => {
    if (choice === "project") return projectRange;
    if (choice === "cursor") return loopRangeFromCursor(session.tempo, useLive.getState().transport?.tick ?? 0);
    return loopRangeBars(session.tempo, from, to);
  };
  const preview = target();

  const apply = async () => {
    const r = target();
    if (!r) return;
    if (await setLoop(r)) {
      setOpen(false);
      anchor?.focus();
    }
  };

  return (
    <div ref={box} className="loop-popover" role="dialog" aria-label="Opções de loop">
      <strong>Loop</strong>
      <div role="radiogroup" aria-label="Trecho do loop" className="loop-options">
        <label>
          <input type="radio" name="loop-choice" checked={choice === "cursor"} onChange={() => setChoice("cursor")} /> 4 compassos a partir do cursor
        </label>
        <label className={projectRange ? undefined : "disabled"}>
          <input type="radio" name="loop-choice" checked={choice === "project"} disabled={!projectRange} onChange={() => setChoice("project")} /> Projeto inteiro
          <small>{projectRange ? `compassos ${loopLabel(session.tempo, projectRange)}` : "sem clipes ainda"}</small>
        </label>
        <label className="loop-bars">
          <input type="radio" name="loop-choice" checked={choice === "bars"} onChange={() => setChoice("bars")} /> Compassos
          <input className="input num" type="number" min={1} value={from} aria-label="Primeiro compasso do loop" onFocus={() => setChoice("bars")} onChange={(e) => setFrom(Math.max(1, Math.round(Number(e.target.value)) || 1))} />
          até
          <input className="input num" type="number" min={1} value={to} aria-label="Último compasso do loop" onFocus={() => setChoice("bars")} onChange={(e) => setTo(Math.max(1, Math.round(Number(e.target.value)) || 1))} onKeyDown={(e) => e.key === "Enter" && void apply()} />
        </label>
      </div>
      <p className="hint">Dica: arraste na faixa de cima da régua para desenhar o loop; arraste as bordas para ajustar. L liga/desliga.</p>
      <p className="hint loop-note">Na gravação o loop é ignorado (grava em linha reta). Para regravar um trecho, use o punch in/out no painel Gravação.</p>
      <div className="loop-actions">
        <Button size="sm" disabled={!range} onClick={() => void setLoop(null).then((ok) => ok && setOpen(false))}>
          Desligar
        </Button>
        <Button size="sm" variant="primary" disabled={!preview} onClick={() => void apply()}>
          Aplicar{preview ? ` (${loopLabel(session.tempo, preview)})` : ""}
        </Button>
      </div>
    </div>
  );
}
