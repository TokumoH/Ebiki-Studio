// Falhas de buffer recentes (AU-11): o núcleo conta xruns desde que abriu; a barra de status só deve
// pedir "aumente o buffer" enquanto elas estão acontecendo, não para sempre depois de um soluço.
import { useEffect, useRef, useState } from "react";
import { useLive } from "./live";

/** Janela em que uma falha ainda conta como "recente". */
export const XRUN_WINDOW_MS = 10_000;

export interface XrunSample {
  at: number;
  total: number;
}

/**
 * Quantas falhas aconteceram dentro da janela, a partir das amostras (tempo, total acumulado).
 * Total que diminuiu = núcleo reaberto (contagem recomeçou): conta a partir dali.
 */
export function recentXruns(samples: readonly XrunSample[], now: number, windowMs = XRUN_WINDOW_MS): number {
  let count = 0;
  for (let i = 1; i < samples.length; i++) {
    const prev = samples[i - 1]!;
    const cur = samples[i]!;
    if (now - cur.at > windowMs) continue;
    count += cur.total >= prev.total ? cur.total - prev.total : cur.total;
  }
  return count;
}

/** Gancho: falhas de buffer nos últimos 10 s (atualiza a cada segundo). */
export function useRecentXruns(): number {
  const samples = useRef<XrunSample[]>([]);
  const [recent, setRecent] = useState(0);
  useEffect(() => {
    const tick = () => {
      const now = Date.now();
      const total = useLive.getState().meters?.xruns ?? 0;
      const s = samples.current;
      if (s.length === 0 || s[s.length - 1]!.total !== total) s.push({ at: now, total });
      // guarda a última amostra antiga como base e descarta o resto
      while (s.length > 2 && now - s[1]!.at > XRUN_WINDOW_MS) s.shift();
      setRecent(recentXruns(s, now));
    };
    const id = setInterval(tick, 1000);
    tick();
    return () => clearInterval(id);
  }, []);
  return recent;
}
