import { describe, expect, it } from "vitest";
import { dbFraction, decayPeak, levelZone, meterFraction, parseRouteKey, peakDb, routeKey, routeOptions, TARGET_ZONE_DB } from "./inputRoutes";

describe("rotas de entrada", () => {
  it("mono por canal e pares estéreo", () => {
    expect(routeOptions(4).map((o) => o.label)).toEqual([
      "Entrada 1 (mono)",
      "Entrada 2 (mono)",
      "Entrada 3 (mono)",
      "Entrada 4 (mono)",
      "Entradas 1/2 (estéreo)",
      "Entradas 3/4 (estéreo)",
    ]);
    expect(routeOptions(1, 1).map((o) => o.value)).toEqual(["0:"]);
    expect(routeOptions(0).length).toBe(3); // mínimo de 2 canais para faixas sem dispositivo
  });
  it("chave ida e volta", () => {
    for (const r of [{ left: 0, right: null }, { left: 2, right: 3 }]) expect(parseRouteKey(routeKey(r))).toEqual(r);
    expect(parseRouteKey("")).toBeNull();
    expect(routeKey(null)).toBe("");
  });
});

describe("medidor", () => {
  it("converte pico em dBFS com piso de −60", () => {
    expect(peakDb(1)).toBeCloseTo(0);
    expect(peakDb(0.5)).toBeCloseTo(-6.02, 1);
    expect(peakDb(0)).toBe(-60);
    expect(meterFraction(1)).toBe(1);
    expect(meterFraction(0)).toBe(0);
    expect(meterFraction(2)).toBe(1);
  });
  it("sobe na hora e desce aos poucos", () => {
    expect(decayPeak(0.1, 0.8)).toBe(0.8);
    expect(decayPeak(0.8, 0)).toBeCloseTo(0.68);
  });
  it("zonas: fraco abaixo de −30, alvo verde de −18 a −6, muito alto acima de −1", () => {
    expect(levelZone(-60)).toBe("low");
    expect(levelZone(-30.5)).toBe("low");
    expect(levelZone(-30)).toBe("quiet");
    expect(levelZone(-18)).toBe("target");
    expect(levelZone(-6)).toBe("target");
    expect(levelZone(-5.9)).toBe("loud");
    expect(levelZone(-1)).toBe("loud");
    expect(levelZone(-0.5)).toBe("clip");
    expect(TARGET_ZONE_DB).toEqual([-18, -6]);
    expect(dbFraction(-18)).toBeCloseTo(0.7);
    expect(dbFraction(-6)).toBeCloseTo(0.9);
    expect(dbFraction(-90)).toBe(0);
    expect(dbFraction(6)).toBe(1);
  });
});
