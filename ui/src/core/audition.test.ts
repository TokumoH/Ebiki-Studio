import { setTransport, type Transport } from "../ipc/transport";
import { createMockTransport } from "../ipc/mock";
import { audition, stopAudition, useAudition } from "./audition";

/** Transporte simulado que registra os comandos (S-3: audição toca no motor, não na WebAudio). */
function spy(): { calls: string[] } {
  const base = createMockTransport();
  const calls: string[] = [];
  const t: Transport = { ...base, kind: base.kind, invoke: <T,>(cmd: string, args?: Record<string, unknown>) => (calls.push(cmd), base.invoke<T>(cmd, args)) };
  setTransport(t);
  return { calls };
}

describe("audição pelo motor (S-3)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useAudition.setState({ playing: null, loading: null });
  });
  afterEach(() => vi.useRealTimers());

  it("marca 'tocando' pela duração devolvida pelo núcleo e depois libera", async () => {
    spy();
    await audition("prog:a", async () => ({ seconds: 2 }));
    expect(useAudition.getState()).toMatchObject({ playing: "prog:a", loading: null });
    vi.advanceTimersByTime(2000);
    expect(useAudition.getState().playing).toBe("prog:a"); // margem para o fim sair no fone
    vi.advanceTimersByTime(200);
    expect(useAudition.getState().playing).toBeNull();
  });

  it("Parar pede ao motor que pare e limpa o estado", async () => {
    const { calls } = spy();
    await audition("base:1", async () => ({ seconds: 5 }));
    stopAudition();
    expect(useAudition.getState()).toEqual({ playing: null, loading: null });
    expect(calls).toContain("audition_stop");
    vi.advanceTimersByTime(6000);
    expect(useAudition.getState().playing).toBeNull();
  });

  it("Parar sem nada tocando não chama o motor", () => {
    const { calls } = spy();
    stopAudition();
    expect(calls).not.toContain("audition_stop");
  });

  it("uma nova audição substitui a anterior sem que o temporizador antigo a apague", async () => {
    spy();
    await audition("a", async () => ({ seconds: 1 }));
    await audition("b", async () => ({ seconds: 3 }));
    vi.advanceTimersByTime(1200);
    expect(useAudition.getState().playing).toBe("b");
    vi.advanceTimersByTime(2000);
    expect(useAudition.getState().playing).toBeNull();
  });

  it("parada durante o preparo descarta o resultado e para o motor de novo", async () => {
    const { calls } = spy();
    let release: (v: { seconds: number }) => void = () => undefined;
    const p = audition("x", () => new Promise((r) => (release = r)));
    expect(useAudition.getState().loading).toBe("x");
    stopAudition();
    release({ seconds: 1 });
    await p;
    expect(useAudition.getState()).toEqual({ playing: null, loading: null });
    expect(calls.filter((c) => c === "audition_stop")).toHaveLength(2); // no clique e depois que o motor começou
  });

  it("erro do núcleo é propagado e limpa o estado", async () => {
    spy();
    await expect(audition("y", async () => Promise.reject(new Error("falhou")))).rejects.toThrow("falhou");
    expect(useAudition.getState()).toEqual({ playing: null, loading: null });
  });
});
