import type { ReactNode } from "react";
import type { HarmonyKey, ProgressionView } from "../../ipc/types";
import { stopAudition, useAudition } from "../../core/audition";
import { useSession } from "../../core/session";
import { Button } from "../../shared/components";
import { auditionProgression, insertProgression, refId, type InsertTarget } from "./actions";
import type { Notation } from "./sequence";

/** Cartão de progressão: nome, gêneros, selo universal, acordes no tom, ouvir e inserir. */
export function ProgressionCard({
  p,
  cifra,
  harmonyKey,
  notation,
  target,
  genreName,
  extra,
}: {
  p: ProgressionView;
  cifra: string[] | null;
  harmonyKey: HarmonyKey;
  notation: Notation;
  target: InsertTarget;
  genreName: (g: string) => string;
  extra?: ReactNode;
}) {
  const id = refId(p.ref);
  const playing = useAudition((s) => s.playing === id || s.loading === id);
  const busy = useSession((s) => s.busy);
  const chords = notation === "cifra" && cifra ? cifra : p.roman;
  return (
    <li className="prog-card" aria-label={`Progressão ${p.name}`}>
      <div className="prog-main">
        <div className="prog-title">
          <strong title={p.name}>{p.name}</strong>
          {p.universal ? <span className="badge">universal</span> : null}
          {p.origin === "user" ? <span className="badge badge--user">minha</span> : null}
          <small>{p.genres.map(genreName).join(" · ")}</small>
        </div>
        <div className="prog-chords mono" aria-label="Acordes">
          {chords.map((c, i) => (
            <span key={i}>{c}</span>
          ))}
        </div>
      </div>
      <div className="prog-actions">
        {playing ? (
          <Button size="sm" onClick={stopAudition} aria-label={`Parar ${p.name}`}>
            ■ Parar
          </Button>
        ) : (
          <Button size="sm" onClick={() => void auditionProgression(p.ref, harmonyKey)} aria-label={`Ouvir ${p.name}`}>
            ▶ Ouvir
          </Button>
        )}
        <Button size="sm" variant="primary" disabled={busy} onClick={() => void insertProgression(p.ref, harmonyKey, target)} aria-label={`Inserir ${p.name} no arranjo`}>
          Inserir no arranjo
        </Button>
        {extra}
      </div>
    </li>
  );
}
