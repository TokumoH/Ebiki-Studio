import { useEffect, useRef, useState } from "react";
import "./mixer.css";
import { api } from "../../ipc/api";
import type { ChannelStrip, EffectSlot, PresetInfo, StripTarget, BusView } from "../../ipc/types";
import { run, useSession, newGesture, endGesture } from "../../core/session";
import { useLive } from "../../core/live";
import { formatDb, formatPan } from "../../core/format";
import { Button, Dialog, Fader, InlineRename, Knob, Meter, Toggle } from "../../shared/components";
import { defaultEffect, EFFECT_LABELS } from "./effectsMeta";
import { EffectEditor } from "./EffectEditor";

function useGesture() {
  const g = useRef<number | null>(null);
  return {
    id: () => (g.current ??= newGesture()),
    end: () => {
      g.current = null;
      void endGesture();
    },
  };
}

interface StripProps {
  name: string;
  target: StripTarget;
  strip: ChannelStrip;
  peak: [number, number];
  buses: BusView[];
  selected?: boolean;
  onSelect?: () => void;
  isMaster?: boolean;
  /** Presente: o nome vira editável (clique/F2). */
  onRename?: (name: string) => void;
  /** Presente: mostra "remover", com confirmação. */
  onDelete?: () => void;
}

function StripView({ name, target, strip, peak, buses, selected, onSelect, isMaster, onRename, onDelete }: StripProps) {
  const vol = useGesture();
  const pan = useGesture();
  const [editing, setEditing] = useState<EffectSlot | null>(null);
  const [adding, setAdding] = useState(false);
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const isTrack = target.kind === "track";
  useEffect(() => {
    if (adding && presets.length === 0) void api.mixerPresets().then(setPresets).catch(() => undefined);
  }, [adding, presets.length]);
  const current = editing ? strip.inserts.find((i) => i.id === editing.id) : undefined;
  return (
    <div className={`strip${isMaster ? " strip--master" : ""}`} aria-selected={selected} aria-label={`Canal ${name}`} onPointerDown={onSelect}>
      {onRename || onDelete ? (
        <div className="strip-name strip-name--editable" title={name}>
          {onRename ? <InlineRename value={name} label={`Renomear ${name}`} inputLabel={`Novo nome de ${name}`} onCommit={onRename} /> : <span>{name}</span>}
          {onDelete ? (
            <Button size="sm" variant="ghost" icon aria-label={`Remover ${name}`} title="Remover bus" onClick={() => setConfirmDelete(true)}>
              ✕
            </Button>
          ) : null}
        </div>
      ) : (
        <div className="strip-name" title={name}>
          {name}
        </div>
      )}
      <div className="strip-inserts">
        {strip.inserts.map((s) => (
          <div key={s.id} className="insert" data-bypass={s.bypass} onClick={() => setEditing(s)} role="button" tabIndex={0} aria-label={`Editar ${EFFECT_LABELS[s.effect.type] ?? s.effect.type}`} onKeyDown={(e) => e.key === "Enter" && setEditing(s)}>
            <span>{EFFECT_LABELS[s.effect.type] ?? s.effect.type}</span>
          </div>
        ))}
        <Button size="sm" variant="ghost" onClick={() => setAdding(true)} aria-label={`Adicionar efeito em ${name}`}>
          + efeito
        </Button>
      </div>
      {isTrack ? (
        <>
          <select
            className="select"
            aria-label={`Saída de ${name}`}
            value={strip.output.kind === "bus" ? strip.output.id : "master"}
            onChange={(e) => void run(() => api.mixerEdit(target, { op: "output", value: e.target.value === "master" ? { kind: "master" } : { kind: "bus", id: e.target.value } }))}
          >
            <option value="master">→ Master</option>
            {buses.map((b) => (
              <option key={b.id} value={b.id}>
                → {b.name}
              </option>
            ))}
          </select>
          {buses.length > 0 ? (
            <select
              className="select"
              aria-label={`Envio de ${name}`}
              value=""
              onChange={(e) => {
                const bus = e.target.value;
                if (!bus) return;
                const existing = strip.sends.find((s) => s.bus === bus);
                void run(() => (existing ? api.mixerEdit(target, { op: "removeSend", id: existing.id }) : api.mixerEdit(target, { op: "addSend", send: { id: crypto.randomUUID(), bus, level: -12, preFader: false } })));
              }}
            >
              <option value="">Envios ({strip.sends.length})</option>
              {buses.map((b) => (
                <option key={b.id} value={b.id}>
                  {strip.sends.some((s) => s.bus === b.id) ? "✓ " : ""}
                  {b.name}
                </option>
              ))}
            </select>
          ) : null}
        </>
      ) : null}
      {!isMaster ? (
        <div style={{ display: "flex", justifyContent: "center" }}>
          <Knob label={`Pan ${name}`} value={strip.pan} min={-1} max={1} format={formatPan} onChange={(v, end) => (end ? pan.end() : void run(() => api.mixerEdit(target, { op: "pan", value: Math.round(v * 100) / 100 }, pan.id())))} />
        </div>
      ) : null}
      <div className="strip-fader">
        <Fader height={120} label={`Volume ${name}`} db={strip.volume} onChange={(db, end) => (end ? (vol.end(), void run(() => api.mixerEdit(target, { op: "volume", value: db }))) : void run(() => api.mixerEdit(target, { op: "volume", value: db }, vol.id())))} />
        <Meter height={120} label={`Nível ${name}`} peak={peak} />
      </div>
      <div className="strip-value mono">
        {formatDb(strip.volume)} dB · {formatPan(strip.pan)}
      </div>
      <div className="strip-buttons">
        <Toggle className="toggle-m" pressed={strip.mute} onChange={(v) => void run(() => api.mixerEdit(target, { op: "mute", value: v }))} aria-label={`Mute ${name}`}>
          M
        </Toggle>
        {!isMaster ? (
          <Toggle className="toggle-s" pressed={strip.solo} onChange={(v) => void run(() => api.mixerEdit(target, { op: "solo", value: v }))} aria-label={`Solo ${name}`}>
            S
          </Toggle>
        ) : null}
      </div>
      <Dialog title={`Adicionar efeito — ${name}`} open={adding} onClose={() => setAdding(false)} footer={<Button onClick={() => setAdding(false)}>Fechar</Button>}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))", gap: 8 }}>
          {Object.entries(EFFECT_LABELS).map(([type, label]) => (
            <Button key={type} onClick={() => void run(() => api.mixerEdit(target, { op: "addEffect", slot: { id: crypto.randomUUID(), bypass: false, effect: defaultEffect(type) }, index: null })).then(() => setAdding(false))}>
              {label}
            </Button>
          ))}
        </div>
        <h4 style={{ margin: "8px 0 0" }}>Presets por instrumento (substituem os efeitos atuais)</h4>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))", gap: 8 }}>
          {presets.map((p) => (
            <Button key={p.id} variant="ghost" onClick={() => void run(() => api.mixerApplyPreset(target, p.id)).then(() => setAdding(false))}>
              {p.label}
            </Button>
          ))}
        </div>
      </Dialog>
      <Dialog
        title={`Remover ${name}?`}
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        footer={
          <>
            <Button onClick={() => setConfirmDelete(false)}>Cancelar</Button>
            <Button
              variant="danger"
              onClick={() => {
                setConfirmDelete(false);
                onDelete?.();
              }}
            >
              Remover
            </Button>
          </>
        }
      >
        <p>As faixas que saem por “{name}” voltam para o Master e os envios para ele são apagados. Você pode desfazer com Ctrl+Z.</p>
      </Dialog>
      <Dialog title={`Efeito — ${name}`} open={!!current} onClose={() => setEditing(null)} footer={<Button onClick={() => setEditing(null)}>Fechar</Button>}>
        {current ? <EffectEditor target={target} slot={current} onClose={() => setEditing(null)} /> : null}
      </Dialog>
    </div>
  );
}

export function MixerPanel() {
  const session = useSession((s) => s.session);
  const selected = useSession((s) => s.selectedTrackId);
  const select = useSession((s) => s.select);
  const meters = useLive((s) => s.meters);
  if (!session) return null;
  const peakOf = (id: string, list: { id: string; peak: [number, number] }[] | undefined): [number, number] => list?.find((m) => m.id === id)?.peak ?? [0, 0];
  return (
    <div className="mixer" aria-label="Mixer">
      <div className="mixer-group">
        {session.tracks.map((t) => (
          <StripView key={t.id} name={t.name} target={{ kind: "track", id: t.id }} strip={t.strip} peak={peakOf(t.id, meters?.tracks)} buses={session.buses} selected={t.id === selected} onSelect={() => select({ trackId: t.id })} />
        ))}
      </div>
      <div className="mixer-sep" />
      <div className="mixer-group">
        {session.buses.map((b) => (
          <StripView
            key={b.id}
            name={b.name}
            target={{ kind: "bus", id: b.id }}
            strip={b.strip}
            peak={peakOf(b.id, meters?.buses)}
            buses={[]}
            onRename={(n) => void run(() => api.busRename(b.id, n))}
            onDelete={() => void run(() => api.busDelete(b.id))}
          />
        ))}
        <div style={{ display: "flex", flexDirection: "column", gap: 4, justifyContent: "flex-start" }}>
          <Button size="sm" onClick={() => void run(() => api.busCreate(`Bus ${session.buses.length + 1}`))}>
            + Bus
          </Button>
        </div>
      </div>
      <div className="mixer-sep" />
      <StripView name="Master" target={{ kind: "master" }} strip={session.master} peak={meters?.master ?? [0, 0]} buses={[]} isMaster />
    </div>
  );
}
