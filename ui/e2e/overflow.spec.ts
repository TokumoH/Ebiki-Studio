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

test("cabeçalho de faixa mostra o nome e os controles numa linha só", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Cabecalho");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  const header = page.getByTestId("track-header-0");
  await expect(header.locator(".name")).toHaveText("voz");
  const sizes = await header.evaluate((h) => ({
    name: h.querySelector(".name")!.getBoundingClientRect().height,
    row: h.querySelector(".row")!.getBoundingClientRect().height,
    bottom: Math.max(...[...h.querySelectorAll("button")].map((b) => b.getBoundingClientRect().bottom)),
    limit: h.getBoundingClientRect().bottom,
  }));
  expect(sizes.name, "nome legível (não esmagado)").toBeGreaterThanOrEqual(12);
  expect(sizes.row, "controles em uma linha").toBeLessThanOrEqual(30);
  expect(sizes.bottom, "nenhum botão fora do cabeçalho").toBeLessThanOrEqual(sizes.limit);
});

/** Botões da barra superior e itens do menu: dentro da janela, com largura útil e sem nada por cima. */
async function blockedTopControls(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const out: string[] = [];
    const els = Array.from(document.querySelectorAll<HTMLElement>(".topbar button, .menubar-row [role=menuitem]"));
    for (const el of els) {
      if (getComputedStyle(el).display === "none" || el.offsetParent === null) continue;
      const r = el.getBoundingClientRect();
      const name = el.getAttribute("aria-label") || (el.textContent ?? "").trim().slice(0, 20);
      if (r.width < 22) out.push(`${name}: estreito (${Math.round(r.width)} px)`);
      if (r.left < 0 || r.right > window.innerWidth + 0.5) out.push(`${name}: fora da janela`);
      const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      if (hit && !el.contains(hit)) out.push(`${name}: coberto por ${hit.tagName.toLowerCase()}.${hit.className}`);
    }
    return out;
  });
}

for (const size of [{ width: 640, height: 360 }, { width: 800, height: 600 }, { width: 1024, height: 640 }]) {
  test(`barra superior e menu utilizáveis em ${size.width}×${size.height} (UX-02)`, async ({ page }) => {
    await page.setViewportSize(size);
    await page.goto("/");
    await page.getByLabel("Nome do projeto").fill("Barra");
    await page.getByRole("button", { name: "Escolher…" }).click();
    await page.getByRole("button", { name: "Criar projeto" }).click();
    await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
    expect(await blockedTopControls(page)).toEqual([]);
    // tocar, parar e gravar sempre ao alcance
    for (const n of ["Tocar", "Parar", "Gravar"]) await expect(page.getByRole("button", { name: n, exact: true })).toBeVisible();
    // Ajuda inteira no menu
    await expect(page.locator(".menubar-row [role=menuitem]", { hasText: "Ajuda" })).toBeVisible();
  });
}

test("diálogo alto: título e botões ficam à vista, só o corpo rola (UX-07)", async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 480 });
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Dialogo");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator("body").click({ position: { x: 5, y: 300 } });
  await page.keyboard.press("Control+/");
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  const close = dialog.locator("footer button").last();
  const box = await close.boundingBox();
  expect(box!.y + box!.height).toBeLessThanOrEqual(480);
  const body = await dialog.locator(".dialog-body").evaluate((el) => ({ sh: el.scrollHeight, ch: el.clientHeight, oy: getComputedStyle(el).overflowY }));
  expect(body.oy).toBe("auto");
});

test("Arranjo continua com a maior parte da tela em janela baixa (UX-03)", async ({ page }) => {
  await page.setViewportSize({ width: 853, height: 480 });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Baixa");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "+ Faixa" }).click();
  const lanes = await page.locator(".lanes").boundingBox();
  expect(lanes!.height).toBeGreaterThanOrEqual(120);
});
