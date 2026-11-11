import { describe, expect, it } from "vitest";
import { popoverPosition } from "./LoopPopover";

describe("posição das opções de loop", () => {
  it("abaixo do botão, alinhada à direita dele e presa à janela", () => {
    expect(popoverPosition({ left: 500, bottom: 70, width: 30 }, 1280)).toEqual({ top: 76, left: 200 });
    expect(popoverPosition({ left: 100, bottom: 70, width: 30 }, 1280).left).toBe(8); // não sai pela esquerda
    expect(popoverPosition({ left: 1270, bottom: 70, width: 30 }, 1280).left).toBe(1280 - 330 - 8); // nem pela direita
  });
});
