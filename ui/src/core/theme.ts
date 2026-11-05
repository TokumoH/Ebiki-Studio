import { create } from "zustand";

type Theme = "dark" | "light";

function load(): Theme {
  try {
    return localStorage.getItem("appmusica.theme") === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

export const useTheme = create<{ theme: Theme; toggle: () => void }>((set, get) => ({
  theme: load(),
  toggle: () => {
    const theme: Theme = get().theme === "dark" ? "light" : "dark";
    try {
      localStorage.setItem("appmusica.theme", theme);
    } catch {
      /* armazenamento indisponível */
    }
    set({ theme });
  },
}));

export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
}
