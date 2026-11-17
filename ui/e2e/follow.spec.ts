import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Seguir");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "+ Faixa" }).click();
  await expect(page.getByTestId("lane-0")).toBeVisible();
}

const lanesScroll = (page: Page) => page.locator(".lanes").evaluate((el) => el.scrollLeft);

async function zoomIn(page: Page, times: number) {
  await page.locator("body").click({ position: { x: 5, y: 5 } });
  for (let i = 0; i < times; i++) await page.keyboard.press("Control+=");
}

test("seguir reprodução: a vista vira a página quando a linha passa da borda", async ({ page }) => {
  await newProject(page);
  await zoomIn(page, 6);
  expect(await lanesScroll(page)).toBe(0);
  await page.keyboard.press("Space");
  // a linha anda; a vista precisa rolar sozinha (vira a página)
  await expect.poll(() => lanesScroll(page), { timeout: 8000 }).toBeGreaterThan(100);
  // a linha continua visível
  const visible = await page.evaluate(() => {
    const lanes = document.querySelector(".lanes")!.getBoundingClientRect();
    const ph = document.querySelector(".lanes .playhead")!.getBoundingClientRect();
    return ph.left >= lanes.left && ph.left <= lanes.right;
  });
  expect(visible).toBe(true);
  await page.keyboard.press("Space");
});

test("seguir desligado (F): a vista fica parada; rolar à mão pausa o seguir", async ({ page }) => {
  await newProject(page);
  await zoomIn(page, 6);
  const seguir = page.getByRole("button", { name: "Seguir reprodução" });
  await expect(seguir).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("f");
  await expect(seguir).toHaveAttribute("aria-pressed", "false");
  await page.keyboard.press("Space");
  await page.waitForTimeout(2500);
  expect(await lanesScroll(page)).toBe(0);
  await page.keyboard.press("Space");
  // religa e rola à mão para longe da linha durante a reprodução: a vista não volta sozinha
  await page.keyboard.press("f");
  await page.keyboard.press("Space");
  await page.locator(".lanes").evaluate((el) => (el.scrollLeft = 6000));
  await page.waitForTimeout(1500);
  expect(await lanesScroll(page)).toBeGreaterThan(5000);
  await page.keyboard.press("Space");
});

test("UX-01: depois de clicar num botão, Espaço toca/pausa em vez de reacionar o botão", async ({ page }) => {
  await newProject(page);
  const metro = page.getByRole("button", { name: "Metrônomo", exact: true });
  await expect(metro).toHaveAttribute("aria-pressed", "false");
  await metro.click();
  await expect(metro).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: /^Pausar/ })).toBeVisible();
  await page.waitForTimeout(300);
  await expect(metro).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: /^Tocar/ })).toBeVisible();
  // pelo teclado (Tab), Espaço continua ativando o botão focado
  await page.keyboard.press("Tab");
  await page.keyboard.press("Shift+Tab");
  await metro.focus();
  await page.keyboard.press("Space");
  await expect(metro).toHaveAttribute("aria-pressed", "false");
});

test("clicar no vazio da faixa ou num clipe posiciona o cursor (UX-08)", async ({ page }) => {
  await newProject(page);
  const pos = page.locator(".transport-display .big").first();
  await expect(pos).toHaveText("001.1.000");
  const lane = (await page.getByTestId("lane-0").boundingBox())!;
  const bar3 = await page.locator(".ruler-mark:not(.ruler-mark--beat)", { hasText: /^3$/ }).first().evaluate((el) => el.getBoundingClientRect().left);
  await page.mouse.click(bar3 + 2, lane.y + lane.height / 2);
  await expect(pos).toHaveText("003.1.000");
  // importa um clipe e clica nele sem arrastar
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await expect(page.locator(".clip").first()).toBeVisible();
  const id = await page.locator(".clip").first().getAttribute("data-clip-id");
  const clip = page.locator(`.clip[data-clip-id="${id}"]`);
  const cb = (await clip.boundingBox())!;
  // o clipe entra no cursor (compasso 3); clicar no meio dele leva o cursor para dentro do clipe
  await page.mouse.click(cb.x + cb.width * 0.6, cb.y + cb.height / 2);
  await expect(pos).not.toHaveText("003.1.000");
});
