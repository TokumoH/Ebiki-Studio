import { decodeKey, EMPTY_FILTERS, encodeKey, filtersToQuery, parseTags } from "./filters";
import { describeMismatch, insertFlow, type InsertFlow } from "./insertFlow";
import type { MismatchReport } from "../../ipc/types";

describe("filtros → BaseQuery", () => {
  it("vazio vira consulta vazia", () => {
    expect(filtersToQuery(EMPTY_FILTERS)).toEqual({});
  });

  it("limpa texto, limita e ordena BPM, decodifica tom e tags", () => {
    const q = filtersToQuery({ ...EMPTY_FILTERS, text: "  bossa ", bpmMin: "180", bpmMax: "90,5", key: "2:major", genre: "bossa_nova", instrument: "nylonGuitar", tags: "Suave, loop, suave,", kind: "generated" });
    expect(q).toEqual({ text: "bossa", bpmMin: 90.5, bpmMax: 180, key: { tonic: 2, mode: "major" }, genre: "bossa_nova", instrument: "nylonGuitar", tags: ["suave", "loop"], kind: "generated" });
  });

  it("ignora BPM inválido e limita ao intervalo 20–999", () => {
    expect(filtersToQuery({ ...EMPTY_FILTERS, bpmMin: "abc", bpmMax: "5000" })).toEqual({ bpmMax: 999 });
    expect(filtersToQuery({ ...EMPTY_FILTERS, bpmMin: "1" })).toEqual({ bpmMin: 20 });
  });

  it("codifica/decodifica tom e rejeita lixo", () => {
    expect(decodeKey(encodeKey({ tonic: 11, mode: "minor" }))).toEqual({ tonic: 11, mode: "minor" });
    expect(decodeKey("12:major")).toBeUndefined();
    expect(decodeKey("x")).toBeUndefined();
    expect(encodeKey(null)).toBe("");
    expect(parseTags(" , ")).toEqual([]);
  });
});

const diverge: MismatchReport = { bpmRatio: 0.8, semitoneShift: 3, modeDiffers: true, needsDecision: true };
const same: MismatchReport = { bpmRatio: 1, semitoneShift: 0, modeDiffers: false, needsDecision: false };

describe("fluxo de inserção de base", () => {
  it("com divergência, só insere com decisão explícita", () => {
    let s: InsertFlow = insertFlow({ step: "idle" }, { type: "open", baseId: "b" });
    expect(s.step).toBe("checking");
    s = insertFlow(s, { type: "report", report: diverge });
    expect(s.step).toBe("ready");
    expect(insertFlow(s, { type: "choose", decision: null })).toBe(s);
    s = insertFlow(s, { type: "choose", decision: "adapt" });
    expect(s).toMatchObject({ step: "inserting", decision: "adapt" });
    const clip = {} as never;
    s = insertFlow(s, { type: "outcome", outcome: { status: "inserted", report: diverge, decision: "adapt", clip } });
    expect(s).toEqual({ step: "done", baseId: "b", decision: "adapt" });
  });

  it("sem divergência insere direto; needsDecision do núcleo volta a perguntar", () => {
    let s: InsertFlow = insertFlow(insertFlow({ step: "idle" }, { type: "open", baseId: "b" }), { type: "report", report: same });
    s = insertFlow(s, { type: "choose", decision: null });
    expect(s.step).toBe("inserting");
    s = insertFlow(s, { type: "outcome", outcome: { status: "needsDecision", report: diverge } });
    expect(s).toMatchObject({ step: "ready", report: diverge });
  });

  it("trocar o tom recalcula; erro e fechar", () => {
    let s: InsertFlow = insertFlow(insertFlow({ step: "idle" }, { type: "open", baseId: "b" }), { type: "report", report: same });
    s = insertFlow(s, { type: "recheck" });
    expect(s.step).toBe("checking");
    s = insertFlow(s, { type: "fail", message: "x" });
    expect(s).toEqual({ step: "error", baseId: "b", message: "x" });
    expect(insertFlow(s, { type: "close" })).toEqual({ step: "idle" });
    expect(insertFlow({ step: "idle" }, { type: "fail", message: "x" })).toEqual({ step: "idle" });
  });

  it("descreve a divergência em PT-BR", () => {
    const lines = describeMismatch(diverge, 96);
    expect(lines[0]).toContain("base 96 BPM → projeto 120 BPM");
    expect(lines[1]).toBe("Tom: 3 semitons acima para chegar ao projeto");
    expect(lines[2]).toContain("Modo diferente");
    expect(describeMismatch({ ...same, semitoneShift: null }, 120)).toEqual(["Andamento igual (120 BPM)", "Tom: não comparado (tom da base ou do projeto desconhecido)"]);
    expect(describeMismatch({ ...diverge, semitoneShift: -1, modeDiffers: false }, 96)[1]).toBe("Tom: 1 semitom abaixo para chegar ao projeto");
  });
});
