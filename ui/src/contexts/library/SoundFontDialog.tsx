import { useEffect, useState } from "react";
import { compositionApi } from "../../ipc/api";
import { transport } from "../../ipc/transport";
import type { SynthStatus } from "../../ipc/types";
import { errorMessage } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog } from "../../shared/components";

export function synthSummary(s: SynthStatus | null): string {
  if (!s) return "Carregando…";
  if (s.engine === "simple") return "Sintetizador simples (embutido)";
  return `SoundFont: ${s.bankName ?? "sem nome"} · ${s.presetCount} presets`;
}

/** Escolha do SoundFont (.sf2) usado para ouvir e renderizar progressões e bases geradas. */
export function SoundFontDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [status, setStatus] = useState<SynthStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  useEffect(() => {
    if (!open) return;
    setError(null);
    compositionApi.synthStatus().then(setStatus, (e) => setError(errorMessage(e)));
  }, [open]);
  const apply = async (path: string | null) => {
    setWorking(true);
    setError(null);
    try {
      const s = await compositionApi.synthSetSoundfont(path);
      setStatus(s);
      notify.success(s.engine === "soundfont" ? "SoundFont carregado." : "Usando o sintetizador simples.");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setWorking(false);
    }
  };
  const choose = async () => {
    const r = await transport().pickOpen({ title: "Escolher SoundFont", filters: [{ name: "SoundFont", extensions: ["sf2"] }] });
    if (r?.[0]) await apply(r[0]);
  };
  if (!open) return null;
  return (
    <Dialog
      title="Sintetizador (SoundFont)"
      open
      onClose={onClose}
      footer={<Button onClick={onClose}>Fechar</Button>}
    >
      <p role="status" aria-label="Estado do sintetizador">
        <strong>{synthSummary(status)}</strong>
      </p>
      {status?.soundfontPath ? (
        <p className="path" title={status.soundfontPath}>
          {status.soundfontPath}
        </p>
      ) : (
        <p className="hint">Sem SoundFont, a audição usa um sintetizador simples. Um arquivo .sf2 General MIDI dá timbres realistas.</p>
      )}
      <div className="row">
        <Button variant="primary" disabled={working} onClick={() => void choose()}>
          Escolher arquivo .sf2…
        </Button>
        <Button disabled={working || status?.engine === "simple"} onClick={() => void apply(null)}>
          Usar sintetizador simples
        </Button>
      </div>
      {error ? <p className="field-error">{error}</p> : null}
    </Dialog>
  );
}
