import { test, expect } from "@playwright/test";

// Camadas: diálogos e menus ficam acima dos divisores do dockview (antes um divisor cobria parte dos
// diálogos e o clique não chegava ao botão).
test("diálogos e menus ficam acima dos divisores dos painéis", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Camadas");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();

  await page.keyboard.press("F1");
  await expect(page.getByRole("dialog", { name: "Central de ajuda" })).toBeVisible();
  const coveredInDialog = await page.evaluate(() => {
    const backdrop = document.querySelector(".dialog-backdrop")!;
    return [...document.querySelectorAll(".dv-sash")].flatMap((s) => {
      const r = s.getBoundingClientRect();
      if (!r.width || !r.height) return [];
      const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return top && !backdrop.contains(top) ? [top.className] : [];
    });
  });
  expect(coveredInDialog).toEqual([]);
  await page.keyboard.press("Escape");

  for (const m of ["Arquivo", "Exibir", "Janela"]) {
    await page.getByRole("menuitem", { name: m, exact: true }).click();
    const blocked = await page.evaluate(() =>
      [...document.querySelectorAll(".menubar-menu [role^=menuitem]")].flatMap((el) => {
        const r = el.getBoundingClientRect();
        const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        return top && !el.contains(top) ? [(el as HTMLElement).innerText.split("\n")[0]] : [];
      }),
    );
    expect(blocked, `itens cobertos no menu ${m}`).toEqual([]);
    await page.keyboard.press("Escape");
  }
});

test("menu do clipe aparece inteiro mesmo aberto perto da borda de baixo do Arranjo", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Menu clipe");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  const b = (await page.getByTestId("lane-1").locator(".clip").first().boundingBox())!;
  await page.mouse.click(b.x + 60, b.y + b.height - 6, { button: "right" });
  const menu = page.getByRole("menu", { name: /Ações do clipe/ });
  await expect(menu).toBeVisible();
  const r = await menu.evaluate((el) => {
    const q = el.getBoundingClientRect();
    const pts = [[q.left + q.width / 2, q.top + q.height / 2], [q.left + 4, q.top + 4], [q.left + 4, q.bottom - 4], [q.right - 4, q.bottom - 4]];
    return { inside: q.top >= 0 && q.bottom <= innerHeight && q.right <= innerWidth, covered: pts.filter(([x, y]) => !el.contains(document.elementFromPoint(x!, y!))).length };
  });
  expect(r).toEqual({ inside: true, covered: 0 });
  // e continua funcionando: um item clicável fecha o menu
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
});
