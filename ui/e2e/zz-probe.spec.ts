import { test, expect } from "@playwright/test";
test("probe", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("P");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator(".dv-tab", { hasText: "Partitura" }).click();
  await page.getByRole("button", { name: "+ Nova partitura" }).click();
  await page.getByRole("button", { name: "Criar partitura" }).click();
  for (const k of ["5", "c", "d", "e", "f"]) await page.keyboard.press(k);
  await expect(page.locator(".score-at svg").first()).toBeVisible({ timeout: 15_000 });
  await page.waitForTimeout(1500);
  const info = await page.evaluate(() => {
    const host = document.querySelector(".score-at")!;
    const hr = host.getBoundingClientRect();
    const svgs = [...host.querySelectorAll("svg")];
    const out: string[] = [`svgs=${svgs.length}`];
    for (const svg of svgs.slice(0, 3)) {
      const els = [...svg.querySelectorAll("*")];
      const tags: Record<string, number> = {};
      els.forEach((e) => (tags[e.tagName] = (tags[e.tagName] ?? 0) + 1));
      out.push(JSON.stringify(tags));
      const lines = els.filter((e) => { const b = (e as SVGGraphicsElement).getBoundingClientRect(); return b.height <= 2 && b.width > 40; })
        .map((e) => { const b = (e as SVGGraphicsElement).getBoundingClientRect(); return `${e.tagName} x=${Math.round(b.x - hr.x)} y=${(b.y - hr.y).toFixed(1)} w=${Math.round(b.width)} cls=${e.getAttribute("class")}`; });
      out.push(...lines.slice(0, 20));
      const texts = els.filter((e) => e.tagName === "text").slice(0, 8).map((e) => { const b = (e as SVGGraphicsElement).getBoundingClientRect(); return `text "${e.textContent?.codePointAt(0)?.toString(16)}" x=${Math.round(b.x - hr.x)} y=${(b.y-hr.y).toFixed(1)} h=${b.height.toFixed(1)}`; });
      out.push(...texts);
    }
    return out.join("\n");
  });
  console.log(info);
});
