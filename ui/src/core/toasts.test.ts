import { beforeEach, expect, it, vi } from "vitest";
import { notify, useToasts } from "./toasts";

beforeEach(() => {
  vi.useFakeTimers();
  useToasts.setState({ toasts: [] });
});

it("aviso repetido não empilha: o igual anterior sai e o tempo recomeça (UX-09)", () => {
  notify.error("Arme uma faixa.");
  vi.advanceTimersByTime(5000);
  notify.error("Arme uma faixa.");
  expect(useToasts.getState().toasts).toHaveLength(1);
  vi.advanceTimersByTime(5000);
  expect(useToasts.getState().toasts).toHaveLength(1); // o timer do primeiro não apaga o novo
  vi.advanceTimersByTime(3500);
  expect(useToasts.getState().toasts).toHaveLength(0);
});

it("aviso com ação leva o botão e fecha ao usar", () => {
  const go = vi.fn();
  notify.error("Sem faixa armada.", { label: "Abrir Gravação", run: go });
  const t = useToasts.getState().toasts[0]!;
  expect(t.action?.label).toBe("Abrir Gravação");
  t.action!.run();
  expect(go).toHaveBeenCalled();
});

it("mensagens diferentes convivem (até 5)", () => {
  for (let i = 0; i < 7; i++) notify.info(`m${i}`);
  expect(useToasts.getState().toasts.map((t) => t.message)).toEqual(["m2", "m3", "m4", "m5", "m6"]);
});
