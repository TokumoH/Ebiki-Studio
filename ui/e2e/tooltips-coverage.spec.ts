import { test, expect } from "@playwright/test";
// Cobertura das legendas: todo controle visível em todos os painéis precisa ter explicação.
test("todo controle visível de todos os painéis tem legenda", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Cob");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "+ Faixa" }).click();
  const missing = new Map<string, Set<string>>();
  const scan = async (where: string) => {
    const r: string[] = await page.evaluate(() => {
      const res = (window as any).__ebikiResolveTip;
      const out: string[] = [];
      document.querySelectorAll("button, select, input, [role=slider], [role=button], .dv-tab").forEach((el) => {
        const b = (el as HTMLElement).getBoundingClientRect();
        if (b.width === 0) return;
        let cur: Element | null = el, ok = false;
        for (let i = 0; cur && i < 6; i++, cur = cur.parentElement) {
          const t = cur.getAttribute("title"); if (t) { cur.setAttribute("data-tip-text", t); cur.removeAttribute("title"); }
          if (res(cur)) { ok = true; break; }
        }
        if (!ok) out.push(`${el.tagName.toLowerCase()} "${(el.getAttribute("aria-label") || (el as HTMLElement).innerText || (el as HTMLInputElement).placeholder || "").slice(0, 40).replace(/\n/g, " ")}"`);
      });
      return out;
    });
    for (const x of r) { if (!missing.has(x)) missing.set(x, new Set()); missing.get(x)!.add(where); }
  };
  await scan("geral");
  for (const tab of ["Mixer", "Gravação", "Mapa de tempo", "Análise da mixagem", "Partitura", "Letra", "Afinação", "Harmonia", "Bases", "Clipe"]) {
    await page.locator(".dv-tab", { hasText: tab }).first().click();
    await page.waitForTimeout(300);
    await scan(tab);
  }
  const report = [...missing].map(([k, v]) => `${[...v].join(",")} :: ${k}`);
  expect(report, "controles sem legenda (adicione ao glossário em src/core/glossary.ts)").toEqual([]);
});
