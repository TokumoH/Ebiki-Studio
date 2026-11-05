import { setTransport } from "./transport";
import { createMockTransport } from "./mock";
import { withCompositionMock } from "./mockComposition";
import { api, compositionApi as c } from "./api";

const C = { tonic: 0, mode: "ionian" };

describe("transporte simulado de composição", () => {
  beforeEach(() => setTransport(withCompositionMock(createMockTransport())));

  it("campo de Dó maior e todos os modos com 7 graus", async () => {
    const f = await c.harmonyField(C);
    expect(f.degrees.map((d) => d.triad.symbol)).toEqual(["C", "Dm", "Em", "F", "G", "Am", "Bdim"]);
    expect(f.degrees.map((d) => d.seventh.symbol)).toEqual(["Cmaj7", "Dm7", "Em7", "Fmaj7", "G7", "Am7", "Bm7(b5)"]);
    const keys = await c.harmonyKeys();
    for (const m of keys.modes) expect((await c.harmonyField({ tonic: 2, mode: m.mode })).degrees).toHaveLength(7);
  });

  it("após G7 em Dó sugere Dó primeiro", async () => {
    const s = await c.harmonySuggestNext({ key: C, history: ["Dm7", "G7"] });
    expect(s[0]?.symbol).toBe("C");
    expect(s[0]?.reason).toBe("authenticCadence");
    await expect(c.harmonySuggestNext({ key: C, history: ["H#"] })).rejects.toMatchObject({ code: "harmony.invalid_chord" });
  });

  it("progressões por gênero e inserção desfazível", async () => {
    const list = await c.harmonyProgressions({ key: C, genre: "samba" });
    expect(list.length).toBeGreaterThan(0);
    expect(list.every((p) => p.progression.genres.includes("samba") || p.progression.universal)).toBe(true);
    await api.projectCreate({ dir: "/tmp", name: "Teste", sampleRate: 48000, bitDepth: 24 });
    const ins = await c.harmonyInsertProgression({ realize: { progression: list[0]!.progression.ref, key: C }, atTick: 0 });
    expect(ins.view.tracks).toHaveLength(1);
    expect((await api.undo()).tracks).toHaveLength(0);
    const wav = await c.harmonyAudition({ progression: list[0]!.progression.ref, key: C });
    expect(new TextDecoder().decode(wav.slice(0, 4))).toBe("RIFF");
  });

  it("progressões do usuário com desfazer da biblioteca", async () => {
    const p = await c.harmonyUserProgressionAdd({ name: "Minha", mode: "ionian", genres: ["samba"], steps: "I VI7 ii7 V7" });
    await expect(c.harmonyUserProgressionAdd({ name: "minha", mode: "ionian", steps: "I" })).rejects.toMatchObject({ code: "harmony.duplicate_name" });
    expect((await c.harmonyUserProgressions()).map((x) => x.name)).toEqual(["Minha"]);
    if (p.ref.kind !== "user") throw new Error("ref");
    await c.harmonyUserProgressionRemove(p.ref.id);
    expect(await c.harmonyUserProgressions()).toEqual([]);
    const h = await c.libraryUndo();
    expect(h.canRedo).toBe(true);
    expect(await c.harmonyUserProgressions()).toHaveLength(1);
  });

  it("base com divergência pede decisão e depois insere", async () => {
    await api.projectCreate({ dir: "/tmp", name: "Bases", sampleRate: 48000, bitDepth: 24 });
    const [loop] = await c.basesSearch({ text: "loop" });
    expect(loop?.bpm).toBe(96);
    const first = await c.basesInsert({ baseId: loop!.id, atTick: 0, projectKey: { tonic: 0, mode: "major" } });
    expect(first.status).toBe("needsDecision");
    if (first.status === "needsDecision") {
      expect(first.report.bpmRatio).toBeCloseTo(0.8);
      expect(first.report.modeDiffers).toBe(true);
      expect(first.report.semitoneShift).toBe(0); // Lá menor → relativa de Dó maior
    }
    const done = await c.basesInsert({ baseId: loop!.id, atTick: 0, projectKey: { tonic: 0, mode: "major" }, decision: "adapt" });
    expect(done.status).toBe("inserted");
    if (done.status === "inserted") expect(done.clip.view.tracks).toHaveLength(1);
  });

  it("gerar, salvar e sintetizador", async () => {
    const req = { progression: { kind: "cifra" as const, chords: "C:4 G:4" }, key: C, bpm: 100, bars: 2, style: "waltz" };
    expect((await c.basesGenerate(req)).meter.numerator).toBe(3);
    const b = await c.basesSaveGenerated({ name: "Gerada", generation: req });
    expect(b.kind.kind).toBe("generated");
    await expect(c.basesGenerate({ ...req, bpm: 5 })).rejects.toMatchObject({ code: "library.invalid_bpm" });
    expect((await c.synthSetSoundfont("/x/a.sf2")).engine).toBe("soundfont");
    expect((await c.synthSetSoundfont(null)).engine).toBe("simple");
  });
});
