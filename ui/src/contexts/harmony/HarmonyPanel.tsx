import { useEffect, useMemo, useState } from "react";
import { compositionApi } from "../../ipc/api";
import type { HarmonicField, HarmonyKey, HarmonyKeysCatalog } from "../../ipc/types";
import { stopAudition, useAudition } from "../../core/audition";
import { useComposition } from "../../core/composition";
import { errorMessage, useSession } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Segmented, Tabs } from "../../shared/components";
import { auditionProgression, type InsertTarget } from "./actions";
import type { Notation } from "./sequence";
import { ProgressionList } from "./ProgressionList";
import { NextChordHelper } from "./NextChordHelper";
import { UserProgressions } from "./UserProgressions";
import "./harmony.css";

type Tab = "progressions" | "next" | "user";
type Size = "triad" | "seventh";

export function tonicLabel(t: { sharp: string; flat: string }): string {
  return t.sharp === t.flat ? t.sharp : `${t.sharp}/${t.flat}`;
}

/** Painel "Harmonia": campo harmônico, progressões por gênero, próximo acorde e progressões do usuário. */
export function HarmonyPanel() {
  const chosen = useComposition((s) => s.key);
  const setChosen = useComposition((s) => s.setKey);
  const tracks = useSession((s) => s.session?.tracks);
  const selectedTrackId = useSession((s) => s.selectedTrackId);
  const playing = useAudition((s) => s.playing ?? s.loading);
  const [catalog, setCatalog] = useState<HarmonyKeysCatalog | null>(null);
  const [field, setField] = useState<HarmonicField | null>(null);
  const [notation, setNotation] = useState<Notation>("cifra");
  const [size, setSize] = useState<Size>("triad");
  const [tab, setTab] = useState<Tab>("progressions");
  const [target, setTarget] = useState<InsertTarget>("new");
  const key: HarmonyKey = useMemo(() => ({ tonic: chosen.tonic, mode: chosen.mode }), [chosen.tonic, chosen.mode]);

  useEffect(() => {
    compositionApi.harmonyKeys().then(setCatalog, (e) => notify.error(errorMessage(e)));
  }, []);
  useEffect(() => {
    let alive = true;
    compositionApi.harmonyField(key).then(
      (f) => alive && setField(f),
      (e) => notify.error(errorMessage(e)),
    );
    return () => {
      alive = false;
    };
  }, [key]);

  const selectedTrack = tracks?.find((t) => t.id === selectedTrackId) ?? null;
  const setKey = (tonic: number, mode: string) => setChosen({ tonic, mode, isMinor: catalog?.modes.find((m) => m.mode === mode)?.isMinor ?? false });

  return (
    <div className="harmony" aria-label="Harmonia">
      <div className="harmony-bar">
        <label className="inline">
          <span>Tom</span>
          <select className="select" aria-label="Tônica" value={chosen.tonic} onChange={(e) => setKey(Number(e.target.value), chosen.mode)}>
            {(catalog?.tonics ?? []).map((t) => (
              <option key={t.value} value={t.value}>
                {tonicLabel(t)}
              </option>
            ))}
          </select>
        </label>
        <label className="inline">
          <span>Modo</span>
          <select className="select" aria-label="Modo" value={chosen.mode} onChange={(e) => setKey(chosen.tonic, e.target.value)}>
            {(catalog?.modes ?? []).map((m) => (
              <option key={m.mode} value={m.mode}>
                {m.namePt}
              </option>
            ))}
          </select>
        </label>
        <Segmented<Notation> label="Notação" value={notation} onChange={setNotation} options={[{ value: "cifra", label: "Cifra" }, { value: "roman", label: "Graus" }]} />
        <Segmented<Size> label="Acordes do campo" value={size} onChange={setSize} options={[{ value: "triad", label: "Tríades" }, { value: "seventh", label: "Tétrades" }]} />
        <span className="grow" />
        <Segmented<InsertTarget>
          label="Inserir em"
          value={target}
          onChange={setTarget}
          options={[
            { value: "new", label: "Nova faixa" },
            { value: "selected", label: selectedTrack ? `Faixa: ${selectedTrack.name}` : "Faixa selecionada" },
          ]}
        />
        <Button size="sm" onClick={stopAudition} disabled={!playing} aria-label="Parar audição">
          ■ Parar
        </Button>
      </div>
      <div className="harmony-field" role="list" aria-label={field ? `Campo harmônico de ${field.label}` : "Campo harmônico"}>
        {field?.degrees.map((d) => {
          const c = size === "triad" ? d.triad : d.seventh;
          return (
            <button
              key={d.degree}
              role="listitem"
              type="button"
              className="chord-chip"
              title={`Ouvir ${c.symbol} (${c.roman})`}
              onClick={() => void auditionProgression({ kind: "cifra", chords: `${c.symbol}:4` }, key)}
            >
              <strong>{notation === "cifra" ? c.symbol : c.roman}</strong>
              <small>{notation === "cifra" ? c.roman : c.symbol}</small>
            </button>
          );
        })}
      </div>
      <Tabs<Tab>
        label="Seções da harmonia"
        value={tab}
        onChange={setTab}
        tabs={[
          { id: "progressions", label: "Progressões" },
          { id: "next", label: "Próximo acorde" },
          { id: "user", label: "Minhas progressões" },
        ]}
      />
      <div className="harmony-body">
        {tab === "progressions" ? <ProgressionList harmonyKey={key} catalog={catalog} notation={notation} target={target} /> : null}
        {tab === "next" ? <NextChordHelper harmonyKey={key} notation={notation} target={target} /> : null}
        {tab === "user" ? <UserProgressions harmonyKey={key} catalog={catalog} notation={notation} target={target} /> : null}
      </div>
    </div>
  );
}
