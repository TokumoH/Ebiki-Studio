// Regras puras: filtros do formulário de busca → `BaseQuery` do núcleo.
import type { BaseKind, BaseQuery } from "../../ipc/types";

export interface BaseFilters {
  text: string;
  bpmMin: string;
  bpmMax: string;
  /** "" = qualquer; senão "<tônica>:<major|minor>". */
  key: string;
  genre: string;
  instrument: string;
  /** Lista separada por vírgula. */
  tags: string;
  kind: "" | BaseKind;
}

export const EMPTY_FILTERS: BaseFilters = { text: "", bpmMin: "", bpmMax: "", key: "", genre: "", instrument: "", tags: "", kind: "" };

function bpm(s: string): number | undefined {
  if (s.trim() === "") return undefined;
  const n = Number(s.replace(",", "."));
  if (!Number.isFinite(n)) return undefined;
  return Math.min(999, Math.max(20, n));
}

export function parseTags(s: string): string[] {
  const seen = new Set<string>();
  for (const t of s.split(",")) {
    const v = t.trim().toLowerCase();
    if (v) seen.add(v);
  }
  return [...seen];
}

export function encodeKey(k: { tonic: number; mode: "major" | "minor" } | null | undefined): string {
  return k ? `${k.tonic}:${k.mode}` : "";
}

export function decodeKey(s: string): { tonic: number; mode: "major" | "minor" } | undefined {
  const m = /^(\d{1,2}):(major|minor)$/.exec(s);
  if (!m) return undefined;
  const tonic = Number(m[1]);
  if (tonic < 0 || tonic > 11) return undefined;
  return { tonic, mode: m[2] as "major" | "minor" };
}

/** Converte os filtros em consulta: omite vazios, limita BPM (20–999) e ordena o intervalo. */
export function filtersToQuery(f: BaseFilters): BaseQuery {
  const q: BaseQuery = {};
  const text = f.text.trim();
  if (text) q.text = text;
  let lo = bpm(f.bpmMin);
  let hi = bpm(f.bpmMax);
  if (lo !== undefined && hi !== undefined && lo > hi) [lo, hi] = [hi, lo];
  if (lo !== undefined) q.bpmMin = lo;
  if (hi !== undefined) q.bpmMax = hi;
  const key = decodeKey(f.key);
  if (key) q.key = key;
  if (f.genre.trim()) q.genre = f.genre.trim();
  if (f.instrument) q.instrument = f.instrument;
  const tags = parseTags(f.tags);
  if (tags.length) q.tags = tags;
  if (f.kind) q.kind = f.kind;
  return q;
}
