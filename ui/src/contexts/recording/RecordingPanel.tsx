import { useEffect, useState } from "react";
import { api } from "../../ipc/api";
import type { Track } from "../../ipc/types";
import { refreshAudio, useAudio } from "../../core/audio";
import { InputSection } from "./InputSection";
import { parseRouteKey, routeKey, routeOptions } from "./inputRoutes";
import { run, useSession } from "../../core/session";
import { Button, Field, Toggle } from "../../shared/components";
import "./recording.css";

const BAR = 960 * 4;

function InputSelect({ track, channels }: { track: Track; channels: number }) {
  return (
    <select
      className="select"
      aria-label={`Entrada de ${track.name}`}
      value={routeKey(track.recording.input)}
      onChange={(e) => void run(() => api.recordingTrack({ trackId: track.id, input: parseRouteKey(e.target.value) }))}
    >
      <option value="">Sem entrada</option>
      {routeOptions(channels).map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

/** Configuração de gravação: entradas, monitoração, takes, metrônomo, punch e latência. */
export function RecordingPanel() {
  const session = useSession((s) => s.session);
  const audio = useAudio((a) => a.status);
  const [punch, setPunch] = useState<[number, number]>([1, 2]);
  useEffect(() => {
    void refreshAudio().catch(() => undefined);
  }, []);
  if (!session) return null;
  const rec = session.recording;
  const channels = audio?.inputName ? (audio.inputChannels ?? 0) : 0;
  return (
    <div className="recording">
      <div className="recording-main">
      <InputSection />
      <section>
        <h3>Faixas</h3>
        {channels === 0 ? <p className="hint">Nenhuma entrada de áudio ativa: escolha o dispositivo em Entrada de áudio, acima.</p> : null}
        <table>
          <thead>
            <tr>
              <th>Faixa</th>
              <th>Armar</th>
              <th>Entrada</th>
              <th>Monitorar</th>
              <th>Takes</th>
            </tr>
          </thead>
          <tbody>
            {session.tracks.map((t) => (
              <tr key={t.id}>
                <td>{t.name}</td>
                <td>
                  <Toggle className="toggle-r" pressed={t.recording.armed} onChange={(v) => void run(() => api.recordingTrack({ trackId: t.id, armed: v }))} aria-label={`Armar ${t.name}`}>
                    ⏺
                  </Toggle>
                </td>
                <td>
                  <InputSelect track={t} channels={channels} />
                </td>
                <td>
                  <select className="select" aria-label={`Monitoração de ${t.name}`} value={t.recording.monitor} onChange={(e) => void run(() => api.recordingTrack({ trackId: t.id, monitor: e.target.value as Track["recording"]["monitor"] }))}>
                    <option value="off">Desligada</option>
                    <option value="whenArmed">Quando armada</option>
                    <option value="always">Sempre</option>
                  </select>
                </td>
                <td>
                  {t.recording.takes.length === 0 ? (
                    <span className="hint">—</span>
                  ) : (
                    <select className="select" aria-label={`Take ativo de ${t.name}`} value={t.recording.activeTake ?? ""} onChange={(e) => void run(() => api.takeSelect(t.id, e.target.value))}>
                      {t.recording.takes.map((k) => (
                        <option key={k.id} value={k.id}>
                          Take {k.number}
                        </option>
                      ))}
                    </select>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      </div>
      <section className="recording-side">
        <h3>Metrônomo</h3>
        <div className="row">
          <Toggle pressed={rec.metronome.enabled} onChange={(v) => void run(() => api.recordingMetronome({ ...rec.metronome, enabled: v }))}>
            Ligado
          </Toggle>
          <Toggle pressed={rec.metronome.accentFirstBeat} onChange={(v) => void run(() => api.recordingMetronome({ ...rec.metronome, accentFirstBeat: v }))}>
            Acentuar 1º tempo
          </Toggle>
        </div>
        <div className="row">
          <Field label="Volume">
            <input type="range" min={0} max={1} step={0.05} value={rec.metronome.volume} onChange={(e) => void run(() => api.recordingMetronome({ ...rec.metronome, volume: Number(e.target.value) }))} />
          </Field>
          <Field label="Subdivisão">
            <select className="select" value={rec.metronome.subdivision} onChange={(e) => void run(() => api.recordingMetronome({ ...rec.metronome, subdivision: Number(e.target.value) }))}>
              <option value={1}>Tempos</option>
              <option value={2}>Colcheias</option>
              <option value={3}>Tercinas</option>
              <option value={4}>Semicolcheias</option>
            </select>
          </Field>
          <Field label="Contagem">
            <select className="select" value={rec.metronome.countInBars} onChange={(e) => void run(() => api.recordingMetronome({ ...rec.metronome, countInBars: Number(e.target.value) }))}>
              {[0, 1, 2, 4].map((n) => (
                <option key={n} value={n}>
                  {n === 0 ? "Sem contagem" : `${n} compasso${n > 1 ? "s" : ""}`}
                </option>
              ))}
            </select>
          </Field>
        </div>
        <h3>Punch in/out</h3>
        <div className="row">
          <Field label="Do compasso">
            <input className="input" type="number" min={1} value={punch[0]} onChange={(e) => setPunch([Math.max(1, Number(e.target.value)), punch[1]])} style={{ width: 70 }} />
          </Field>
          <Field label="Até o compasso">
            <input className="input" type="number" min={2} value={punch[1]} onChange={(e) => setPunch([punch[0], Math.max(punch[0] + 1, Number(e.target.value))])} style={{ width: 70 }} />
          </Field>
          <Button onClick={() => void run(() => api.recordingPunch([(punch[0] - 1) * BAR, (punch[1] - 1) * BAR]))}>Definir</Button>
          <Button variant="ghost" disabled={!rec.punch} onClick={() => void run(() => api.recordingPunch(null))}>
            Limpar
          </Button>
        </div>
        <p className="hint">Compassos em 4/4 a partir do início. {rec.punch ? `Ativo: ticks ${rec.punch[0]}–${rec.punch[1]}.` : "Sem punch: grava do ponto atual."}</p>
        <h3>Compensação de latência</h3>
        <div className="row">
          <Field label="Amostras">
            <input className="input" type="number" min={0} max={48000} value={rec.latencyCompensationFrames} onChange={(e) => void run(() => api.recordingLatency(Math.max(0, Math.min(48000, Number(e.target.value)))))} style={{ width: 100 }} />
          </Field>
          {audio?.bufferSize ? (
            <Button variant="ghost" onClick={() => void run(() => api.recordingLatency((audio.bufferSize ?? 0) * 2))} title="Estimativa: buffer de entrada + saída">
              Usar estimativa ({(audio.bufferSize ?? 0) * 2})
            </Button>
          ) : null}
        </div>
      </section>
    </div>
  );
}
