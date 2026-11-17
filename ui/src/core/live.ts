// Telemetria do núcleo (posição e medidores, ~30 Hz).
import { create } from "zustand";
import type { MetersView, TransportView } from "../ipc/types";
import { transport } from "../ipc/transport";
import { AudioStatus } from "../ipc/schema";
import { useAudio } from "./audio";
import { notify } from "./toasts";
import { api } from "../ipc/api";
import { useSession } from "./session";

interface LiveState {
  transport: TransportView | null;
  meters: MetersView | null;
}

export const useLive = create<LiveState>(() => ({ transport: null, meters: null }));

export async function startTelemetry(): Promise<() => void> {
  const t = transport();
  const offA = await t.listen<TransportView>("transport", (p) => useLive.setState({ transport: p }));
  const offB = await t.listen<MetersView>("meters", (p) => useLive.setState({ meters: p }));
  // o núcleo reabriu o áudio sozinho (dispositivo desconectado ou religado): atualiza e avisa
  const offC = await t.listen<{ message: string; status: unknown }>("audio-reopened", (p) => {
    const s = AudioStatus.safeParse(p.status);
    if (s.success) useAudio.getState().setStatus(s.data);
    notify.info(p.message);
    // reabrir finaliza uma gravação em andamento: relê a sessão para o take aparecer (AU-07)
    if (useSession.getState().session) {
      void api
        .projectView()
        .then((v) => useSession.getState().setSession(v))
        .catch(() => undefined);
    }
  });
  return () => {
    offA();
    offB();
    offC();
  };
}
