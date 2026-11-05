// Regras puras do construtor de sequência (aba "Próximo acorde") e de notação.

/** Limite de passos de uma progressão no domínio (1–64). */
export const MAX_SEQUENCE = 64;

export interface SequenceChord {
  symbol: string;
  roman: string;
}

export function appendChord(seq: readonly SequenceChord[], c: SequenceChord): SequenceChord[] {
  if (seq.length >= MAX_SEQUENCE) return [...seq];
  return [...seq, { symbol: c.symbol, roman: c.roman }];
}

export function removeLast(seq: readonly SequenceChord[]): SequenceChord[] {
  return seq.slice(0, -1);
}

/** Histórico (cifras) enviado ao `harmony_suggest_next`. */
export function historyOf(seq: readonly SequenceChord[]): string[] {
  return seq.map((c) => c.symbol);
}

/**
 * Texto de cifra aceito por `ProgressionRef.cifra` ("C:4 G7:4"). Vazio quando não há acordes.
 * `beats` é limitado a 1–64 tempos por acorde.
 */
export function toCifraText(seq: readonly SequenceChord[], beats = 4): string {
  const b = Math.min(64, Math.max(1, Math.round(Number.isFinite(beats) ? beats : 4)));
  return seq.map((c) => `${c.symbol}:${b}`).join(" ");
}

export type Notation = "cifra" | "roman";

export function label(c: SequenceChord, notation: Notation): string {
  return notation === "cifra" ? c.symbol : c.roman;
}

/** Barra de pontuação (0..1) → número de blocos 1..5 para exibir a força da sugestão. */
export function scoreBars(score: number): number {
  if (!Number.isFinite(score)) return 1;
  return Math.min(5, Math.max(1, Math.round(score * 5)));
}
