import { latencyEstimateSamples } from "./audio";
import type { AudioStatus } from "../ipc/types";

const status = (over: Partial<AudioStatus>): AudioStatus => ({
  mode: "device", outputName: "Fone", inputName: null, outputId: null, inputId: null, sampleRate: 48000, bufferSize: null,
  inputChannels: 0, estimatedLatencyMs: null, inputGainDb: 0, error: null, warning: null, callbackFrames: null, deviceSampleRate: null,
  ...over,
});

describe("latencyEstimateSamples", () => {
  it("entrada + saída pelo buffer fixo ou, no padrão do driver, pelo callback real (AU-04)", () => {
    expect(latencyEstimateSamples(status({ bufferSize: 256 }))).toBe(512);
    expect(latencyEstimateSamples(status({ callbackFrames: 480 }))).toBe(960);
    expect(latencyEstimateSamples(status({ deviceSampleRate: 48000, callbackFrames: 480 }))).toBe(960);
  });

  it("fone convertido (S-1): quadros do dispositivo viram amostras da taxa do projeto", () => {
    // projeto a 48 kHz, fone a 44,1 kHz: 2 × 441 quadros = 20 ms = 960 amostras do projeto
    expect(latencyEstimateSamples(status({ deviceSampleRate: 44100, callbackFrames: 441 }))).toBe(960);
    // fone em modo chamada (16 kHz): 2 × 160 quadros = 20 ms = 960
    expect(latencyEstimateSamples(status({ deviceSampleRate: 16000, bufferSize: 160 }))).toBe(960);
  });

  it("desconhecido: 0 (sem botão de estimativa)", () => {
    expect(latencyEstimateSamples(null)).toBe(0);
    expect(latencyEstimateSamples(status({}))).toBe(0);
  });
});
