// Rótulos de exibição do contexto Biblioteca (sem regra de negócio além da formatação).
import type { BaseKey, BaseKind, HarmonyKeysCatalog } from "../../ipc/types";

export const KIND_LABEL: Record<BaseKind, string> = { generated: "gerada", userAudio: "áudio", bundledCc0: "CC0" };

const NAMES = ["C", "C#/Db", "D", "D#/Eb", "E", "F", "F#/Gb", "G", "G#/Ab", "A", "A#/Bb", "B"];

export function tonicName(tonic: number, catalog?: HarmonyKeysCatalog | null): string {
  const t = catalog?.tonics.find((x) => x.value === tonic);
  if (t) return t.sharp === t.flat ? t.sharp : `${t.sharp}/${t.flat}`;
  return NAMES[tonic] ?? String(tonic);
}

export function keyLabel(k: BaseKey | null | undefined, catalog?: HarmonyKeysCatalog | null): string {
  if (!k) return "—";
  return `${tonicName(k.tonic, catalog)} ${k.mode === "major" ? "maior" : "menor"}`;
}

/** Todas as 24 tonalidades maior/menor para seletores. */
export function allBaseKeys(): BaseKey[] {
  return Array.from({ length: 24 }, (_, i) => ({ tonic: i >> 1, mode: i % 2 === 0 ? "major" : "minor" }) as BaseKey);
}
