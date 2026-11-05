import { useCallback, useEffect, useMemo, useState } from "react";
import { compositionApi } from "../../ipc/api";
import type { BaseOptions, HarmonyKeysCatalog, LibraryBase, LibraryHistory } from "../../ipc/types";
import { audition, stopAudition, useAudition } from "../../core/audition";
import { errorMessage, useSession } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, EmptyState } from "../../shared/components";
import { EMPTY_FILTERS, encodeKey, filtersToQuery, type BaseFilters } from "./filters";
import { allBaseKeys, KIND_LABEL, keyLabel } from "./labels";
import { InsertBaseDialog } from "./InsertBaseDialog";
import { GenerateBaseDialog } from "./GenerateBaseDialog";
import { AddUserBaseDialog } from "./AddUserBaseDialog";
import { SoundFontDialog } from "./SoundFontDialog";
import "./library.css";

type DialogKind = "generate" | "add" | "soundfont" | null;

function BaseRow({ b, catalog, genreName, onInsert, onRemove }: { b: LibraryBase; catalog: HarmonyKeysCatalog | null; genreName: (c: string) => string; onInsert: () => void; onRemove: () => void }) {
  const id = `base:${b.id}`;
  const playing = useAudition((s) => s.playing === id || s.loading === id);
  const busy = useSession((s) => s.busy);
  const listen = async () => {
    try {
      await audition(id, () => compositionApi.basesAudition(b.id));
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };
  return (
    <tr aria-label={`Base ${b.name}`}>
      <td className="name" title={b.name}>
        {b.name}
        {b.tags.length ? <small>{b.tags.join(", ")}</small> : null}
      </td>
      <td>{KIND_LABEL[b.kind.kind]}</td>
      <td className="mono num">{b.bpm.toLocaleString("pt-BR")}</td>
      <td>{keyLabel(b.key, catalog)}</td>
      <td className="mono">
        {b.meter.numerator}/{b.meter.denominator}
      </td>
      <td>{b.genre ? genreName(b.genre) : "—"}</td>
      <td className="actions">
        {playing ? (
          <Button size="sm" onClick={stopAudition} aria-label={`Parar ${b.name}`}>
            ■
          </Button>
        ) : (
          <Button size="sm" onClick={() => void listen()} aria-label={`Ouvir ${b.name}`} title="Ouvir">
            ▶
          </Button>
        )}
        <Button size="sm" variant="primary" disabled={busy} onClick={onInsert} aria-label={`Inserir ${b.name}`}>
          Inserir
        </Button>
        <Button size="sm" variant="danger" disabled={b.kind.kind === "bundledCc0"} onClick={onRemove} aria-label={`Remover ${b.name}`} title={b.kind.kind === "bundledCc0" ? "Bases incluídas não podem ser removidas" : "Remover"}>
          ✕
        </Button>
      </td>
    </tr>
  );
}

/** Painel "Bases": busca, audição, geração, áudio do usuário, SoundFont e inserção com divergência. */
export function BasesPanel() {
  const [filters, setFilters] = useState<BaseFilters>(EMPTY_FILTERS);
  const [list, setList] = useState<LibraryBase[] | null>(null);
  const [history, setHistory] = useState<LibraryHistory | null>(null);
  const [options, setOptions] = useState<BaseOptions | null>(null);
  const [catalog, setCatalog] = useState<HarmonyKeysCatalog | null>(null);
  const [dialog, setDialog] = useState<DialogKind>(null);
  const [inserting, setInserting] = useState<LibraryBase | null>(null);
  const [revision, setRevision] = useState(0);
  const query = useMemo(() => filtersToQuery(filters), [filters]);

  useEffect(() => {
    Promise.all([compositionApi.basesOptions(), compositionApi.harmonyKeys()]).then(
      ([o, k]) => {
        setOptions(o);
        setCatalog(k);
      },
      (e) => notify.error(errorMessage(e)),
    );
  }, []);
  useEffect(() => {
    let alive = true;
    Promise.all([compositionApi.basesSearch(query), compositionApi.libraryHistory()]).then(
      ([l, h]) => {
        if (!alive) return;
        setList(l);
        setHistory(h);
      },
      (e) => alive && notify.error(errorMessage(e)),
    );
    return () => {
      alive = false;
    };
  }, [query, revision]);

  const reload = useCallback(() => setRevision((r) => r + 1), []);
  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (e) {
      notify.error(errorMessage(e));
    }
    reload();
  };
  const set = (patch: Partial<BaseFilters>) => setFilters((f) => ({ ...f, ...patch }));
  const genreName = (code: string) => catalog?.genres.find((g) => g.code === code)?.namePt ?? code;

  return (
    <div className="bases" aria-label="Bases">
      <form className="bases-filters" role="search" aria-label="Filtros de bases" onSubmit={(e) => e.preventDefault()}>
        <input className="input grow" type="search" placeholder="Buscar nome, tag, progressão…" aria-label="Buscar bases" value={filters.text} onChange={(e) => set({ text: e.target.value })} />
        <span className="bpm">
          <input className="input" inputMode="decimal" placeholder="BPM mín." aria-label="BPM mínimo" value={filters.bpmMin} onChange={(e) => set({ bpmMin: e.target.value })} />
          <span>–</span>
          <input className="input" inputMode="decimal" placeholder="máx." aria-label="BPM máximo" value={filters.bpmMax} onChange={(e) => set({ bpmMax: e.target.value })} />
        </span>
        <select className="select" aria-label="Filtrar por tom" value={filters.key} onChange={(e) => set({ key: e.target.value })}>
          <option value="">Qualquer tom</option>
          {allBaseKeys().map((k) => (
            <option key={encodeKey(k)} value={encodeKey(k)}>
              {keyLabel(k, catalog)}
            </option>
          ))}
        </select>
        <select className="select" aria-label="Filtrar por gênero" value={filters.genre} onChange={(e) => set({ genre: e.target.value })}>
          <option value="">Qualquer gênero</option>
          {(catalog?.genres ?? []).map((g) => (
            <option key={g.code} value={g.code}>
              {g.namePt}
            </option>
          ))}
        </select>
        <select className="select" aria-label="Filtrar por instrumento" value={filters.instrument} onChange={(e) => set({ instrument: e.target.value })}>
          <option value="">Qualquer instrumento</option>
          {(options?.instruments ?? []).map((i) => (
            <option key={i.value} value={i.value}>
              {i.namePt}
            </option>
          ))}
        </select>
        <select className="select" aria-label="Filtrar por tipo" value={filters.kind} onChange={(e) => set({ kind: e.target.value as BaseFilters["kind"] })}>
          <option value="">Qualquer tipo</option>
          <option value="generated">Geradas</option>
          <option value="userAudio">Áudios meus</option>
          <option value="bundledCc0">Incluídas (CC0)</option>
        </select>
        <input className="input tags" placeholder="tags, separadas, por vírgula" aria-label="Filtrar por tags" value={filters.tags} onChange={(e) => set({ tags: e.target.value })} />
        <Button size="sm" variant="ghost" onClick={() => setFilters(EMPTY_FILTERS)}>
          Limpar
        </Button>
      </form>
      <div className="bases-actions">
        <Button size="sm" variant="primary" onClick={() => setDialog("generate")}>
          Gerar base…
        </Button>
        <Button size="sm" onClick={() => setDialog("add")}>
          Adicionar áudio…
        </Button>
        <Button size="sm" onClick={() => setDialog("soundfont")}>
          SoundFont…
        </Button>
        <span className="grow" />
        <Button size="sm" disabled={!history?.canUndo} title={history?.undoLabel ?? undefined} onClick={() => void act(compositionApi.libraryUndo)} aria-label="Desfazer na biblioteca">
          ↶ {history?.undoLabel ? `Desfazer: ${history.undoLabel}` : "Desfazer"}
        </Button>
        <Button size="sm" disabled={!history?.canRedo} title={history?.redoLabel ?? undefined} onClick={() => void act(compositionApi.libraryRedo)} aria-label="Refazer na biblioteca">
          ↷ Refazer
        </Button>
      </div>
      <div className="bases-list">
        {list && list.length === 0 ? (
          <EmptyState title="Nenhuma base encontrada.">Ajuste os filtros ou gere uma base nova.</EmptyState>
        ) : (
          <table aria-label="Resultados de bases">
            <thead>
              <tr>
                <th>Nome</th>
                <th>Tipo</th>
                <th>BPM</th>
                <th>Tom</th>
                <th>Compasso</th>
                <th>Gênero</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {list?.map((b) => (
                <BaseRow key={b.id} b={b} catalog={catalog} genreName={genreName} onInsert={() => setInserting(b)} onRemove={() => void act(() => compositionApi.basesRemove(b.id))} />
              ))}
            </tbody>
          </table>
        )}
      </div>
      <GenerateBaseDialog
        open={dialog === "generate"}
        options={options}
        catalog={catalog}
        onClose={() => setDialog(null)}
        onSaved={() => {
          setDialog(null);
          reload();
        }}
      />
      <AddUserBaseDialog
        open={dialog === "add"}
        options={options}
        catalog={catalog}
        onClose={() => setDialog(null)}
        onSaved={() => {
          setDialog(null);
          reload();
        }}
      />
      <SoundFontDialog open={dialog === "soundfont"} onClose={() => setDialog(null)} />
      <InsertBaseDialog base={inserting} catalog={catalog} onClose={() => setInserting(null)} />
    </div>
  );
}
