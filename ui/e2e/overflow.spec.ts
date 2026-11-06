import { test, expect, type Page } from "@playwright/test";
import { PANEL_CATALOG } from "../src/panels/catalog";

// derivado do catálogo: painéis novos entram na verificação automaticamente
const PANELS = PANEL_CATALOG.map((p) => p.title);

/** Elementos que ultrapassam a borda inferior do painel sem um ancestral rolável que os alcance. */
async function clippedInPanels(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const out: string[] = [];
    for (const host of Array.from(document.querySelectorAll<HTMLElement>(".panel-host"))) {
      const hr = host.getBoundingClientRect();
      if (hr.height === 0) continue;
      for (const el of Array.from(host.querySelectorAll<HTMLElement>("*"))) {
        const r = el.getBoundingClientRect();
        if (r.height === 0 || r.bottom <= hr.bottom + 1) continue;
        let scrollable = false;
        for (let a = el.parentElement; a && a !== host.parentElement; a = a.parentElement) {
          const oy = getComputedStyle(a).overflowY;
          // data-scroll-sync: rolagem espelhada por código (ex.: cabeçalhos seguem as trilhas)
          if ((oy === "auto" || oy === "scroll" || a.dataset.scrollSync) && a.getBoundingClientRect().bottom <= hr.bottom + 1) { scrollable = true; break; }
        }
        if (!scrollable) out.push(`${el.tagName.toLowerCase()}.${el.className || ""}<${el.parentElement?.className ?? ""}> h=${Math.round(r.height)} over=${Math.round(r.bottom - hr.bottom)} "${(el.textContent ?? "").trim().slice(0, 30)}"`);
      }
    }
    return out;
  });
}

for (const size of [{ width: 1280, height: 720 }, { width: 1024, height: 640 }]) {
  test(`nenhum painel corta conteúdo sem rolagem em ${size.width}×${size.height}`, async ({ page }) => {
    await page.setViewportSize(size);
    await page.goto("/");
    await page.getByLabel("Nome do projeto").fill("Overflow");
    await page.getByRole("button", { name: "Escolher…" }).click();
    await page.getByRole("button", { name: "Criar projeto" }).click();
    await page.getByRole("button", { name: "Importar", exact: true }).click();
    const problems: string[] = [];
    for (const name of PANELS) {
      await page.locator(".dv-tab").filter({ hasText: new RegExp(`^${name}$`) }).first().click();
      await page.waitForTimeout(150);
      for (const p of await clippedInPanels(page)) problems.push(`${name}: ${p}`);
    }
    expect(problems.slice(0, 15)).toEqual([]);
  });
}
