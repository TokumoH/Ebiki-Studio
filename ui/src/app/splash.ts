// Controle da tela de carregamento definida em index.html (fica fora do React para aparecer já).
const MIN_VISIBLE_MS = 700;

export function splashStatus(text: string) {
  const el = document.getElementById("splash-status");
  if (el) el.textContent = text;
}

/** Esconde a tela de carregamento com fade, respeitando um tempo mínimo para não "piscar". */
export function hideSplash(): Promise<void> {
  const el = document.getElementById("splash");
  if (!el) return Promise.resolve();
  const wait = Math.max(0, MIN_VISIBLE_MS - performance.now());
  return new Promise((resolve) => {
    setTimeout(() => {
      el.classList.add("splash-out");
      const done = () => {
        el.remove();
        resolve();
      };
      el.addEventListener("transitionend", done, { once: true });
      setTimeout(done, 500); // sem transição (movimento reduzido): remove mesmo assim
    }, wait);
  });
}
