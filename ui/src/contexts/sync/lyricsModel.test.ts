import type { LyricLine } from "../../ipc/types";
import { activeUnit, beatsToTicks, finishTap, formatPosition, lastSyllable, lyricUnits, lyricsText, startTap, tap, tapsRemaining, ticksToBeats, undoTap } from "./lyricsModel";

const lines: LyricLine[] = [
  { words: [{ syllables: [{ text: "Can-", span: { start: 0, end: 640 } }, { text: "to", span: { start: 640, end: 960 } }] }, { syllables: [{ text: "a-", span: null }, { text: "qui", span: null }] }] },
  { words: [{ syllables: [{ text: "ou-", span: null }, { text: "tra", span: null }] }] },
];

describe("unidades da letra", () => {
  it("por sílaba, palavra e linha com intervalos derivados", () => {
    expect(lyricUnits(lines, "syllable")).toHaveLength(6);
    const words = lyricUnits(lines, "word");
    expect(words.map((w) => w.text)).toEqual(["Can-to", "a-qui", "ou-tra"]);
    expect(words[0]!.span).toEqual({ start: 0, end: 960 });
    expect(words[1]!.span).toBeNull();
    const ls = lyricUnits(lines, "line");
    expect(ls[0]).toMatchObject({ text: "Can-to a-qui", span: { start: 0, end: 960 }, last: { line: 0, word: 1, syllable: 1 } });
    expect(lyricsText(lines)).toBe("Can-to a-qui\nou-tra");
    expect(lastSyllable(lines)).toEqual({ line: 1, word: 0, syllable: 1 });
    expect(activeUnit(words, 700)).toBe(0);
    expect(activeUnit(words, 2000)).toBe(-1);
  });
  it("posição compasso.tempo e conversão tempos ↔ ticks", () => {
    expect(formatPosition(0)).toBe("1.1");
    expect(formatPosition(3840 + 960)).toBe("2.2");
    expect(formatPosition(480)).toBe("1.1.480");
    expect(formatPosition(3840 + 960 + 48)).toBe("2.2.048");
    expect(formatPosition(1440, { numerator: 6, denominator: 8 })).toBe("1.4");
    expect(beatsToTicks(1.5)).toBe(1440);
    expect(ticksToBeats(1440)).toBe(1.5);
    expect(beatsToTicks(-2)).toBe(0);
  });
});

describe("máquina de estados da sincronização por toque", () => {
  it("toques crescentes, desfazer, limite de unidades e conclusão", () => {
    let s = startTap(3, 0);
    expect(tapsRemaining(s)).toBe(3);
    for (const t of [100, 500, 900]) {
      const r = tap(s, t);
      expect(r.ok).toBe(true);
      if (r.ok) s = r.state;
    }
    expect(tapsRemaining(s)).toBe(0);
    expect(tap(s, 1200)).toMatchObject({ ok: false });
    s = undoTap(s);
    expect(s.taps).toEqual([100, 500]);
    expect(finishTap(s, 1300)).toEqual({ ok: true, taps: [100, 500], end: 1300, from: 0 });
  });
  it("recusa toque que não avança e fim antes do último toque vira meio tempo depois", () => {
    let s = startTap(4, 1);
    const r = tap(s, 480);
    if (r.ok) s = r.state;
    expect(tap(s, 480)).toMatchObject({ ok: false });
    expect(tap(s, 100)).toMatchObject({ ok: false });
    expect(finishTap(s, 10)).toMatchObject({ ok: true, end: 960, from: 1 });
    expect(finishTap(startTap(2, 0), 100)).toMatchObject({ ok: false });
  });
  it("início limitado às unidades existentes", () => {
    expect(startTap(3, 10).from).toBe(2);
  });
});
