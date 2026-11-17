import { beforeEach, describe, expect, it } from "vitest";
import { followScrollLeft, isRolling, useFollow } from "./follow";

describe("followScrollLeft (vira a página)", () => {
  it("não mexe enquanto a linha está entre o início e 90% da vista", () => {
    expect(followScrollLeft(500, 0, 1200)).toBeNull();
    expect(followScrollLeft(1079, 0, 1200)).toBeNull();
  });
  it("ao passar de 90%, deixa a linha em 10%", () => {
    expect(followScrollLeft(1080, 0, 1200)).toBe(960);
  });
  it("linha antes da vista (fim do loop, voltar ao início): vai até ela", () => {
    expect(followScrollLeft(100, 2000, 1200)).toBe(0);
    expect(followScrollLeft(3000, 5000, 1000)).toBe(2900);
  });
  it("linha muito à frente (fora da tela ao dar play): vai até ela", () => {
    expect(followScrollLeft(9000, 0, 1000)).toBe(8900);
  });
  it("vista sem largura ou valores inválidos: não rola", () => {
    expect(followScrollLeft(500, 0, 0)).toBeNull();
    expect(followScrollLeft(Number.NaN, 0, 1000)).toBeNull();
  });
});

describe("isRolling", () => {
  it("tocando, gravando ou contagem", () => {
    expect(isRolling("playing")).toBe(true);
    expect(isRolling("recording")).toBe(true);
    expect(isRolling("countin")).toBe(true);
    expect(isRolling("paused")).toBe(false);
    expect(isRolling("stopped")).toBe(false);
    expect(isRolling(undefined)).toBe(false);
  });
});

describe("useFollow", () => {
  beforeEach(() => {
    localStorage.clear();
    useFollow.setState({ enabled: true, suspended: false });
  });
  it("liga por padrão e alterna lembrando a escolha", () => {
    expect(useFollow.getState().enabled).toBe(true);
    useFollow.getState().toggle();
    expect(useFollow.getState().enabled).toBe(false);
    expect(localStorage.getItem("ebiki.follow.v1")).toBe("0");
  });
  it("religar tira a pausa causada por rolagem manual", () => {
    useFollow.getState().suspend();
    useFollow.getState().toggle();
    useFollow.getState().toggle();
    expect(useFollow.getState().suspended).toBe(false);
  });
});
