// Rotas de entrada (mono por canal e pares estéreo) — usadas pela entrada da faixa e pelo
// "Ouvir entrada". Funções puras.

export interface InputRoute {
  left: number;
  right: number | null;
}

export interface RouteOption {
  value: string;
  label: string;
  route: InputRoute;
}

/** "0:" = canal 1 mono; "0:1" = canais 1/2 estéreo. */
export function routeKey(r: InputRoute | null): string {
  return r ? `${r.left}:${r.right ?? ""}` : "";
}

export function parseRouteKey(v: string): InputRoute | null {
  if (!v) return null;
  const [l, r] = v.split(":");
  return { left: Number(l), right: r ? Number(r) : null };
}

/** Opções para `channels` canais de entrada (mínimo 2 para quem ainda não tem dispositivo). */
export function routeOptions(channels: number, minChannels = 2): RouteOption[] {
  const n = Math.max(channels, minChannels);
  const opts: RouteOption[] = [];
  for (let c = 0; c < n; c++) opts.push({ value: `${c}:`, label: `Entrada ${c + 1} (mono)`, route: { left: c, right: null } });
  for (let c = 0; c + 1 < n; c += 2) opts.push({ value: `${c}:${c + 1}`, label: `Entradas ${c + 1}/${c + 2} (estéreo)`, route: { left: c, right: c + 1 } });
  return opts;
}

/**
 * Opções de entrada de uma faixa: só os canais que o microfone tem agora (AU-05). Uma rota salva que
 * deixou de existir (microfone trocado/desconectado) aparece marcada como indisponível, para a
 * pessoa ver por que a gravação não aceita aquela faixa.
 */
export function trackRouteOptions(channels: number, current: InputRoute | null): RouteOption[] {
  const opts = routeOptions(channels, 0);
  const key = routeKey(current);
  if (current && key && !opts.some((o) => o.value === key)) {
    const label = current.right === null ? `Entrada ${current.left + 1}` : `Entradas ${current.left + 1}/${current.right + 1}`;
    opts.unshift({ value: key, label: `${label} (indisponível)`, route: current });
  }
  return opts;
}

/** Pico linear → dBFS (piso de −60). */
export function peakDb(peak: number): number {
  return peak > 0 ? Math.max(-60, 20 * Math.log10(peak)) : -60;
}

/** Largura do medidor (0–1) na escala de −60 a 0 dBFS. */
export function meterFraction(peak: number): number {
  return Math.min(1, (peakDb(peak) + 60) / 60);
}

/** Decaimento visual: sobe na hora, desce aos poucos (~20 dB/s a 30 leituras/s). */
export function decayPeak(previous: number, next: number, factor = 0.85): number {
  return Math.max(next, previous * factor);
}

/** Faixa-alvo do nível de entrada (dBFS), mostrada em verde nos medidores. */
export const TARGET_ZONE_DB: readonly [number, number] = [-18, -6];
/** Abaixo disto o sinal é fraco demais (aviso "Sinal baixo"). */
export const LOW_SIGNAL_DB = -30;
/** Acima disto está perto de distorcer ("Muito alto"). */
export const CLIP_DB = -1;

export type LevelZone = "low" | "quiet" | "target" | "loud" | "clip";

/** Zona do nível (dBFS) para cor e aviso: fraco, abaixo do alvo, no alvo, acima do alvo, distorcendo. */
export function levelZone(db: number): LevelZone {
  if (db > CLIP_DB) return "clip";
  if (db > TARGET_ZONE_DB[1]) return "loud";
  if (db >= TARGET_ZONE_DB[0]) return "target";
  if (db >= LOW_SIGNAL_DB) return "quiet";
  return "low";
}

/** Posição (0–1) de um nível em dB na escala do medidor (−60 a 0 dBFS). */
export function dbFraction(db: number): number {
  return Math.min(1, Math.max(0, (db + 60) / 60));
}
