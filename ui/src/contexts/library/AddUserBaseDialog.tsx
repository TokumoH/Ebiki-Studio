import { useEffect, useState } from "react";
import { compositionApi } from "../../ipc/api";
import { transport } from "../../ipc/transport";
import type { BaseOptions, HarmonyKeysCatalog } from "../../ipc/types";
import { errorMessage } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog, Field } from "../../shared/components";
import { decodeKey, encodeKey, parseTags } from "./filters";
import { allBaseKeys, keyLabel } from "./labels";

export const AUDIO_EXTENSIONS = ["wav", "flac", "mp3", "ogg", "aiff", "aif"];

/** Adiciona um áudio do usuário como base (o arquivo é copiado para a biblioteca). */
export function AddUserBaseDialog({ open, options, catalog, onClose, onSaved }: { open: boolean; options: BaseOptions | null; catalog: HarmonyKeysCatalog | null; onClose: () => void; onSaved: () => void }) {
  const [path, setPath] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [bpm, setBpm] = useState(120);
  const [key, setKey] = useState("");
  const [num, setNum] = useState(4);
  const [den, setDen] = useState(4);
  const [genre, setGenre] = useState("");
  const [instruments, setInstruments] = useState<string[]>([]);
  const [tags, setTags] = useState("");
  const [progression, setProgression] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    setPath(null);
    setName("");
    setError(null);
  }, [open]);

  const pick = async () => {
    try {
      const r = await transport().pickOpen({ title: "Escolher áudio da base", filters: [{ name: "Áudio", extensions: AUDIO_EXTENSIONS }] });
      const p = r?.[0];
      if (!p) return;
      setPath(p);
      if (!name) setName((p.split(/[\\/]/).pop() ?? "").replace(/\.[^.]+$/, "").slice(0, 64));
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const save = async () => {
    if (!path) return;
    setSaving(true);
    setError(null);
    try {
      const b = await compositionApi.basesAddUser(path, {
        name,
        bpm,
        key: decodeKey(key) ?? null,
        meter: { numerator: num, denominator: den },
        genre: genre || null,
        instruments,
        tags: parseTags(tags),
        progression: progression.trim() || null,
      });
      notify.success(`Base "${b.name}" adicionada.`);
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  };

  if (!open) return null;
  return (
    <Dialog
      title="Adicionar áudio como base"
      open
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" disabled={!path || !name.trim() || saving} onClick={() => void save()}>
            Adicionar
          </Button>
        </>
      }
    >
      <div className="row">
        <Button onClick={() => void pick()}>Escolher arquivo…</Button>
        <span className="path" title={path ?? undefined}>
          {path ?? "Nenhum arquivo escolhido (WAV, FLAC, MP3, OGG, AIFF)"}
        </span>
      </div>
      <div className="form-grid">
        <Field label="Nome">
          <input className="input" aria-label="Nome do áudio" maxLength={64} value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label="BPM">
          <input className="input" type="number" aria-label="BPM do áudio" min={20} max={999} value={bpm} onChange={(e) => setBpm(Number(e.target.value))} />
        </Field>
        <Field label="Tom">
          <select className="select" aria-label="Tom do áudio" value={key} onChange={(e) => setKey(e.target.value)}>
            <option value="">Desconhecido</option>
            {allBaseKeys().map((k) => (
              <option key={encodeKey(k)} value={encodeKey(k)}>
                {keyLabel(k, catalog)}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Compasso">
          <div className="row nowrap">
            <input className="input" type="number" aria-label="Tempos do compasso" min={1} max={16} value={num} onChange={(e) => setNum(Number(e.target.value))} style={{ width: 56 }} />
            <span>/</span>
            <select className="select" aria-label="Figura do compasso" value={den} onChange={(e) => setDen(Number(e.target.value))}>
              {[2, 4, 8, 16].map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          </div>
        </Field>
        <Field label="Gênero">
          <select className="select" aria-label="Gênero do áudio" value={genre} onChange={(e) => setGenre(e.target.value)}>
            <option value="">—</option>
            {(catalog?.genres ?? []).map((g) => (
              <option key={g.code} value={g.code}>
                {g.namePt}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Tags (vírgula)">
          <input className="input" aria-label="Tags do áudio" value={tags} onChange={(e) => setTags(e.target.value)} />
        </Field>
      </div>
      <Field label="Progressão anotada (opcional)">
        <input className="input mono" aria-label="Progressão do áudio" value={progression} onChange={(e) => setProgression(e.target.value)} />
      </Field>
      <fieldset className="pick-list">
        <legend>Instrumentos</legend>
        {(options?.instruments ?? []).map((i) => (
          <label key={i.value}>
            <input type="checkbox" checked={instruments.includes(i.value)} onChange={(e) => setInstruments(e.target.checked ? [...instruments, i.value] : instruments.filter((x) => x !== i.value))} />
            {i.namePt}
          </label>
        ))}
      </fieldset>
      {error ? <p className="field-error">{error}</p> : null}
    </Dialog>
  );
}
