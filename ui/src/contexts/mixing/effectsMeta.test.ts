import { defaultEffect, PARAMS, EFFECT_LABELS } from "./effectsMeta";
import * as S from "../../ipc/schema";

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
