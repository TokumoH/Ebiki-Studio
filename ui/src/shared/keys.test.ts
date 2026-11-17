import { expect, it } from "vitest";
import { displayKeys } from "./keys";

it("mostra as teclas em PT-BR (UX-13)", () => {
  expect(displayKeys("Space", false)).toBe("Espaço");
  expect(displayKeys("Ctrl+Shift+Z", false)).toBe("Ctrl+Shift+Z");
  expect(displayKeys("ArrowLeft", false)).toBe("←");
  expect(displayKeys("", false)).toBe("");
});

it("no macOS, Ctrl vira ⌘ e Alt vira ⌥, sem o +", () => {
  expect(displayKeys("Ctrl+S", true)).toBe("⌘S");
  expect(displayKeys("Ctrl+Shift+Z", true)).toBe("⌘⇧Z");
  expect(displayKeys("Alt+F", true)).toBe("⌥F");
});
