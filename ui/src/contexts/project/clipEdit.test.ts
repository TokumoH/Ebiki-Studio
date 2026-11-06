import type { Clip, Track } from "../../ipc/types";
import { canSplitAt, envelopeAt, fadeFromPx, fadeGain, fadePath, framesPerTick, framesToMs, msToFrames, overlappingNeighbor, trimEndTo, trimStartTo, MIN_CLIP_FRAMES } from "./clipEdit";
import { spectrumColor } from "./spectrogramCache";

// 120 BPM @ 48 kHz: 1 tick = 25 amostras
const clip = (over: Partial<Clip> = {}): Clip => ({
  id: "c1",
  sourceId: "s1",
  sourceName: "voz.wav",
  start: 960,
  startFrame: 24_000,
  offsetFrames: 2_500,
  lengthFrames: 24_000,
  endTick: 1920,
  gain: 0,
  sourceFrames: 48_000,
  fadeInFrames: 0,
  fadeOutFrames: 0,
  fadeInShape: "linear",
  fadeOutShape: "linear",
  polarityInverted: false,
  reversed: false,
  ...over,
});

describe("cortar pelas bordas", () => {
  it("borda esquerda mantém o fim e respeita o início da fonte", () => {
    const c = clip();
    expect(framesPerTick(c)).toBe(25);
    expect(trimStartTo(c, 1000)).toEqual({ start: 1000, lengthFrames: 23_000 });
    // só há 2 500 amostras (100 ticks) antes do offset
    expect(trimStartTo(c, 0)).toEqual({ start: 860, lengthFrames: 26_500 });
    // não passa do fim (deixa ao menos MIN_CLIP_FRAMES)
    const r = trimStartTo(c, 5000);
    expect(r.lengthFrames).toBeGreaterThanOrEqual(MIN_CLIP_FRAMES);
    expect(r.start).toBeLessThan(c.endTick);
  });
  it("borda direita limitada pela fonte (normal e reverso)", () => {
    expect(trimEndTo(clip(), 1440)).toEqual({ start: 960, lengthFrames: 12_000 });
    expect(trimEndTo(clip(), 99_999).lengthFrames).toBe(48_000 - 2_500);
    expect(trimEndTo(clip({ reversed: true }), 99_999).lengthFrames).toBe(2_500 + 24_000);
    expect(trimEndTo(clip(), 0).lengthFrames).toBe(MIN_CLIP_FRAMES);
  });
  it("reverso: a borda esquerda revela o que vem depois da região na fonte", () => {
    const c = clip({ reversed: true, offsetFrames: 0, sourceFrames: 26_500 });
    expect(trimStartTo(c, 0)).toEqual({ start: 860, lengthFrames: 26_500 });
  });
});

describe("fades", () => {
  it("curvas iguais às da engine", () => {
    expect(fadeGain("linear", 0.5)).toBe(0.5);
    expect(fadeGain("equalPower", 0.5)).toBeCloseTo(Math.SQRT1_2, 6);
    expect(fadeGain("linear", 2)).toBe(1);
    expect(fadeGain("linear", NaN)).toBe(0);
  });
  it("envelope aplica entrada e saída como a engine", () => {
    const c = clip({ lengthFrames: 1000, fadeInFrames: 100, fadeOutFrames: 200 });
    expect(envelopeAt(c, 0)).toBe(0);
    expect(envelopeAt(c, 50)).toBeCloseTo(0.5);
    expect(envelopeAt(c, 500)).toBe(1);
    expect(envelopeAt(c, 900)).toBeCloseTo(0.5);
  });
  it("alça limita o fade ao espaço livre", () => {
    const c = clip({ lengthFrames: 1000, fadeOutFrames: 300 });
    expect(fadeFromPx(c, "in", 50, 100)).toBe(500);
    expect(fadeFromPx(c, "in", 90, 100)).toBe(700);
    expect(fadeFromPx(c, "out", -10, 100)).toBe(0);
  });
  it("caminho SVG começa e termina no topo", () => {
    const d = fadePath("linear", "in", 0, 10, 100, 2);
    expect(d).toBe("M0.0,0 L0.0,100.0 L5.0,50.0 L10.0,0.0 L10.0,0 Z");
  });
  it("ms ↔ amostras", () => {
    expect(msToFrames(10, 48_000)).toBe(480);
    expect(framesToMs(480, 48_000)).toBe(10);
    expect(msToFrames(-5, 48_000)).toBe(0);
  });
});

describe("dividir e crossfade", () => {
  it("só divide estritamente dentro do clipe", () => {
    expect(canSplitAt(clip(), 960)).toBe(false);
    expect(canSplitAt(clip(), 961)).toBe(true);
    expect(canSplitAt(clip(), 1920)).toBe(false);
  });
  it("acha o vizinho sobreposto parcialmente na mesma faixa", () => {
    const a = clip({ id: "a", startFrame: 0, lengthFrames: 1000 });
    const b = clip({ id: "b", startFrame: 800, lengthFrames: 1000 });
    const inner = clip({ id: "i", startFrame: 100, lengthFrames: 10 });
    const t = { clips: [a, b] } as unknown as Track;
    expect(overlappingNeighbor(t, "a")?.id).toBe("b");
    expect(overlappingNeighbor(t, "b")?.id).toBe("a");
    expect(overlappingNeighbor({ clips: [a, inner] } as unknown as Track, "a")).toBeNull();
    expect(overlappingNeighbor(t, "x")).toBeNull();
  });
});

it("mapa de cores do espectrograma vai do escuro ao claro", () => {
  const lum = (v: number) => spectrumColor(v).reduce((a, b) => a + b, 0);
  expect(lum(0)).toBeLessThan(lum(128));
  expect(lum(128)).toBeLessThan(lum(255));
  expect(spectrumColor(-10)).toEqual(spectrumColor(0));
});
