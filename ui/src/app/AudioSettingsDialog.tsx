import { useEffect, useState } from "react";
import { api } from "../ipc/api";
import type { AudioStatus, DeviceList } from "../ipc/types";
import { errorMessage } from "../core/session";
import { notify } from "../core/toasts";
import { Button, Dialog, Field } from "../shared/components";

/** Opção para um dispositivo configurado que não está mais conectado (o seletor não perde a escolha). */
function missing(list: { id: string }[] | undefined, id: string) {
  if (!id || !list || list.some((d) => d.id === id)) return null;
  return <option value={id}>{id} (desconectado)</option>;
}

export function AudioSettingsDialog({ open, onClose, onChanged }: { open: boolean; onClose: () => void; onChanged: (s: AudioStatus) => void }) {
  const [devices, setDevices] = useState<DeviceList | null>(null);
  const [out, setOut] = useState<string>("");
  const [inp, setInp] = useState<string>("");
  const [buffer, setBuffer] = useState<number>(256);
  const [busy, setBusy] = useState(false);
  // ao abrir: lista os dispositivos e mostra a configuração ATUAL ("" = padrão do sistema / nenhuma)
  useEffect(() => {
    if (!open) return;
    void Promise.all([api.audioDevices(), api.audioStatus()])
      .then(([d, s]) => {
        setDevices(d);
        setOut(s.outputId ?? "");
        setInp(s.inputId ?? "");
        if (s.bufferSize) setBuffer(s.bufferSize);
      })
      .catch((e) => notify.error(errorMessage(e)));
  }, [open]);
  const apply = async () => {
    setBusy(true);
    try {
      const s = await api.audioConfigure({ outputId: out || null, inputId: inp || null, bufferSize: buffer });
      onChanged(s);
      if (s.error) notify.error(`Dispositivo indisponível: ${s.error}. Usando saída silenciosa.`);
      else notify.success(`Áudio: ${s.outputName ?? "saída padrão"} · ${s.sampleRate} Hz · buffer ${s.bufferSize ?? "padrão"}`);
      onClose();
    } catch (e) {
      notify.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const latency = (buffer * 2 * 1000) / 48000;
  return (
    <Dialog
      title="Configurações de áudio"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" onClick={() => void apply()} disabled={busy}>
            {busy ? "Aplicando…" : "Aplicar"}
          </Button>
        </>
      }
    >
      <p style={{ margin: 0, color: "var(--fg-muted)" }}>Driver: {devices?.host ?? "…"}</p>
      <Field label="Saída">
        <select className="select" value={out} onChange={(e) => setOut(e.target.value)} aria-label="Dispositivo de saída">
          <option value="">Padrão do sistema</option>
          {missing(devices?.outputs, out)}
          {devices?.outputs.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
              {d.isDefault ? " (padrão)" : ""}
            </option>
          ))}
        </select>
      </Field>
      <Field label="Entrada (gravação)">
        <select className="select" value={inp} onChange={(e) => setInp(e.target.value)} aria-label="Dispositivo de entrada">
          <option value="">Nenhuma</option>
          {missing(devices?.inputs, inp)}
          {devices?.inputs.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
              {d.isDefault ? " (padrão)" : ""}
            </option>
          ))}
        </select>
      </Field>
      <Field label={`Buffer — ~${latency.toFixed(1)} ms de ida e volta a 48 kHz`}>
        <select className="select" value={buffer} onChange={(e) => setBuffer(Number(e.target.value))} aria-label="Tamanho do buffer">
          {[32, 64, 128, 256, 512, 1024, 2048].map((b) => (
            <option key={b} value={b}>
              {b} amostras{b <= 128 ? " (gravação)" : b >= 1024 ? " (mixagem pesada)" : ""}
            </option>
          ))}
        </select>
      </Field>
      <p style={{ margin: 0, fontSize: 12, color: "var(--fg-muted)" }}>Buffers menores reduzem a latência; se ouvir estalos, aumente o buffer.</p>
    </Dialog>
  );
}
