import { useEffect, useState } from "react";
import { compositionApi } from "../../ipc/api";
import type { BaseOptions, GenerateBaseRequest, GeneratedBase, HarmonyKeysCatalog } from "../../ipc/types";
import { audition, stopAudition, useAudition } from "../../core/audition";
import { useComposition } from "../../core/composition";
import { errorMessage } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog, Field, Segmented } from "../../shared/components";
import { parseTags } from "./filters";
import { tonicName } from "./labels";

type ProgKind = "cifra" | "roman";

const DEFAULT_TEXT: Record<ProgKind, string> = { cifra: "C:4 G:4 Am:4 F:4", roman: "I:4 V:4 vi:4 IV:4" };

/** Gera uma base (progressão + estilo + bateria + baixo), ouve e salva a receita na biblioteca. */
export function GenerateBaseDialog({ open, options, catalog, onClose, onSaved }: { open: boolean; options: BaseOptions | null; catalog: HarmonyKeysCatalog | null; onClose: () => void; onSaved: () => void }) {
  const chosen = useComposition((s) => s.key);
  const [kind, setKind] = useState<ProgKind>("roman");
  const [text, setText] = useState(DEFAULT_TEXT.roman);
  const [tonic, setTonic] = useState(chosen.tonic);
  const [minor, setMinor] = useState(chosen.isMinor);
  const [bpm, setBpm] = useState(100);
  const [bars, setBars] = useState(8);
  const [style, setStyle] = useState("popStrum");
  const [drums, setDrums] = useState("");
  const [bass, setBass] = useState("");
  const [name, setName] = useState("Minha base");
  const [genre, setGenre] = useState("");
  const [tags, setTags] = useState("");
  const [result, setResult] = useState<GeneratedBase | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const playing = useAudition((s) => s.playing === "generated" || s.loading === "generated");

  useEffect(() => {
    if (!open) return;
    setTonic(chosen.tonic);
    setMinor(chosen.isMinor);
    setResult(null);
    setError(null);
  }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  const mode = minor ? "aeolian" : "ionian";
  const request = (): GenerateBaseRequest => ({
    progression: kind === "cifra" ? { kind: "cifra", chords: text } : { kind: "roman", steps: text, mode },
    key: { tonic, mode },
    bpm,
    bars,
    style,
    drums: drums || null,
    bass: bass || null,
  });
  const invalidate = () => setResult(null);

  const run = async <T,>(fn: () => Promise<T>): Promise<T | null> => {
    setWorking(true);
    setError(null);
    try {
      return await fn();
    } catch (e) {
      setError(errorMessage(e));
      return null;
    } finally {
      setWorking(false);
    }
  };
  const generate = () => void run(() => compositionApi.basesGenerate(request())).then((r) => r && setResult(r));
  const listen = () =>
    void run(async () => {
      await audition("generated", () => compositionApi.basesGeneratedAudition(request()));
      return true;
    });
  const save = () =>
    void run(() => compositionApi.basesSaveGenerated({ name, generation: request(), genre: genre || null, tags: parseTags(tags) })).then((b) => {
      if (!b) return;
      notify.success(`Base "${b.name}" salva na biblioteca.`);
      onSaved();
    });

  if (!open) return null;
  const styleMeter = options?.styles.find((s) => s.value === style)?.meterNumerator ?? 4;
  const fits = (m: number | null) => m === null || m === styleMeter;
  return (
    <Dialog
      title="Gerar base"
      open
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button disabled={working} onClick={generate}>
            Gerar
          </Button>
          {playing ? (
            <Button onClick={stopAudition}>■ Parar</Button>
          ) : (
            <Button disabled={working} onClick={listen} aria-label="Ouvir base gerada">
              ▶ Ouvir
            </Button>
          )}
          <Button variant="primary" disabled={working || !name.trim()} onClick={save}>
            Salvar na biblioteca
          </Button>
        </>
      }
    >
      <div className="row">
        <Segmented<ProgKind>
          label="Tipo de progressão"
          value={kind}
          onChange={(k) => {
            setKind(k);
            setText(DEFAULT_TEXT[k]);
            invalidate();
          }}
          options={[
            { value: "roman", label: "Graus" },
            { value: "cifra", label: "Cifra" },
          ]}
        />
      </div>
      <Field label={kind === "cifra" ? "Progressão em cifra (acorde:tempos)" : "Progressão em graus (grau:tempos)"}>
        <input
          className="input mono"
          aria-label="Progressão"
          placeholder={DEFAULT_TEXT[kind]}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            invalidate();
          }}
        />
      </Field>
      <div className="form-grid">
        <Field label="Tônica">
          <select className="select" aria-label="Tônica da base" value={tonic} onChange={(e) => (setTonic(Number(e.target.value)), invalidate())}>
            {Array.from({ length: 12 }, (_, i) => (
              <option key={i} value={i}>
                {tonicName(i, catalog)}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Modo">
          <select className="select" aria-label="Modo da base" value={minor ? "minor" : "major"} onChange={(e) => (setMinor(e.target.value === "minor"), invalidate())}>
            <option value="major">Maior</option>
            <option value="minor">Menor</option>
          </select>
        </Field>
        <Field label="BPM">
          <input className="input" type="number" aria-label="BPM da base" min={20} max={999} value={bpm} onChange={(e) => (setBpm(Number(e.target.value)), invalidate())} />
        </Field>
        <Field label="Compassos">
          <input className="input" type="number" aria-label="Compassos" min={1} max={256} value={bars} onChange={(e) => (setBars(Number(e.target.value)), invalidate())} />
        </Field>
        <Field label="Estilo">
          <select className="select" aria-label="Estilo" value={style} onChange={(e) => (setStyle(e.target.value), setDrums(""), setBass(""), invalidate())}>
            {(options?.styles ?? []).map((s) => (
              <option key={s.value} value={s.value}>
                {s.namePt}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Bateria">
          <select className="select" aria-label="Bateria" value={drums} onChange={(e) => (setDrums(e.target.value), invalidate())}>
            <option value="">Automática (do estilo)</option>
            {(options?.drums ?? []).filter((d) => fits(d.meterNumerator)).map((d) => (
              <option key={d.value} value={d.value}>
                {d.namePt}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Baixo">
          <select className="select" aria-label="Baixo" value={bass} onChange={(e) => (setBass(e.target.value), invalidate())}>
            <option value="">Automático (do estilo)</option>
            {(options?.basses ?? []).filter((b) => fits(b.meterNumerator)).map((b) => (
              <option key={b.value} value={b.value}>
                {b.namePt}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Gênero">
          <select className="select" aria-label="Gênero da base" value={genre} onChange={(e) => setGenre(e.target.value)}>
            <option value="">—</option>
            {(catalog?.genres ?? []).map((g) => (
              <option key={g.code} value={g.code}>
                {g.namePt}
              </option>
            ))}
          </select>
        </Field>
      </div>
      <div className="form-grid">
        <Field label="Nome">
          <input className="input" aria-label="Nome da base" maxLength={64} value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label="Tags (vírgula)">
          <input className="input" aria-label="Tags da base" value={tags} onChange={(e) => setTags(e.target.value)} />
        </Field>
      </div>
      {result ? (
        <p className="gen-result" role="status">
          Gerada: {result.meter.numerator}/{result.meter.denominator} · {bars} compassos · {result.events.length} notas · receita <span className="mono">{result.recipeProgression}</span>
        </p>
      ) : null}
      {error ? <p className="field-error">{error}</p> : null}
    </Dialog>
  );
}
