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
