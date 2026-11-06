import { setTransport } from "./transport";
import { createMockTransport } from "./mock";
import { api } from "./api";

describe("transporte simulado cumpre o contrato da API", () => {
  beforeEach(() => setTransport(createMockTransport()));

  it("fluxo: criar, importar, mixar, desfazer, salvar", async () => {
    let v = await api.projectCreate({ dir: "/tmp", name: "Teste", sampleRate: 48000, bitDepth: 24 });
    expect(v.history.dirty).toBe(false);
    v = await api.audioImport(["/x/voz.wav"], null, 0);
    expect(v.tracks).toHaveLength(1);
    const t = v.tracks[0]!;
    for (const db of [-1, -3, -6]) v = await api.mixerEdit({ kind: "track", id: t.id }, { op: "volume", value: db }, 7);
    expect(v.tracks[0]!.strip.volume).toBe(-6);
    expect(v.history.undoLabel).toBe("Volume");
    v = await api.undo();
    expect(v.tracks[0]!.strip.volume).toBe(0);
    expect(v.history.dirty).toBe(true);
    v = await api.projectSave();
    expect(v.history.dirty).toBe(false);
  });

  it("erros chegam tipados", async () => {
    await expect(api.projectView()).rejects.toMatchObject({ code: "app.no_project" });
    await api.projectCreate({ dir: "/tmp", name: "A", sampleRate: 48000, bitDepth: 24 });
    await expect(api.trackCreate("   ")).rejects.toMatchObject({ code: "project.invalid_name" });
  });

  it("forma de onda vem em binário", async () => {
    const v = await api.projectCreate({ dir: "/tmp", name: "B", sampleRate: 48000, bitDepth: 24 });
    const v2 = await api.audioImport(["/x/a.wav"], null, 0);
    const peaks = await api.waveform(v2.sources[0]!.id, 1024);
    // 8 s a 48 kHz = 384000 quadros → ceil(384000 / 1024) = 375 picos (mín., máx.) = 750 valores
    expect(peaks).toHaveLength(750);
    expect(peaks[0]).toBeLessThan(0);
    expect(peaks[1]).toBeCloseTo(-peaks[0]!, 6);
    expect(v.project.name).toBe("B");
  });
});
