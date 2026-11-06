import { act, fireEvent, render, screen } from "@testing-library/react";
import { TOUR_STEPS, Tutorial, cardPosition, setTutorialDone, shouldAutoStartTutorial, tutorialDone, useTour } from "./Tutorial";
import { APP_VERSION } from "./about";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

beforeEach(() => {
  localStorage.clear();
  act(() => useTour.getState().stop());
});

describe("posição do cartão do tutorial", () => {
  const vp = { width: 1280, height: 720 };
  it("abaixo do alvo quando cabe; acima quando não cabe; centralizado sem alvo", () => {
    expect(cardPosition({ top: 100, left: 50, width: 400, height: 100 }, vp, 200)).toEqual({ top: 214, left: 50 });
    expect(cardPosition({ top: 500, left: 50, width: 400, height: 150 }, vp, 200).top).toBe(286);
    expect(cardPosition(null, vp, 200)).toEqual({ top: 260, left: 450 });
  });
  it("nunca sai da tela pela direita", () => {
    expect(cardPosition({ top: 10, left: 1200, width: 60, height: 20 }, vp, 100).left).toBe(1280 - 380 - 16);
  });
});

describe("tutorial", () => {
  it("navega com botões e teclado, e pular lembra a escolha", () => {
    render(<Tutorial />);
    act(() => useTour.getState().start(1));
    expect(screen.getByRole("dialog", { name: `Tutorial: passo 1 de ${TOUR_STEPS.length}` })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Próximo" }));
    expect(useTour.getState().step).toBe(2);
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(useTour.getState().step).toBe(3);
    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(useTour.getState().step).toBe(2);
    fireEvent.click(screen.getByRole("button", { name: "Pular tutorial" }));
    expect(useTour.getState().step).toBeNull();
    expect(tutorialDone()).toBe(true);
  });

  it("desmarcar 'não mostrar ao abrir' faz o tour voltar na próxima vez", () => {
    render(<Tutorial />);
    act(() => useTour.getState().start(TOUR_STEPS.length));
    fireEvent.click(screen.getByRole("checkbox", { name: /Não mostrar ao abrir/ }));
    fireEvent.click(screen.getByRole("button", { name: "Concluir" }));
    expect(tutorialDone()).toBe(false);
  });

  it("não abre sozinho em testes nem depois de concluído", () => {
    expect(shouldAutoStartTutorial()).toBe(false);
    setTutorialDone(true);
    expect(shouldAutoStartTutorial()).toBe(false);
  });
});

it("a versão mostrada na interface é a do app (tauri.conf.json)", () => {
  const conf = JSON.parse(readFileSync(resolve(__dirname, "../../../crates/app-desktop/tauri.conf.json"), "utf8")) as { version: string };
  expect(APP_VERSION).toBe(conf.version);
});
