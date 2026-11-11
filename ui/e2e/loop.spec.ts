import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page, withClips = false) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Loop");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
  if (withClips) {
    await page.getByRole("button", { name: "Importar", exact: true }).click();
    await expect(page.locator(".clip").first()).toBeVisible();
  }
}

const loopBtn = (page: Page) => page.getByRole("button", { name: "Loop", exact: true });
const barX = (page: Page, bar: number) => page.locator(".ruler-mark:not(.ruler-mark--beat)", { hasText: new RegExp(`^${bar}$`) }).first().evaluate((el) => el.getBoundingClientRect().left);

test("opções de loop: compassos X até Y, projeto inteiro, desligar e religar o mesmo trecho", async ({ page }) => {
  await newProject(page, true);
  await page.getByRole("button", { name: "Opções de loop" }).click();
  const pop = page.getByRole("dialog", { name: "Opções de loop" });
  await pop.getByLabel("Primeiro compasso do loop").fill("2");
  await pop.getByLabel("Último compasso do loop").fill("5");
  await pop.getByRole("button", { name: /Aplicar \(2–5\)/ }).click();
  await expect(pop).toHaveCount(0);
  await expect(loopBtn(page)).toContainText("Loop · 2–5");
  await expect(loopBtn(page)).toHaveAttribute("aria-pressed", "true");
  // desligar e religar volta ao mesmo trecho (não recomeça em 4 compassos)
  await loopBtn(page).click();
  await expect(loopBtn(page)).toHaveText(/^⟲ Loop$/);
  await loopBtn(page).click();
  await expect(loopBtn(page)).toContainText("Loop · 2–5");
  // projeto inteiro: do início ao fim do compasso do último clipe
  await page.getByRole("button", { name: "Opções de loop" }).click();
  await pop.getByRole("radio", { name: /Projeto inteiro/ }).check();
  await expect(pop).toContainText(/compassos 1–\d+/);
  await pop.getByRole("button", { name: /Aplicar/ }).click();
  await expect(loopBtn(page)).toContainText(/Loop · 1–\d+/);
});

test("sem clipes, 'Projeto inteiro' fica indisponível; Esc fecha; menu Transporte abre as opções", async ({ page }) => {
  await newProject(page);
  await page.getByRole("menuitem", { name: "Transporte", exact: true }).click();
  await page.getByRole("menuitem", { name: "Opções de loop…" }).click();
  const pop = page.getByRole("dialog", { name: "Opções de loop" });
  await expect(pop.getByRole("radio", { name: /Projeto inteiro/ })).toBeDisabled();
  await expect(pop).toContainText("sem clipes ainda");
  await expect(pop).toContainText("Na gravação o loop é ignorado");
  await page.keyboard.press("Escape");
  await expect(pop).toHaveCount(0);
});

test("régua: arrastar na faixa de cima desenha o loop; bordas ajustam; clique só posiciona", async ({ page }) => {
  await newProject(page);
  const ruler = (await page.locator(".ruler").boundingBox())!;
  const x2 = await barX(page, 2);
  const x4 = await barX(page, 4);
  await page.mouse.move(x2 + 1, ruler.y + 4);
  await page.mouse.down();
  await page.mouse.move(x4 + 1, ruler.y + 4, { steps: 6 });
  await page.mouse.up();
  await expect(loopBtn(page)).toContainText("Loop · 2–3");
  // borda do fim: arrastar até o compasso 6 → 2–5
  const end = (await page.getByLabel("Fim do loop").boundingBox())!;
  const x6 = await barX(page, 6);
  await page.mouse.move(end.x + end.width / 2, end.y + end.height / 2);
  await page.mouse.down();
  await page.mouse.move(x6, end.y + end.height / 2, { steps: 6 });
  await page.mouse.up();
  await expect(loopBtn(page)).toContainText("Loop · 2–5");
  // clique sem arrastar na faixa de cima não mexe no loop, só move o cursor
  const x7 = await barX(page, 7); // visível na janela de 1280 px
  await page.mouse.click(x7 + 1, ruler.y + 4);
  await expect(loopBtn(page)).toContainText("Loop · 2–5");
  await expect(page.getByTestId("bar-beat")).toContainText("007.1");
});

test("opções de loop aparecem inteiras (não cortadas pela barra) em janela pequena", async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 640 });
  await newProject(page);
  await page.getByRole("button", { name: "Opções de loop" }).click();
  const hidden = await page.getByRole("dialog", { name: "Opções de loop" }).evaluate((el) => {
    const r = el.getBoundingClientRect();
    const pts = [[r.left + r.width / 2, r.top + r.height / 2], [r.left + 4, r.top + 4], [r.right - 4, r.top + 4], [r.left + 4, r.bottom - 4], [r.right - 4, r.bottom - 4]];
    const out = r.left < 0 || r.top < 0 || r.right > innerWidth || r.bottom > innerHeight;
    return { out, covered: pts.filter(([x, y]) => !el.contains(document.elementFromPoint(x!, y!))).length };
  });
  expect(hidden).toEqual({ out: false, covered: 0 });
});
