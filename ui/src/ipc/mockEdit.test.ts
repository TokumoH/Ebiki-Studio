import { setTransport } from "./transport";
import { createMockTransport } from "./mock";
import { api, editApi } from "./api";

async function withClip() {
  await api.projectCreate({ dir: "/tmp", name: "Edição", sampleRate: 48000, bitDepth: 24 });
  const v = await api.audioImport(["/x/voz.wav"], null, 0);
  return v.tracks[0]!.clips[0]!;
}

describe("transporte simulado — edição (Fase 3)", () => {
  beforeEach(() => setTransport(createMockTransport()));

  it("dividir gera duas partes contíguas e desfazer volta a uma", async () => {
    const c = await withClip();
    let v = await editApi.clipSplit(c.id, 960);
    const [a, b] = v.tracks[0]!.clips;
    expect(a!.lengthFrames).toBe(24000);
    expect(b).toMatchObject({ start: 960, offsetFrames: 24000, lengthFrames: c.lengthFrames - 24000 });
    expect(v.history.undoLabel).toBe("Dividir clipe");
    v = await api.undo();
    expect(v.tracks[0]!.clips).toHaveLength(1);
    await expect(editApi.clipSplit(c.id, 0)).rejects.toMatchObject({ code: "project.invalid_split" });
  });

  it("fades respeitam a duração e um gesto vira um único desfazer", async () => {
    const c = await withClip();
    for (const f of [100, 200, 300]) await editApi.clipSetFades(c.id, { fadeInFrames: f, fadeOutFrames: 0, fadeInShape: "linear", fadeOutShape: "linear" }, 9);
    let v = await api.undo();
    expect(v.tracks[0]!.clips[0]!.fadeInFrames).toBe(0);
    await expect(editApi.clipSetFades(c.id, { fadeInFrames: c.lengthFrames, fadeOutFrames: 1, fadeInShape: "linear", fadeOutShape: "linear" })).rejects.toMatchObject({ code: "project.invalid_fade" });
    v = await editApi.clipSetPolarity(c.id, true);
    expect(v.tracks[0]!.clips[0]!.polarityInverted).toBe(true);
    v = await editApi.clipSetReversed(c.id, true);
    expect(v.tracks[0]!.clips[0]!.reversed).toBe(true);
    v = await editApi.clipNormalize(c.id);
    expect(v.tracks[0]!.clips[0]!.gain).toBeCloseTo(1.5, 1);
  });

  it("cortar pelas bordas e crossfade em clipes sobrepostos", async () => {
    const c = await withClip();
    let v = await editApi.clipTrim(c.id, 96, c.lengthFrames - 2400);
    expect(v.tracks[0]!.clips[0]).toMatchObject({ start: 96, offsetFrames: 2400 });
    await expect(editApi.clipTrim(c.id, 0, c.lengthFrames + 1)).rejects.toMatchObject({ code: "project.clip_out_of_bounds" });
    v = await editApi.clipSplit(c.id, 960);
    const second = v.tracks[0]!.clips[1]!;
    v = await editApi.clipTrim(second.id, 864, second.lengthFrames + 2400);
    v = await editApi.clipCrossfade(c.id, second.id, "equalPower");
    expect(v.tracks[0]!.clips[0]!.fadeOutFrames).toBe(2400);
    expect(v.tracks[0]!.clips[1]).toMatchObject({ fadeInFrames: 2400, fadeInShape: "equalPower" });
  });

  it("espectrograma vem em binário AMSG", async () => {
    const c = await withClip();
    const s = await editApi.sourceSpectrogram(c.sourceId, { fftSize: 1024, hop: 512, bins: 64 });
    expect(s.bins).toBe(64);
    expect(s.columns).toBe(Math.ceil(c.sourceFrames / 512));
    expect(s.data.some((x) => x > 200)).toBe(true);
  });
});
