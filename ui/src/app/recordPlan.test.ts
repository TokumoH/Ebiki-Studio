import { expect, it } from "vitest";
import type { SessionView } from "../ipc/types";
import { recordPlan } from "./recordPlan";

const track = (id: string, armed: boolean, input: { left: number; right: number | null } | null) =>
  ({ id, name: `Faixa ${id}`, recording: { armed, input } }) as unknown as SessionView["tracks"][number];
const session = (...tracks: ReturnType<typeof track>[]) => ({ tracks }) as unknown as SessionView;

it("faixa armada com entrada: grava", () => {
  expect(recordPlan(session(track("a", true, { left: 0, right: null })), null, 2)).toEqual({ kind: "record" });
});

it("nada armado, microfone presente e faixa selecionada: arma a selecionada", () => {
  expect(recordPlan(session(track("a", false, null), track("b", false, null)), "b", 2)).toEqual({ kind: "arm", trackId: "b", trackName: "Faixa b" });
});

it("sem microfone: explica o que falta", () => {
  const p = recordPlan(session(track("a", false, null)), "a", 0);
  expect(p.kind).toBe("setup");
});

it("sem faixa selecionada ou sem projeto: explica o que falta", () => {
  expect(recordPlan(session(), null, 2).kind).toBe("setup");
  expect(recordPlan(null, null, 2).kind).toBe("setup");
  // armada sem entrada não conta como pronta
  expect(recordPlan(session(track("a", true, null)), "a", 2)).toEqual({ kind: "arm", trackId: "a", trackName: "Faixa a" });
});
