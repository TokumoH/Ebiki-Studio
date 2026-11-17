// "Microfone (entrada)" do painel Gravação (mockup 7, revisto no S1 da varredura 5): mostra o
// microfone escolhido e o ganho (somente leitura — escolhidos nas Configurações de áudio, num lugar
// só), os medidores por canal com a faixa-alvo verde e o "Ouvir o microfone no fone" (monitor direto
// do dispositivo, fora da gravação e da exportação).
import { useEffect, useState } from "react";
import { api } from "../../ipc/api";
import type { DeviceMonitor } from "../../ipc/types";
import { errorMessage } from "../../core/session";
import { notify } from "../../core/toasts";
import { openAudioSettings, useAudio, useAudioSettingsUi } from "../../core/audio";
import { Button, Toggle } from "../../shared/components";
import { InputMeters, InputSummaryMeter } from "./InputMeters";
import { parseRouteKey, routeKey, routeOptions } from "./inputRoutes";
import { formatDecimal } from "../../core/format";

/** Volume inicial do retorno no fone: 0 dB (o sinal como chega; antes era −6 dB e soava baixo). */
export const DEFAULT_MONITOR_GAIN_DB = 0;

const COLLAPSED_KEY = "ebiki.recording.inputCollapsed";

function loadCollapsed(): boolean {
  try {
    return window.localStorage.getItem(COLLAPSED_KEY) === "1";
  } catch {
    return false;
  }
}

/** Ganho em dB no formato da interface ("0 dB", "+12 dB"). */
export function formatGainDb(db: number): string {
  return `${db > 0 ? "+" : ""}${(Math.round(db * 10) / 10).toLocaleString("pt-BR")} dB`;
}

export function InputSection() {
  const status = useAudio((a) => a.status);
  const [collapsed, setCollapsed] = useState(loadCollapsed);
  const toggleCollapsed = () =>
    setCollapsed((c) => {
      try {
        window.localStorage.setItem(COLLAPSED_KEY, c ? "0" : "1");
      } catch {
        /* sem armazenamento: vale só nesta sessão */
      }
      return !c;
    });
  const [monitor, setMonitor] = useState<DeviceMonitor | null>(null);
  const [route, setRoute] = useState("0:");
  const [gainDb, setGainDb] = useState(DEFAULT_MONITOR_GAIN_DB);

  // relê a monitoração ao abrir, quando o dispositivo muda (a rota pode ter deixado de existir) e
  // quando as Configurações de áudio fecham (o retorno pode ter sido ligado/desligado lá)
  const inputId = status?.inputId ?? "";
  const settingsOpen = useAudioSettingsUi((u) => u.open);
  useEffect(() => {
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
  }, [inputId, settingsOpen, status?.inputChannels]);

  const hasInput = !!status?.inputName;
  const channels = hasInput ? (status?.inputChannels ?? 0) : 0;
  const options = routeOptions(channels, 1);
  // AU-06: rota que não existe mais no microfone atual volta para a primeira válida (senão o seletor
  // mostra uma coisa e o app envia outra)
  useEffect(() => {
    if (!options.some((o) => o.value === route)) setRoute(options[0]?.value ?? "0:");
  }, [channels]); // eslint-disable-line react-hooks/exhaustive-deps

  const applyMonitor = async (on: boolean, nextRoute = route, nextGain = gainDb) => {
    const r = parseRouteKey(nextRoute);
    try {
      setMonitor(await api.audioMonitorSet(on && r ? { left: r.left, right: r.right, gainDb: nextGain } : null));
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const configure = (
    <Button size="sm" onClick={openAudioSettings} data-tip="rec.configureAudio">
      Configurar microfone e fone…
    </Button>
  );

  return (
    <section className={`input-section${collapsed ? " input-section--collapsed" : ""}`} aria-label="Microfone (entrada)">
      <div className="input-head">
        <h3>
          <button type="button" className="input-toggle" aria-expanded={!collapsed} aria-controls="input-section-body" data-tip="rec.inputCollapse" onClick={toggleCollapsed}>
            <span aria-hidden="true">{collapsed ? "▸" : "▾"}</span> Microfone (entrada)
          </button>
        </h3>
        {collapsed ? (
          <span className="input-summary">
            <span className="input-summary-device">{status?.inputName ?? "Sem microfone"}</span>
            {status?.inputName ? <InputSummaryMeter /> : null}
            {monitor ? <span className="input-summary-listen">🎧 ouvindo</span> : null}
          </span>
        ) : (
          configure
        )}
      </div>
      {collapsed ? null : (
        <div id="input-section-body">
          <dl className="input-facts">
            <div>
              <dt>Dispositivo</dt>
              <dd>{status?.inputName ?? "Nenhum"}</dd>
            </div>
            <div>
              <dt>Ganho</dt>
              <dd data-tip="rec.inputGain">
                {formatGainDb(status?.inputGainDb ?? 0)}
              </dd>
            </div>
            {hasInput ? (
              <div>
                <dt>Formato</dt>
                <dd>
                  {channels} {channels === 1 ? "canal" : "canais"} · {((status?.sampleRate ?? 48000) / 1000).toLocaleString("pt-BR")} kHz
                  {status?.estimatedLatencyMs ? ` · ~${formatDecimal(status.estimatedLatencyMs, 1)} ms` : ""}
                </dd>
              </div>
            ) : null}
          </dl>
          {hasInput ? null : <p className="hint">Escolha o microfone em “Configurar microfone e fone…” para gravar e ver os níveis.</p>}
          {hasInput && channels > 0 ? (
            <div className="input-body">
              <InputMeters channels={Math.min(channels, 8)} />
              <div className="input-monitor">
                <div className="row">
                  <Toggle pressed={!!monitor} onChange={(on) => void applyMonitor(on)} aria-label="Ouvir o microfone no fone">
                    🎧 Ouvir no fone
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
                  <span className="mono">{gainDb <= -60 ? "mudo" : formatGainDb(gainDb)}</span>
                </label>
                <p className="hint">Ouve o microfone direto no fone, sem armar faixa. Não entra na gravação nem na exportação. Use fones para evitar microfonia.</p>
              </div>
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}
