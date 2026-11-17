import { DEFAULT_SHORTCUTS, eventKeys, findConflict, isActivationKey, isTypingTarget, loadShortcuts, noteKeyboardNavigation, notePointerDown, resetShortcuts, saveShortcut } from "./shortcuts";

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

it("botão focado pelo clique do mouse: Espaço/Enter voltam a ser do atalho global (UX-01)", () => {
  const button = document.createElement("button");
  const icon = document.createElement("span");
  button.appendChild(icon);
  notePointerDown(icon);
  expect(isActivationKey("Space", button)).toBe(false);
  expect(isActivationKey("Enter", button)).toBe(false);
  noteKeyboardNavigation();
  expect(isActivationKey("Space", button)).toBe(true);
  notePointerDown(document.body);
  expect(isActivationKey("Space", button)).toBe(true);
});

it("conflito: avisa quem já usa a tecla; restaurar volta ao padrão (UX-13)", () => {
  const list = loadShortcuts();
  expect(findConflict(list, "metronome", "L")?.id).toBe("loop");
  expect(findConflict(list, "loop", "L")).toBeNull();
  expect(findConflict(list, "metronome", "")).toBeNull();
  saveShortcut("metronome", "M");
  expect(resetShortcuts()).toEqual(DEFAULT_SHORTCUTS);
  expect(loadShortcuts().find((s) => s.id === "metronome")?.keys).toBe("C");
});

it("atalhos padrão sem teclas repetidas e com End, Ctrl+N, Ctrl+O e Delete (UX-13, UX-14)", () => {
  const keys = DEFAULT_SHORTCUTS.map((s) => s.keys);
  expect(new Set(keys).size).toBe(keys.length);
  for (const k of ["End", "Ctrl+N", "Ctrl+O", "Delete"]) expect(keys).toContain(k);
});
