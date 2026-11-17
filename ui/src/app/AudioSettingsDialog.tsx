// "Configurações de áudio" (S1 da varredura 5, mockup aprovado — prancheta 1): separa com clareza o
// que você OUVE (fone / alto-falantes — saída) do que você GRAVA (microfone / interface — entrada).
// É o único lugar onde se escolhem os dispositivos; o painel Gravação abre este mesmo diálogo.
import { useEffect, useRef, useState } from "react";
import { api } from "../ipc/api";
import type { AudioStatus, DeviceList, DeviceMonitor } from "../ipc/types";
import { setInputGain, useAudio, useAudioSettingsUi } from "../core/audio";
import { errorMessage } from "../core/session";
import { notify } from "../core/toasts";
import { Button, Dialog, Field } from "../shared/components";
import { InputMeters } from "../contexts/recording/InputMeters";
import "./audioSettings.css";
import { formatDecimal } from "../core/format";

/** Faixa do ganho de entrada (dB), duração do tom de teste (ms) e canal padrão do retorno (entrada 1, mono). */
export const INPUT_GAIN_MAX_DB = 24;
const TONE_MS = 1000;
const MONITOR_ROUTE: Pick<DeviceMonitor, "left" | "right"> = { left: 0, right: null };

/** Opção para um dispositivo configurado que não está mais conectado (o seletor não perde a escolha). */
function missing(list: { id: string }[] | undefined, id: string) {
  if (!id || !list || list.some((d) => d.id === id)) return null;
  return <option value={id}>{id} (desconectado)</option>;
}

/** Latência de ida e volta (ms) para `buffer` amostras na taxa REAL do dispositivo. */
export function roundTripMs(buffer: number, sampleRate: number): number {
  return (buffer * 2 * 1000) / (sampleRate > 0 ? sampleRate : 48000);
}

/** "48 kHz", "44,1 kHz". */
export function formatRate(sampleRate: number): string {
  return `${(sampleRate / 1000).toLocaleString("pt-BR")} kHz`;
}

const signed = (db: number) => `${db > 0 ? "+" : ""}${db} dB`;

function HeadphonesIcon() {
  return (
    <svg className="audio-section-icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M4 15v-3a8 8 0 0 1 16 0v3" />
      <rect x="3" y="14" width="4" height="7" rx="1.5" />
      <rect x="17" y="14" width="4" height="7" rx="1.5" />
    </svg>
  );
}

function MicIcon() {
  return (
    <svg className="audio-section-icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="9" y="3" width="6" height="11" rx="3" />
      <path d="M5 11a7 7 0 0 0 14 0" />
      <path d="M12 18v3" />
    </svg>
  );
}

export function AudioSettingsDialog({ onChanged }: { onChanged?: (s: AudioStatus) => void }) {
  const open = useAudioSettingsUi((u) => u.open);
  const setOpen = useAudioSettingsUi((u) => u.setOpen);
  const status = useAudio((a) => a.status);
  const onClose = () => setOpen(false);
  const [devices, setDevices] = useState<DeviceList | null>(null);
  const [out, setOut] = useState<string>("");
  const [inp, setInp] = useState<string>("");
  const [buffer, setBuffer] = useState<number>(256);
  const [busy, setBusy] = useState(false);
  const [gainDb, setGainDb] = useState(0);
  const [monitor, setMonitor] = useState<DeviceMonitor | null>(null);
  const [monitorDb, setMonitorDb] = useState(0);
  const [route, setRoute] = useState(MONITOR_ROUTE);
  const [toneOn, setToneOn] = useState(false);
  const toneTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => () => void (toneTimer.current && clearTimeout(toneTimer.current)), []);

  // ao abrir: lista os dispositivos e mostra a configuração ATUAL ("" = padrão do sistema / nenhuma)
  useEffect(() => {
    if (!open) return;
    void Promise.all([api.audioDevices(), api.audioStatus(), api.audioMonitorGet()])
      .then(([d, s, m]) => {
        setDevices(d);
        useAudio.getState().setStatus(s);
        setOut(s.outputId ?? "");
        setInp(s.inputId ?? "");
        if (s.bufferSize) setBuffer(s.bufferSize);
        setGainDb(s.inputGainDb ?? 0);
        setMonitor(m);
        if (m) {
          setMonitorDb(m.gainDb);
          setRoute({ left: m.left, right: m.right });
        }
      })
      .catch((e) => notify.error(errorMessage(e)));
  }, [open]);

  const apply = async () => {
    setBusy(true);
    try {
      const s = await api.audioConfigure({ outputId: out || null, inputId: inp || null, bufferSize: buffer });
      onChanged?.(s);
      useAudio.getState().setStatus(s);
      if (s.error) notify.error(`Sem som: ${s.error}. O app ficou em modo silencioso — escolha outro fone/alto-falante.`);
      else if (s.warning) notify.info(s.warning);
      else notify.success(`Áudio: ${s.outputName ?? "saída padrão"} · ${formatRate(s.sampleRate)} · buffer ${s.bufferSize ?? "padrão"}`);
      onClose();
    } catch (e) {
      notify.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const testTone = async () => {
    try {
      await api.audioTestTone();
      setToneOn(true);
      if (toneTimer.current) clearTimeout(toneTimer.current);
      toneTimer.current = setTimeout(() => setToneOn(false), TONE_MS);
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const changeGain = async (db: number) => {
    setGainDb(db);
    try {
      await setInputGain(db);
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const applyMonitor = async (on: boolean, db = monitorDb) => {
    try {
      setMonitor(await api.audioMonitorSet(on ? { ...route, gainDb: db } : null));
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const sampleRate = status?.sampleRate ?? 48000;
  const latency = roundTripMs(buffer, sampleRate);
  const liveChannels = status?.inputName ? Math.min(status.inputChannels ?? 0, 8) : 0;
  // os medidores mostram o dispositivo EM USO; outro escolhido só vale depois de Aplicar
  const pendingInput = inp !== (status?.inputId ?? "");

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
      <p className="audio-system">
        Sistema de áudio: {devices?.host ?? "…"} · {formatRate(sampleRate)}
      </p>

      <section className="audio-section" aria-labelledby="audio-out-title">
        <h3 id="audio-out-title" className="audio-section-title">
          <HeadphonesIcon /> Fone / alto-falantes — o que você ouve
        </h3>
        <Field label="Dispositivo de saída">
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
        {status && status.mode !== "device" ? (
          <p className="audio-alert" role="alert">
            Sem som agora: nenhum fone/alto-falante abriu{status.error ? ` (${status.error})` : ""}. Escolha outro dispositivo e clique em Aplicar.
          </p>
        ) : status?.warning ? (
          <p className="audio-alert audio-alert--warn" role="status">
            {status.warning}
          </p>
        ) : null}
        <div className="audio-row">
          <Button onClick={() => void testTone()} data-tip="audio.testTone" disabled={status?.mode !== "device"} title={status?.mode !== "device" ? "Sem dispositivo de saída aberto" : undefined}>
            Testar som
          </Button>
          <span className={`tone-indicator${toneOn ? " tone-indicator--on" : ""}`} role="status" aria-label="Tom de teste">
            <span aria-hidden="true" />
            {toneOn ? "Tocando…" : ""}
          </span>
        </div>
        <p className="audio-hint">Toca um tom curto a −12 dBFS. Se ouvir baixo aqui, o volume do sistema operacional ou do fone está baixo — o Ebiki não reduz o volume.</p>
      </section>

      <section className="audio-section" aria-labelledby="audio-in-title">
        <h3 id="audio-in-title" className="audio-section-title">
          <MicIcon /> Microfone / interface — o que você grava
        </h3>
        <Field label="Dispositivo de entrada">
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
        {pendingInput ? (
          <p className="audio-hint">Clique em Aplicar para usar o novo microfone e ver os níveis dele.</p>
        ) : liveChannels > 0 ? (
          <InputMeters channels={liveChannels} />
        ) : (
          <p className="audio-hint">Escolha um microfone para ver os níveis.</p>
        )}
        <label className="audio-slider">
          <span>Ganho</span>
          <input
            type="range"
            min={0}
            max={INPUT_GAIN_MAX_DB}
            step={1}
            value={gainDb}
            aria-label="Ganho de entrada"
            aria-valuetext={signed(gainDb)}
            onChange={(e) => void changeGain(Number(e.target.value))}
          />
          <span className="mono">{signed(gainDb)}</span>
        </label>
        <label className="audio-check">
          <input type="checkbox" checked={!!monitor} disabled={!status?.inputName} onChange={(e) => void applyMonitor(e.target.checked)} />
          Ouvir o microfone no fone enquanto grava
        </label>
        <label className="audio-slider">
          <span>Volume no fone</span>
          <input
            type="range"
            min={-60}
            max={6}
            step={1}
            value={monitorDb}
            aria-label="Volume de monitoração"
            onChange={(e) => setMonitorDb(Number(e.target.value))}
            onPointerUp={() => monitor && void applyMonitor(true, monitorDb)}
            onKeyUp={() => monitor && void applyMonitor(true, monitorDb)}
          />
          <span className="mono">{monitorDb <= -60 ? "mudo" : signed(monitorDb)}</span>
        </label>
      </section>

      <details className="audio-section audio-perf">
        <summary>Desempenho</summary>
        <Field label={`Buffer — ~${formatDecimal(latency, 1)} ms de ida e volta a ${formatRate(sampleRate)}`}>
          <select className="select" value={buffer} onChange={(e) => setBuffer(Number(e.target.value))} aria-label="Tamanho do buffer">
            {[32, 64, 128, 256, 512, 1024, 2048].map((b) => (
              <option key={b} value={b}>
                {b} amostras{b <= 128 ? " (gravação)" : b >= 1024 ? " (mixagem pesada)" : ""}
              </option>
            ))}
          </select>
        </Field>
        <p className="audio-hint">Buffers menores reduzem a latência; se ouvir estalos, aumente o buffer.</p>
      </details>
    </Dialog>
  );
}
