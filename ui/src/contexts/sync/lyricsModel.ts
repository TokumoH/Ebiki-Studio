// Regras puras do painel de letra: unidades por granularidade, texto, posições e a máquina de
// estados da sincronização por toque (a validação final é refeita no núcleo — ctx-sync).
import type { Granularity, LyricLine, LyricSpan, LyricUnit, SyllableRef } from "../../ipc/types";

export const PPQ = 960;

export interface UnitInfo {
  unit: LyricUnit;
  text: string;
  span: LyricSpan | null;
  first: SyllableRef;
  last: SyllableRef;
}

/** Unidades (sílaba/palavra/linha) na ordem de leitura, com texto e intervalo derivados. */
export function lyricUnits(lines: LyricLine[], g: Granularity): UnitInfo[] {
  const out: UnitInfo[] = [];
  const spanOf = (spans: (LyricSpan | null)[]): LyricSpan | null => {
    const timed = spans.filter((s): s is LyricSpan => s !== null);
    if (timed.length === 0) return null;
    return { start: Math.min(...timed.map((s) => s.start)), end: Math.max(...timed.map((s) => s.end)) };
  };
  lines.forEach((l, line) => {
    if (g === "line") {
      const syl = l.words.flatMap((w) => w.syllables);
      const lastWord = l.words.length - 1;
      out.push({
        unit: { kind: "line", line },
        text: l.words.map((w) => w.syllables.map((s) => s.text).join("")).join(" "),
        span: spanOf(syl.map((s) => s.span)),
        first: { line, word: 0, syllable: 0 },
        last: { line, word: Math.max(0, lastWord), syllable: Math.max(0, (l.words[lastWord]?.syllables.length ?? 1) - 1) },
      });
      return;
    }
    l.words.forEach((w, word) => {
      if (g === "word") {
        out.push({ unit: { kind: "word", line, word }, text: w.syllables.map((s) => s.text).join(""), span: spanOf(w.syllables.map((s) => s.span)), first: { line, word, syllable: 0 }, last: { line, word, syllable: w.syllables.length - 1 } });
        return;
      }
      w.syllables.forEach((s, syllable) => out.push({ unit: { kind: "syllable", line, word, syllable }, text: s.text, span: s.span, first: { line, word, syllable }, last: { line, word, syllable } }));
    });
  });
  return out;
}

/** Texto editável da letra (sílabas concatenadas; hífens explícitos preservados). */
export function lyricsText(lines: LyricLine[]): string {
  return lines.map((l) => l.words.map((w) => w.syllables.map((s) => s.text).join("")).join(" ")).join("\n");
}

/** Última sílaba da letra (fim do intervalo de deslocamento). */
export function lastSyllable(lines: LyricLine[]): SyllableRef | null {
  for (let line = lines.length - 1; line >= 0; line--) {
    const words = lines[line]!.words;
    if (words.length) return { line, word: words.length - 1, syllable: words[words.length - 1]!.syllables.length - 1 };
  }
  return null;
}

/** Índice da unidade ativa no tick (karaokê). */
export function activeUnit(units: UnitInfo[], tick: number): number {
  return units.findIndex((u) => u.span !== null && tick >= u.span.start && tick < u.span.end);
}

/** "compasso.tempo" (1-based) a partir do tick, pela fórmula inicial; fora do tempo acrescenta
 * os ticks dentro do tempo (como o contador do transporte: 2.1.480). */
export function formatPosition(tick: number, meter: { numerator: number; denominator: number } = { numerator: 4, denominator: 4 }): string {
  const beat = (PPQ * 4) / meter.denominator;
  const bar = beat * meter.numerator;
  const b = Math.floor(tick / bar);
  const rest = tick - b * bar;
  const bt = Math.floor(rest / beat);
  const t = Math.round(rest - bt * beat);
  return t ? `${b + 1}.${bt + 1}.${String(t).padStart(3, "0")}` : `${b + 1}.${bt + 1}`;
}

export const ticksToBeats = (t: number) => Math.round((t / PPQ) * 1000) / 1000;
export const beatsToTicks = (b: number) => Math.max(0, Math.round(b * PPQ));

// ---------------------------------------------------------------- sincronização por toque

export interface TapState {
  /** Primeira unidade a sincronizar (índice na granularidade). */
  from: number;
  /** Total de unidades da letra na granularidade. */
  total: number;
  /** Inícios tocados (ticks, estritamente crescentes). */
  taps: number[];
}

export type TapResult = { ok: true; state: TapState } | { ok: false; error: string };

export function startTap(total: number, from: number): TapState {
  return { from: Math.max(0, Math.min(from, Math.max(0, total - 1))), total, taps: [] };
}

export const tapsRemaining = (s: TapState) => s.total - s.from - s.taps.length;

/** Registra um toque (início da próxima unidade). Recusa tempo que não avança e excesso de toques. */
export function tap(s: TapState, tick: number): TapResult {
  if (tapsRemaining(s) <= 0) return { ok: false, error: "Todas as unidades já foram marcadas — conclua com Enter." };
  const last = s.taps[s.taps.length - 1];
  if (last !== undefined && tick <= last) return { ok: false, error: "O toque precisa vir depois do anterior (a reprodução está parada?)." };
  if (tick < 0) return { ok: false, error: "Posição inválida." };
  return { ok: true, state: { ...s, taps: [...s.taps, tick] } };
}

export function undoTap(s: TapState): TapState {
  return { ...s, taps: s.taps.slice(0, -1) };
}

/** Fecha a sessão: `end` encerra a última unidade tocada. */
export function finishTap(s: TapState, end: number): { ok: true; taps: number[]; end: number; from: number } | { ok: false; error: string } {
  const last = s.taps[s.taps.length - 1];
  if (last === undefined) return { ok: false, error: "Nenhum toque registrado." };
  return { ok: true, taps: s.taps, end: end > last ? end : last + PPQ / 2, from: s.from };
}
