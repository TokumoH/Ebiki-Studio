import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page, name: string) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill(name);
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

const menu = (page: Page, name: string) => page.getByRole("menubar", { name: "Menu principal" }).getByRole("menuitem", { name, exact: true });

test("tela de carregamento com a logo aparece e sai sozinha", async ({ page }) => {
  await page.goto("/");
  const splash = page.locator("#splash");
  await expect(splash.locator("img")).toHaveAttribute("src", "/splash-mark.png");
  await expect(splash).toHaveCount(0, { timeout: 5000 });
  await expect(page.getByLabel("Nome do projeto")).toBeVisible();
});

test("menu fixo: Exibir troca o tema e Alt leva o teclado ao menu", async ({ page }) => {
  await newProject(page, "Menu");
  await menu(page, "Exibir").click();
  await page.getByRole("menuitemradio", { name: /Tema claro/ }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await menu(page, "Exibir").click();
  await expect(page.getByRole("menuitemradio", { name: /Tema claro/ })).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Escape"); // fecha o menu (foco fica em "Exibir")
  await expect(menu(page, "Exibir")).toBeFocused();
  await page.keyboard.press("Escape"); // sai da barra

  await page.keyboard.press("Alt");
  await expect(menu(page, "Arquivo")).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("menu", { name: "Arquivo" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu", { name: "Arquivo" })).toHaveCount(0);
});

test("central de ajuda: F1, busca sem acento e 'Mostrar no app' abre o tutorial no passo certo", async ({ page }) => {
  await newProject(page, "Ajuda");
  await page.keyboard.press("F1");
  const dialog = page.getByRole("dialog", { name: "Central de ajuda" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Buscar na ajuda").fill("edicao fade");
  await expect(dialog.getByRole("button", { name: "Edição de clipes" })).toHaveAttribute("aria-current", "page");
  await expect(dialog.getByRole("heading", { name: "Edição de clipes" })).toBeVisible();
  await dialog.getByRole("button", { name: "Mostrar no app" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: /Tutorial: passo 2 de 7/ })).toBeVisible();
  await expect(page.locator(".tour-spot")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("tour")).toHaveCount(0);
});

test("tutorial de início: avança, volta, pula e lembra a escolha", async ({ page }) => {
  await newProject(page, "Tour");
  await menu(page, "Ajuda").click();
  await page.getByRole("menuitem", { name: "Tutorial de início" }).click();
  const card = (n: number) => page.getByRole("dialog", { name: `Tutorial: passo ${n} de 7` });
  await expect(card(1)).toBeVisible();
  await page.getByRole("button", { name: "Próximo", exact: true }).click();
  await expect(card(2)).toBeVisible();
  // o destaque envolve a timeline
  const spot = await page.locator(".tour-spot").boundingBox();
  const arr = await page.locator('[data-tour="arranjo"]').boundingBox();
  expect(spot && arr && spot.x <= arr.x && spot.y <= arr.y && spot.x + spot.width >= arr.x + arr.width).toBeTruthy();
  await page.keyboard.press("ArrowRight");
  await expect(card(3)).toBeVisible();
  await page.getByRole("button", { name: "Voltar", exact: true }).click();
  await expect(card(2)).toBeVisible();
  await page.getByRole("button", { name: "Pular tutorial" }).click();
  await expect(page.getByTestId("tour")).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("ebiki.tutorial.v1"))).toBe("done");
});

test("Sobre mostra a versão e a licença", async ({ page }) => {
  await newProject(page, "Sobre");
  await menu(page, "Ajuda").click();
  await page.getByRole("menuitem", { name: "Sobre o Ebiki Studio" }).click();
  const dialog = page.getByRole("dialog", { name: "Sobre o Ebiki Studio" });
  await expect(dialog).toContainText("Ebiki Studio 0.2.0");
  await expect(dialog).toContainText("GPL-3.0");
});
