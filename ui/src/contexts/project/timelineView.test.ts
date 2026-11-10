import { beforeEach, describe, expect, it } from "vitest";
import { anchoredScrollLeft, clampHeight, HEIGHT_MAX, HEIGHT_MIN, trackIndexAt, useTimelineView, wheelFactor, ZOOM_MAX, ZOOM_MIN } from "./timelineView";

const initial = useTimelineView.getState();
beforeEach(() => useTimelineView.setState({ ...initial, heights: {}, baseHeight: 72, pxPerBeat: 40 }, true));

describe("zoom pela roda", () => {
  it("rolar para cima aproxima, para baixo afasta, proporcional ao giro (trackpad suave)", () => {
    expect(wheelFactor(-100, 0)).toBeGreaterThan(1);
    expect(wheelFactor(100, 0)).toBeLessThan(1);
    expect(wheelFactor(-100, 0) * wheelFactor(100, 0)).toBeCloseTo(1); // ida e volta
    expect(Math.abs(wheelFactor(-4, 0) - 1)).toBeLessThan(0.02); // trackpad: passos pequenos
    expect(wheelFactor(-3, 1)).toBeCloseTo(wheelFactor(-100, 0), 1); // deltaMode linhas ≈ pixels
  });
  it("o ponto sob o mouse fica parado na tela", () => {
    // mouse a 300 px da borda, no tique 9600 com 0,1 px/tique → depois do zoom 0,2 px/tique
    const left = anchoredScrollLeft(9600, 0.2, 300);
    expect(9600 * 0.2 - left).toBe(300);
    expect(anchoredScrollLeft(100, 0.2, 300)).toBe(0); // nunca negativo
  });
  it("zoomBy respeita os limites", () => {
    useTimelineView.getState().zoomBy(1000);
    expect(useTimelineView.getState().pxPerBeat).toBe(ZOOM_MAX);
    useTimelineView.getState().zoomBy(0.00001);
    expect(useTimelineView.getState().pxPerBeat).toBe(ZOOM_MIN);
  });
});

describe("altura das faixas", () => {
  it("limita a altura e escala todas (Alt+roda) mantendo as diferenças", () => {
    expect(clampHeight(10)).toBe(HEIGHT_MIN);
    expect(clampHeight(9999)).toBe(HEIGHT_MAX);
    const v = useTimelineView.getState();
    v.setHeight("a", 144);
    v.scaleHeights(1.5);
    const s = useTimelineView.getState();
    expect(s.heightOf("a")).toBe(216);
    expect(s.heightOf("b")).toBe(108); // sem altura própria: segue a base
    s.scaleHeights(100);
    expect(useTimelineView.getState().heightOf("a")).toBe(HEIGHT_MAX);
  });
  it("faixa sob o ponteiro com alturas diferentes", () => {
    const h = [72, 144, 72];
    expect(trackIndexAt(h, 0)).toBe(0);
    expect(trackIndexAt(h, 71)).toBe(0);
    expect(trackIndexAt(h, 72)).toBe(1);
    expect(trackIndexAt(h, 215)).toBe(1);
    expect(trackIndexAt(h, 216)).toBe(2);
    expect(trackIndexAt(h, 5000)).toBe(2); // abaixo da última: a última
    expect(trackIndexAt(h, -10)).toBe(0);
    expect(trackIndexAt([], 10)).toBe(0);
  });
});
