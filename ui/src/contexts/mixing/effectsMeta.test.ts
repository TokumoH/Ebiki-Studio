import { defaultEffect, PARAMS, EFFECT_LABELS } from "./effectsMeta";
import * as S from "../../ipc/schema";

it("rótulos visíveis em PT-BR (Mistura/Saturação; Reverb/Delay como no manual)", () => {
  const labels = Object.values(PARAMS).flatMap((ps) => ps.map((p) => p.label));
  expect(labels).not.toContain("Mix");
  expect(labels).not.toContain("Drive");
  expect(PARAMS.reverb?.find((p) => p.key === "mix")?.label).toBe("Mistura");
  expect(PARAMS.saturation?.find((p) => p.key === "driveDb")?.label).toBe("Saturação");
  expect(EFFECT_LABELS.reverb).toBe("Reverb");
  expect(EFFECT_LABELS.delay).toBe("Delay");
});

it("todo efeito padrão é válido pelo contrato e tem metadados coerentes", () => {
  for (const type of Object.keys(EFFECT_LABELS)) {
    const e = defaultEffect(type);
    expect(S.Effect.parse(e).type).toBe(type);
    for (const p of PARAMS[type] ?? []) {
      const v = (e.params as Record<string, number>)[p.key];
      expect(v, `${type}.${p.key}`).toBeGreaterThanOrEqual(p.min);
      expect(v, `${type}.${p.key}`).toBeLessThanOrEqual(p.max);
    }
  }
});
