import { useEffect, useState } from "react";
import { api } from "../ipc/api";
import { transport } from "../ipc/transport";
import type { ExportFormat, ExportSettings } from "../ipc/types";
import { errorMessage, useSession } from "../core/session";
import { notify } from "../core/toasts";
import { Button, Dialog, Field, Segmented, Toggle } from "../shared/components";

const LABELS: Record<ExportFormat, string> = { wav: "WAV", aiff: "AIFF", flac: "FLAC", mp3: "MP3", ogg: "OGG", aac: "AAC" };

export function ExportDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const session = useSession((s) => s.session);
  const [formats, setFormats] = useState<ExportFormat[]>(["wav"]);
  const [s, setS] = useState<ExportSettings>({ format: "wav", bitDepth: 24, mp3Kbps: 320, vorbisQuality: 0.6, normalize: false });
  const [stems, setStems] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (open) void api.exportFormats().then(setFormats).catch(() => undefined);
  }, [open]);
  if (!session) return null;
  const lossless = s.format === "wav" || s.format === "aiff" || s.format === "flac";
  const go = async () => {
    setBusy(true);
    try {
      if (stems) {
        const dir = (await transport().pickOpen({ directory: true, title: "Pasta para os stems" }))?.[0];
        if (!dir) return;
        const files = await api.exportStems(dir, s);
        notify.success(`${files.length} stems exportados.`);
      } else {
        const path = await transport().pickSave({ defaultPath: `${session.project.name}.${s.format === "ogg" ? "ogg" : s.format}`, filters: [{ name: LABELS[s.format], extensions: [s.format] }], title: "Exportar mixagem" });
        if (!path) return;
        await api.exportMix(path, s, null);
        notify.success("Mixagem exportada.");
      }
      onClose();
    } catch (e) {
      notify.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      title="Exportar"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" onClick={() => void go()} disabled={busy || session.contentEndFrame === 0}>
            {busy ? "Renderizando…" : "Exportar"}
          </Button>
        </>
      }
    >
      {session.contentEndFrame === 0 ? <p>O projeto ainda não tem áudio para exportar.</p> : null}
      <Field label="Formato" tip="export.format">
        <Segmented label="Formato" value={s.format} onChange={(f) => setS({ ...s, format: f, bitDepth: f === "flac" && s.bitDepth === 32 ? 24 : s.bitDepth })} options={formats.map((f) => ({ value: f, label: LABELS[f] }))} />
      </Field>
      {lossless ? (
        <Field label="Resolução" tip="export.bitDepth">
          <Segmented label="Resolução" value={s.bitDepth} onChange={(b) => setS({ ...s, bitDepth: b })} options={[{ value: 16, label: "16 bits" }, { value: 24, label: "24 bits" }, ...(s.format === "flac" ? [] : [{ value: 32, label: "32 float" }])]} />
        </Field>
      ) : null}
      {s.format === "mp3" ? (
        <Field label="Qualidade MP3" tip="export.quality">
          <select className="select" value={s.mp3Kbps ?? 0} onChange={(e) => setS({ ...s, mp3Kbps: Number(e.target.value) || null })}>
            {[320, 256, 192, 160, 128].map((k) => (
              <option key={k} value={k}>
                CBR {k} kbps
              </option>
            ))}
            <option value={0}>VBR V0</option>
          </select>
        </Field>
      ) : null}
      {s.format === "ogg" ? (
        <Field label={`Qualidade Vorbis (${s.vorbisQuality.toFixed(1)})`}>
          <input type="range" min={-0.2} max={1} step={0.1} value={s.vorbisQuality} onChange={(e) => setS({ ...s, vorbisQuality: Number(e.target.value) })} />
        </Field>
      ) : null}
      <div className="row">
        <Toggle pressed={s.normalize} onChange={(v) => setS({ ...s, normalize: v })} data-tip="export.normalize">
          Normalizar (−1 dBFS)
        </Toggle>
        <Toggle pressed={stems} onChange={setStems} data-tip="export.stems">
          Uma faixa por arquivo (stems)
        </Toggle>
      </div>
    </Dialog>
  );
}
