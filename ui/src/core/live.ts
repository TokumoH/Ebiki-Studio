// Telemetria do núcleo (posição e medidores, ~30 Hz).
import { create } from "zustand";
import type { MetersView, TransportView } from "../ipc/types";
import { transport } from "../ipc/transport";

interface LiveState {
  transport: TransportView | null;
  meters: MetersView | null;
}

export const useLive = create<LiveState>(() => ({ transport: null, meters: null }));

export async function startTelemetry(): Promise<() => void> {
  const t = transport();
  const offA = await t.listen<TransportView>("transport", (p) => useLive.setState({ transport: p }));
  const offB = await t.listen<MetersView>("meters", (p) => useLive.setState({ meters: p }));
  return () => {
    offA();
    offB();
  };
}
