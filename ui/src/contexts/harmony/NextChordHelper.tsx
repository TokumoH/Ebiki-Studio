import { useEffect, useState } from "react";
import { create } from "zustand";
import { compositionApi } from "../../ipc/api";
import type { ChordSuggestion, HarmonyKey } from "../../ipc/types";
import { stopAudition, useAudition } from "../../core/audition";
import { errorMessage, useSession } from "../../core/session";
import { Button } from "../../shared/components";
import { auditionProgression, insertProgression, refId, type InsertTarget } from "./actions";
import { appendChord, historyOf, label, removeLast, scoreBars, toCifraText, type Notation, type SequenceChord } from "./sequence";

/** Sequência em construção (sobrevive à troca de abas do painel). */
const useSequence = create<{ seq: SequenceChord[]; beats: number; set: (seq: SequenceChord[]) => void; setBeats: (b: number) => void }>((set) => ({
  seq: [],
  beats: 4,
  set: (seq) => set({ seq }),
  setBeats: (beats) => set({ beats }),
}));

/** Monta uma sequência clicando nos próximos acordes sugeridos (RF-5.4), com os motivos. */
export function NextChordHelper({ harmonyKey, notation, target }: { harmonyKey: HarmonyKey; notation: Notation; target: InsertTarget }) {
  const { seq, beats, set, setBeats } = useSequence();
  const [suggestions, setSuggestions] = useState<ChordSuggestion[]>([]);
  const [error, setError] = useState<string | null>(null);
  const busy = useSession((s) => s.busy);
  const text = toCifraText(seq, beats);
  const ref = { kind: "cifra" as const, chords: text };
  const playing = useAudition((s) => s.playing === refId(ref) || s.loading === refId(ref));
  const history = historyOf(seq).join(" ");

  useEffect(() => {
    let alive = true;
    compositionApi.harmonySuggestNext({ key: harmonyKey, history: history ? history.split(" ") : [], limit: 8 }).then(
      (s) => {
        if (!alive) return;
        setSuggestions(s);
        setError(null);
      },
      (e) => alive && setError(errorMessage(e)),
    );
    return () => {
      alive = false;
    };
  }, [harmonyKey, history]);

  const last = seq.at(-1);
  return (
    <div className="harmony-section">
      <div className="row seq-row">
        <span className="seq-label">Sequência:</span>
        <ol className="seq" aria-label="Sequência de acordes">
          {seq.length === 0 ? <li className="hint">vazia — escolha um acorde abaixo</li> : null}
          {seq.map((c, i) => (
            <li key={i} className="chord-pill">
              {label(c, notation)}
            </li>
          ))}
        </ol>
      </div>
      <div className="row">
        <label className="inline">
          <span>Tempos/acorde</span>
          <select className="select" aria-label="Tempos por acorde" value={beats} onChange={(e) => setBeats(Number(e.target.value))}>
            {[1, 2, 3, 4, 6, 8].map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
        </label>
        <Button size="sm" disabled={!seq.length} onClick={() => set(removeLast(seq))}>
          ↶ Remover último
        </Button>
        <Button size="sm" disabled={!seq.length} data-tip="harmony.clearSeq" onClick={() => set([])}>
          Limpar
        </Button>
        {playing ? (
          <Button size="sm" onClick={stopAudition} aria-label="Parar sequência">
            ■ Parar
          </Button>
        ) : (
          <Button size="sm" disabled={!seq.length} onClick={() => void auditionProgression(ref, harmonyKey)} aria-label="Ouvir sequência">
            ▶ Ouvir
          </Button>
        )}
        <Button size="sm" variant="primary" disabled={!seq.length || busy} onClick={() => void insertProgression(ref, harmonyKey, target)} aria-label="Inserir sequência no arranjo">
          Inserir no arranjo
        </Button>
      </div>
      <h4 className="sub">{last ? `Sugestões após ${label(last, notation)}` : "Sugestões para começar"}</h4>
      {error ? <p className="field-error">{error}</p> : null}
      <ul className="suggestions" aria-label="Próximos acordes sugeridos">
        {suggestions.map((s) => (
          <li key={`${s.symbol}-${s.roman}`}>
            <button type="button" className="suggestion" onClick={() => set(appendChord(seq, s))} title={`Adicionar ${s.symbol} (${s.roman})`} aria-label={`Adicionar ${s.symbol} — ${s.reasonLabel}`}>
              <strong>{notation === "cifra" ? s.symbol : s.roman}</strong>
              <small>{notation === "cifra" ? s.roman : s.symbol}</small>
              <span className="reason">{s.reasonLabel}</span>
              <span className="score" aria-label={`Força ${Math.round(s.score * 100)}%`}>
                {"▮".repeat(scoreBars(s.score))}
                <span className="faint">{"▮".repeat(5 - scoreBars(s.score))}</span>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
