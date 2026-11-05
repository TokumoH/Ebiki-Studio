import { useEffect, useState } from "react";
import { compositionApi } from "../../ipc/api";
import type { HarmonyKey, HarmonyKeysCatalog, ProgressionSuggestion } from "../../ipc/types";
import { errorMessage } from "../../core/session";
import { EmptyState } from "../../shared/components";
import type { InsertTarget } from "./actions";
import type { Notation } from "./sequence";
import { ProgressionCard } from "./ProgressionCard";

export function genreNamer(catalog: HarmonyKeysCatalog | null) {
  return (g: string) => catalog?.genres.find((x) => x.genre === g)?.namePt ?? g;
}

/** Progressões ranqueadas para o tom e o gênero escolhidos (RF-5.3). */
export function ProgressionList({ harmonyKey, catalog, notation, target }: { harmonyKey: HarmonyKey; catalog: HarmonyKeysCatalog | null; notation: Notation; target: InsertTarget }) {
  const [genre, setGenre] = useState("");
  const [list, setList] = useState<ProgressionSuggestion[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    compositionApi.harmonyProgressions({ key: harmonyKey, genre: genre || null, limit: 30 }).then(
      (l) => {
        if (!alive) return;
        setList(l);
        setError(null);
      },
      (e) => alive && setError(errorMessage(e)),
    );
    return () => {
      alive = false;
    };
  }, [harmonyKey, genre]);
  const genreName = genreNamer(catalog);
  return (
    <div className="harmony-section">
      <div className="row">
        <label className="inline">
          <span>Gênero</span>
          <select className="select" aria-label="Gênero" value={genre} onChange={(e) => setGenre(e.target.value)}>
            <option value="">Todos os gêneros</option>
            {(catalog?.genres ?? []).map((g) => (
              <option key={g.genre} value={g.genre}>
                {g.namePt}
              </option>
            ))}
          </select>
        </label>
        <small className="hint">{list ? `${list.length} progressões` : "Carregando…"}</small>
      </div>
      {error ? <p className="field-error">{error}</p> : null}
      {list && list.length === 0 ? <EmptyState title="Nenhuma progressão para este tom e gênero." /> : null}
      <ul className="prog-list" aria-label="Progressões sugeridas">
        {list?.map((s) => (
          <ProgressionCard key={JSON.stringify(s.progression.ref)} p={s.progression} cifra={s.cifra} harmonyKey={harmonyKey} notation={notation} target={target} genreName={genreName} />
        ))}
      </ul>
    </div>
  );
}
