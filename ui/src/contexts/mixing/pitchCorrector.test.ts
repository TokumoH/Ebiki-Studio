import { defaultEffect, EFFECT_LABELS, maskIncludes, maskPreset, scaleMask, toggleMaskBit } from "./effectsMeta";
import * as S from "../../ipc/schema";

describe("PitchCorrector no mixer", () => {
  it("rótulo PT-BR e efeito padrão válido pelo contrato", () => {
    expect(EFFECT_LABELS.pitchCorrector).toBe("Correção de afinação");
    const e = defaultEffect("pitchCorrector");
    expect(S.PitchCorrectorParams.parse(e.params)).toMatchObject({ enabled: true, scaleMask: 0x0fff, a4Hz: 440 });
  });
  it("máscara de 12 bits igual à do núcleo (Sol maior = 0b1010_1101_0101)", () => {
    expect(scaleMask(7, "major")).toBe(0b1010_1101_0101);
    expect(scaleMask(0, "major")).toBe(0b1010_1011_0101);
    expect(maskPreset(scaleMask(9, "minor"), 9)).toBe("minor");
    expect(maskPreset(scaleMask(0, "major"), 2)).toBeNull();
  });
  it("alternar classes mantém a tônica", () => {
    let m = scaleMask(0, "major");
    m = toggleMaskBit(m, 1, 0);
    expect(maskIncludes(m, 1)).toBe(true);
    m = toggleMaskBit(m, 0, 0);
    expect(maskIncludes(m, 0)).toBe(true);
    expect(toggleMaskBit(m, 1, 0)).toBe(scaleMask(0, "major"));
  });
});
