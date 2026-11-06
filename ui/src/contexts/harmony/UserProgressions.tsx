import { useCallback, useEffect, useState } from "react";
import { compositionApi } from "../../ipc/api";
import type { HarmonyKey, HarmonyKeysCatalog, LibraryHistory, ProgressionView, UserProgressionInput } from "../../ipc/types";
import { errorMessage } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog, EmptyState, Field, UndoRedoButtons } from "../../shared/components";
import type { InsertTarget } from "./actions";
import type { Notation } from "./sequence";
import { ProgressionCard } from "./ProgressionCard";
import { genreNamer } from "./ProgressionList";

interface Draft {
  id: string | null;
  name: string;
  mode: string;
  beatsPerBar: number;
  steps: string;
  genres: string[];
}

const EMPTY = (mode: string): Draft => ({ id: null, name: "", mode, beatsPerBar: 4, steps: "", genres: [] });

function UserProgressionDialog({ draft, catalog, onClose, onSaved }: { draft: Draft | null; catalog: HarmonyKeysCatalog | null; onClose: () => void; onSaved: () => void }) {
  const [d, setD] = useState<Draft | null>(draft);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    setD(draft);
    setError(null);
  }, [draft]);
  if (!d) return null;
  const save = async () => {
    const input: UserProgressionInput = { name: d.name, mode: d.mode, genres: d.genres, beatsPerBar: d.beatsPerBar, steps: d.steps };
    setSaving(true);
    try {
      if (d.id) await compositionApi.harmonyUserProgressionEdit(d.id, input);
      else await compositionApi.harmonyUserProgressionAdd(input);
      notify.success(d.id ? "Progressão atualizada." : "Progressão cadastrada.");
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  };
  const valid = d.name.trim().length > 0 && d.steps.trim().length > 0;
  return (
    <Dialog
      title={d.id ? "Editar progressão" : "Nova progressão"}
      open
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" disabled={!valid || saving} onClick={() => void save()}>
            Salvar
          </Button>
        </>
      }
    >
      <Field label="Nome">
        <input className="input" aria-label="Nome da progressão" maxLength={64} value={d.name} onChange={(e) => setD({ ...d, name: e.target.value })} />
      </Field>
      <div className="row">
        <Field label="Modo">
          <select className="select" aria-label="Modo da progressão" value={d.mode} onChange={(e) => setD({ ...d, mode: e.target.value })}>
            {(catalog?.modes ?? []).map((m) => (
              <option key={m.mode} value={m.mode}>
                {m.namePt}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Tempos por compasso">
          <input className="input" type="number" min={1} max={16} value={d.beatsPerBar} onChange={(e) => setD({ ...d, beatsPerBar: Math.min(16, Math.max(1, Number(e.target.value) || 4)) })} style={{ width: 90 }} />
        </Field>
      </div>
      <Field label="Graus romanos (grau ou grau:tempos)" error={error}>
        <input className="input mono" aria-label="Graus" placeholder="I VI7 ii7 V7  ·  ii7:2 V7:2 Imaj7:8" value={d.steps} onChange={(e) => setD({ ...d, steps: e.target.value })} />
      </Field>
      <fieldset className="genre-pick">
        <legend>Gêneros</legend>
        {(catalog?.genres ?? []).map((g) => (
          <label key={g.genre}>
            <input
              type="checkbox"
              checked={d.genres.includes(g.genre)}
              onChange={(e) => setD({ ...d, genres: e.target.checked ? [...d.genres, g.genre] : d.genres.filter((x) => x !== g.genre) })}
            />
            {g.namePt}
          </label>
        ))}
      </fieldset>
    </Dialog>
  );
}

/** Progressões do usuário: cadastrar, editar, remover, com desfazer/refazer da biblioteca. */
export function UserProgressions({ harmonyKey, catalog, notation, target }: { harmonyKey: HarmonyKey; catalog: HarmonyKeysCatalog | null; notation: Notation; target: InsertTarget }) {
  const [list, setList] = useState<ProgressionView[]>([]);
  const [history, setHistory] = useState<LibraryHistory | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const reload = useCallback(async () => {
    try {
      const [l, h] = await Promise.all([compositionApi.harmonyUserProgressions(), compositionApi.libraryHistory()]);
      setList(l);
      setHistory(h);
    } catch (e) {
      notify.error(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    void reload();
  }, [reload]);
  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      notify.error(errorMessage(e));
    }
    await reload();
  };
  const genreName = genreNamer(catalog);
  return (
    <div className="harmony-section">
      <div className="row">
        <Button size="sm" variant="primary" onClick={() => setDraft(EMPTY(harmonyKey.mode))}>
          + Nova progressão
        </Button>
        <UndoRedoButtons history={history} scope="na biblioteca" onUndo={() => void act(compositionApi.libraryUndo)} onRedo={() => void act(compositionApi.libraryRedo)} />
      </div>
      {list.length === 0 ? (
        <EmptyState title="Nenhuma progressão sua ainda.">Cadastre progressões em graus romanos; elas aparecem nas sugestões dos gêneros escolhidos.</EmptyState>
      ) : (
        <ul className="prog-list" aria-label="Minhas progressões">
          {list.map((p) => (
            <ProgressionCard
              key={JSON.stringify(p.ref)}
              p={p}
              cifra={null}
              harmonyKey={harmonyKey}
              notation={notation}
              target={target}
              genreName={genreName}
              extra={
                <>
                  <Button
                    size="sm"
                    aria-label={`Editar ${p.name}`}
                    onClick={() => p.ref.kind === "user" && setDraft({ id: p.ref.id, name: p.name, mode: p.mode, beatsPerBar: p.beatsPerBar, steps: p.stepsText, genres: p.genres })}
                  >
                    Editar
                  </Button>
                  <Button size="sm" variant="danger" aria-label={`Remover ${p.name}`} onClick={() => p.ref.kind === "user" && void act(() => compositionApi.harmonyUserProgressionRemove((p.ref as { id: string }).id))}>
                    Remover
                  </Button>
                </>
              }
            />
          ))}
        </ul>
      )}
      <UserProgressionDialog
        draft={draft}
        catalog={catalog}
        onClose={() => setDraft(null)}
        onSaved={() => {
          setDraft(null);
          void reload();
        }}
      />
    </div>
  );
}
