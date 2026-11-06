// "Digitar notas" (mockup 8): texto corrido → prévia ao vivo → inserção em lote a partir do cursor.
import { useMemo, useState } from "react";
import type { NoteValue, ScoreEvent } from "../../ipc/types";
import { Button } from "../../shared/components";
import { parseNoteText, type ParseOptions } from "./noteText";
import { durationFrac, pitchLabel } from "./scoreEditing";

const SYMBOL: Record<NoteValue, string> = { whole: "𝅝", half: "𝅗𝅥", quarter: "♩", eighth: "♪", sixteenth: "𝅘𝅥𝅯", thirtySecond: "𝅘𝅥𝅰", sixtyFourth: "𝅘𝅥𝅱", oneTwentyEighth: "𝅘𝅥𝅲" };

function chip(e: ScoreEvent): string {
  const d = `${SYMBOL[e.duration.value]}${e.duration.dots ? "." : ""}`;
  return e.kind.type === "rest" ? `pausa ${d}` : `${e.kind.notes.map((n) => pitchLabel(n.pitch)).join("+")} ${d}`;
}

/** "3 compassos e meio" a partir da soma das durações e da capacidade do compasso (semibreves). */
export function measuresLabel(total: number, capacity: number): string {
  if (total <= 0 || capacity <= 0) return "";
  const m = total / capacity;
  const whole = Math.floor(m + 1e-9);
  const rest = m - whole;
  const frac = rest < 0.01 ? "" : Math.abs(rest - 0.5) < 0.01 ? " e meio" : rest < 0.5 ? " e pouco" : " e mais da metade";
  if (whole === 0) return Math.abs(rest - 0.5) < 0.01 ? "meio compasso" : "menos de um compasso";
  return `${whole} compasso${whole > 1 ? "s" : ""}${frac}`;
}

export function NoteTextEntry({ options, capacity, onInsert }: { options: ParseOptions; capacity: number; onInsert: (events: ScoreEvent[]) => Promise<boolean> }) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const result = useMemo(() => parseNoteText(text, options), [text, options]);
  const events = result.ok ? result.events : [];
  const total = events.reduce((a, e) => a + durationFrac(e.duration), 0);

  const insert = async () => {
    if (!result.ok || events.length === 0 || busy) return;
    setBusy(true);
    const ok = await onInsert(events);
    setBusy(false);
    if (ok) setText("");
  };

  return (
    <section className="note-text" aria-label="Digitar notas">
      <div className="note-text-head">
        <strong>Digitar notas</strong>
        <span className="hint">insere a partir do cursor; sem :duração usa a selecionada na barra</span>
      </div>
      <div className="note-text-row">
        <input
          className="input mono"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            e.stopPropagation(); // não aciona os atalhos do editor
            if (e.key === "Enter") {
              e.preventDefault();
              void insert();
            }
          }}
          placeholder="C, C, C, x, x, x, D, D, D, E5:8, F#:8, C+E+G:2"
          aria-label="Notas em texto"
          aria-invalid={!result.ok}
          aria-describedby="note-text-status"
          spellCheck={false}
          autoComplete="off"
        />
        <Button variant="primary" onClick={() => void insert()} disabled={!result.ok || events.length === 0 || busy}>
          {busy ? "Inserindo…" : "Inserir"}
        </Button>
      </div>
      {result.ok ? (
        events.length ? (
          <div className="note-text-preview" aria-label="Prévia">
            {events.slice(0, 64).map((e, i) => (
              <span key={i} className={e.kind.type === "rest" ? "chip rest" : "chip"}>
                {chip(e)}
              </span>
            ))}
            {events.length > 64 ? <span className="hint">+{events.length - 64}</span> : null}
            <span id="note-text-status" className="note-text-ok" role="status">
              ✓ {events.length} evento{events.length > 1 ? "s" : ""}
              {measuresLabel(total, capacity) ? ` · ${measuresLabel(total, capacity)}` : ""}
            </span>
          </div>
        ) : (
          <span id="note-text-status" className="hint">
            Notas A–G (ou Dó–Si), x = pausa, # b = acidentes, C5 = oitava, :8 = duração (:4. pontuada), C+E+G = acorde.
          </span>
        )
      ) : (
        <p id="note-text-status" className="note-text-error" role="alert">
          ✕ {result.error.message} Nada é inserido até corrigir.
        </p>
      )}
    </section>
  );
}
