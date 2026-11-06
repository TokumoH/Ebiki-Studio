// "Entrada de áudio" do painel Gravação (mockup 7): dispositivo de entrada, medidores por canal e
// "Ouvir entrada" (monitor direto do dispositivo, fora da gravação e da exportação).
import { useEffect, useRef, useState } from "react";
import { api } from "../../ipc/api";
import type { DeviceList, DeviceMonitor } from "../../ipc/types";
import { errorMessage } from "../../core/session";
import { notify } from "../../core/toasts";
import { setInputDevice, useAudio } from "../../core/audio";
import { useLive } from "../../core/live";
import { Toggle } from "../../shared/components";
import { decayPeak, meterFraction, parseRouteKey, peakDb, routeKey, routeOptions } from "./inputRoutes";

const DEFAULT_GAIN_DB = -6;

function InputMeters({ channels }: { channels: number }) {
  const inputs = useLive((s) => s.meters?.inputs);
  const shown = useRef<number[]>([]);
  // decaimento visual: subir na hora, descer aos poucos
  shown.current = Array.from({ length: channels }, (_, i) => decayPeak(shown.current[i] ?? 0, inputs?.[i] ?? 0));
  return (
    <div className="input-meters">
      {shown.current.map((p, i) => {
        const db = peakDb(p);
        return (
          <div key={i} className="input-meter-row">
            <span className="input-meter-label">Entrada {i + 1}</span>
            <div className="input-meter" role="meter" aria-label={`Nível da entrada ${i + 1}`} aria-valuemin={-60} aria-valuemax={0} aria-valuenow={Math.round(db)}>
              <div className={`input-meter-fill${db > -3 ? " hot" : db > -12 ? " warm" : ""}`} style={{ width: `${meterFraction(p) * 100}%` }} />
            </div>
            <span className="input-meter-db mono">{db <= -60 ? "−∞" : `${Math.round(db)} dB`}</span>
          </div>
        );
      })}
    </div>
  );
}

export function InputSection() {
  const status = useAudio((a) => a.status);
  const [devices, setDevices] = useState<DeviceList["inputs"] | null>(null);
  const [monitor, setMonitor] = useState<DeviceMonitor | null>(null);
  const [route, setRoute] = useState("0:");
  const [gainDb, setGainDb] = useState(DEFAULT_GAIN_DB);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void api
      .audioDevices()
      .then((d) => setDevices(d.inputs))
      .catch(() => setDevices([]));
    void api
      .audioMonitorGet()
      .then((m) => {
        setMonitor(m);
        if (m) {
          setRoute(routeKey(m));
          setGainDb(m.gainDb);
        }
      })
      .catch(() => undefined);
  }, []);

  const hasInput = !!status?.inputName;
  const channels = hasInput ? (status?.inputChannels ?? 0) : 0;
  const options = routeOptions(channels, 1);
  const inputId = status?.inputId ?? "";
  const known = !inputId || (devices ?? []).some((d) => d.id === inputId);

  const changeDevice = async (id: string) => {
    setBusy(true);
    try {
      const s = await setInputDevice(id || null);
      if (s.error) notify.error(`Dispositivo indisponível: ${s.error}.`);
      // a rota pode ter deixado de existir no novo dispositivo
      setMonitor(await api.audioMonitorGet());
    } catch (e) {
      notify.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const applyMonitor = async (on: boolean, nextRoute = route, nextGain = gainDb) => {
    const r = parseRouteKey(nextRoute);
    try {
      setMonitor(await api.audioMonitorSet(on && r ? { left: r.left, right: r.right, gainDb: nextGain } : null));
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  return (
    <section className="input-section" aria-label="Entrada de áudio">
      <div className="input-head">
        <h3>Entrada de áudio</h3>
        <span className="hint">Saída e buffer: Arquivo › Configurações de áudio</span>
      </div>
      <div className="row">
        <label className="field">
          <span>Dispositivo</span>
          <select className="select" aria-label="Dispositivo de entrada da gravação" value={inputId} disabled={busy || devices === null} onChange={(e) => void changeDevice(e.target.value)}>
            <option value="">Nenhuma</option>
            {!known ? <option value={inputId}>{inputId} (desconectado)</option> : null}
            {(devices ?? []).map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
                {d.isDefault ? " (padrão)" : ""}
              </option>
            ))}
          </select>
        </label>
        {hasInput ? (
          <span className="hint">
            {channels} {channels === 1 ? "canal" : "canais"} · {(status?.sampleRate ?? 48000) / 1000} kHz
            {status?.estimatedLatencyMs ? ` · ~${status.estimatedLatencyMs.toFixed(1)} ms` : ""}
          </span>
        ) : (
          <span className="hint">Escolha um dispositivo para gravar e ver os níveis.</span>
        )}
      </div>
      {hasInput && channels > 0 ? (
        <div className="input-body">
          <InputMeters channels={Math.min(channels, 8)} />
          <div className="input-monitor">
            <div className="row">
              <Toggle pressed={!!monitor} onChange={(on) => void applyMonitor(on)} aria-label="Ouvir entrada">
                🎧 Ouvir entrada
              </Toggle>
              <select
                className="select"
                aria-label="Canais para ouvir"
                value={route}
                onChange={(e) => {
                  setRoute(e.target.value);
                  if (monitor) void applyMonitor(true, e.target.value);
                }}
              >
                {options.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </div>
            <label className="input-gain">
              <span>Volume</span>
              <input
                type="range"
                min={-60}
                max={6}
                step={1}
                value={gainDb}
                aria-label="Volume de monitoração"
                onChange={(e) => setGainDb(Number(e.target.value))}
                onPointerUp={() => monitor && void applyMonitor(true, route, gainDb)}
                onKeyUp={() => monitor && void applyMonitor(true, route, gainDb)}
              />
              <span className="mono">{gainDb <= -60 ? "mudo" : `${gainDb > 0 ? "+" : ""}${gainDb} dB`}</span>
            </label>
            <p className="hint">Ouve o dispositivo direto na saída, sem armar faixa. Não entra na gravação nem na exportação. Use fones para evitar microfonia.</p>
          </div>
        </div>
      ) : null}
    </section>
  );
}
