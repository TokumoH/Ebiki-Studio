// Metadados de exibição dos efeitos (rótulos, faixas e unidades) — espelham os limites do domínio.
import type { Effect } from "../../ipc/types";

export interface ParamMeta {
  key: string;
  label: string;
  min: number;
  max: number;
  step: number;
  unit: string;
}

export const EFFECT_LABELS: Record<string, string> = {
  eq: "Equalizador",
  compressor: "Compressor",
  gate: "Gate",
  limiter: "Limitador",
  reverb: "Reverb",
  delay: "Delay",
  saturation: "Saturação",
  chorus: "Chorus",
  deEsser: "De-esser",
};

export const PARAMS: Record<string, ParamMeta[]> = {
  compressor: [
    { key: "thresholdDb", label: "Limiar", min: -60, max: 0, step: 0.5, unit: "dB" },
    { key: "ratio", label: "Razão", min: 1, max: 20, step: 0.1, unit: ":1" },
    { key: "attackMs", label: "Ataque", min: 0.1, max: 200, step: 0.1, unit: "ms" },
    { key: "releaseMs", label: "Liberação", min: 5, max: 2000, step: 1, unit: "ms" },
    { key: "kneeDb", label: "Joelho", min: 0, max: 24, step: 0.5, unit: "dB" },
    { key: "makeupDb", label: "Ganho", min: 0, max: 24, step: 0.5, unit: "dB" },
  ],
  gate: [
    { key: "thresholdDb", label: "Limiar", min: -90, max: 0, step: 0.5, unit: "dB" },
    { key: "attackMs", label: "Ataque", min: 0.05, max: 50, step: 0.05, unit: "ms" },
    { key: "holdMs", label: "Retenção", min: 0, max: 500, step: 1, unit: "ms" },
    { key: "releaseMs", label: "Liberação", min: 5, max: 2000, step: 1, unit: "ms" },
    { key: "rangeDb", label: "Atenuação", min: -90, max: 0, step: 1, unit: "dB" },
  ],
  limiter: [
    { key: "ceilingDb", label: "Teto", min: -24, max: 0, step: 0.1, unit: "dB" },
    { key: "inputGainDb", label: "Ganho de entrada", min: 0, max: 24, step: 0.1, unit: "dB" },
    { key: "releaseMs", label: "Liberação", min: 1, max: 1000, step: 1, unit: "ms" },
  ],
  reverb: [
    { key: "roomSize", label: "Tamanho", min: 0, max: 1, step: 0.01, unit: "" },
    { key: "damping", label: "Amortecimento", min: 0, max: 1, step: 0.01, unit: "" },
    { key: "width", label: "Largura", min: 0, max: 1, step: 0.01, unit: "" },
    { key: "predelayMs", label: "Pré-atraso", min: 0, max: 200, step: 1, unit: "ms" },
    { key: "mix", label: "Mix", min: 0, max: 1, step: 0.01, unit: "" },
  ],
  delay: [
    { key: "timeMs", label: "Tempo", min: 1, max: 2000, step: 1, unit: "ms" },
    { key: "feedback", label: "Realimentação", min: 0, max: 0.95, step: 0.01, unit: "" },
    { key: "highCutHz", label: "Corte de agudos", min: 500, max: 20000, step: 10, unit: "Hz" },
    { key: "mix", label: "Mix", min: 0, max: 1, step: 0.01, unit: "" },
  ],
  saturation: [
    { key: "driveDb", label: "Drive", min: 0, max: 36, step: 0.5, unit: "dB" },
    { key: "tone", label: "Tom", min: 0, max: 1, step: 0.01, unit: "" },
    { key: "mix", label: "Mix", min: 0, max: 1, step: 0.01, unit: "" },
  ],
  chorus: [
    { key: "rateHz", label: "Velocidade", min: 0.05, max: 10, step: 0.05, unit: "Hz" },
    { key: "depthMs", label: "Profundidade", min: 0.1, max: 15, step: 0.1, unit: "ms" },
    { key: "mix", label: "Mix", min: 0, max: 1, step: 0.01, unit: "" },
  ],
  deEsser: [
    { key: "freqHz", label: "Frequência", min: 2000, max: 12000, step: 50, unit: "Hz" },
    { key: "thresholdDb", label: "Limiar", min: -60, max: 0, step: 0.5, unit: "dB" },
    { key: "reductionDb", label: "Redução máx.", min: -24, max: 0, step: 0.5, unit: "dB" },
  ],
};

export function defaultEffect(type: string): Effect {
  switch (type) {
    case "eq":
      return {
        type: "eq",
        params: {
          bands: [
            { kind: "highPass", enabled: false, freqHz: 80, gainDb: 0, q: 0.707 },
            { kind: "lowShelf", enabled: true, freqHz: 120, gainDb: 0, q: 0.707 },
            { kind: "peak", enabled: true, freqHz: 1000, gainDb: 0, q: 1 },
            { kind: "highShelf", enabled: true, freqHz: 8000, gainDb: 0, q: 0.707 },
          ],
        },
      };
    case "compressor":
      return { type, params: { thresholdDb: -18, ratio: 3, attackMs: 10, releaseMs: 120, kneeDb: 6, makeupDb: 0 } };
    case "gate":
      return { type, params: { thresholdDb: -50, attackMs: 1, holdMs: 30, releaseMs: 150, rangeDb: -60 } };
    case "limiter":
      return { type, params: { ceilingDb: -1, inputGainDb: 0, releaseMs: 80 } };
    case "reverb":
      return { type, params: { roomSize: 0.6, damping: 0.5, width: 1, predelayMs: 20, mix: 0.25 } };
    case "delay":
      return { type, params: { timeMs: 375, feedback: 0.35, highCutHz: 6000, mix: 0.2, pingPong: false } };
    case "saturation":
      return { type, params: { driveDb: 6, tone: 0.5, mix: 1 } };
    case "chorus":
      return { type, params: { rateHz: 0.8, depthMs: 3, mix: 0.4 } };
    default:
      return { type: "deEsser", params: { freqHz: 6500, thresholdDb: -28, reductionDb: -8 } };
  }
}
