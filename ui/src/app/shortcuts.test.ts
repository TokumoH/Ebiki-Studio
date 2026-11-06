import { eventKeys, isActivationKey, isTypingTarget, loadShortcuts, saveShortcut } from "./shortcuts";

it("normaliza combinações", () => {
  expect(eventKeys({ key: "z", ctrlKey: true, metaKey: false, shiftKey: true, altKey: false })).toBe("Ctrl+Shift+Z");
  expect(eventKeys({ key: " ", ctrlKey: false, metaKey: false, shiftKey: false, altKey: false })).toBe("Space");
  expect(eventKeys({ key: "s", ctrlKey: false, metaKey: true, shiftKey: false, altKey: false })).toBe("Ctrl+S");
});

it("não dispara atalho ao digitar", () => {
  expect(isTypingTarget(document.createElement("input"))).toBe(true);
  expect(isTypingTarget(document.createElement("div"))).toBe(false);
});

it("Enter/Espaço em botões e itens de menu ativam o controle, não o atalho global", () => {
  const button = document.createElement("button");
  const item = document.createElement("div");
  item.setAttribute("role", "menuitemcheckbox");
  const div = document.createElement("div");
  expect(isActivationKey("Enter", button)).toBe(true);
  expect(isActivationKey("Space", button)).toBe(true);
  expect(isActivationKey("Enter", item)).toBe(true);
  expect(isActivationKey("Enter", div)).toBe(false);
  expect(isActivationKey("Space", document.body)).toBe(false);
  expect(isActivationKey("L", button)).toBe(false);
});

it("atalhos são editáveis e persistem", () => {
  saveShortcut("metronome", "M");
  expect(loadShortcuts().find((s) => s.id === "metronome")?.keys).toBe("M");
  localStorage.clear();
  expect(loadShortcuts().find((s) => s.id === "metronome")?.keys).toBe("C");
});
