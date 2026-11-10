import { test, expect, type Page } from "@playwright/test";

async function newProjectWithTracks(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Zoom");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "+ Faixa" }).click();
  await page.getByRole("button", { name: "+ Faixa" }).click();
  await expect(page.getByTestId("lane-1")).toBeVisible();
}

const barX = (page: Page, bar: number) =>
  page.locator(".ruler-mark:not(.ruler-mark--beat)", { hasText: new RegExp(`^${bar}$`) }).first().evaluate((el) => el.getBoundingClientRect().left);

test("Ctrl+roda aproxima no tempo mantendo o ponto sob o mouse", async ({ page }) => {
  await newProjectWithTracks(page);
  const x3 = await barX(page, 3);
  const x2 = await barX(page, 2);
  const lane = await page.getByTestId("lane-0").boundingBox();
  await page.mouse.move(x3, lane!.y + 20);
  await page.keyboard.down("Control");
  await page.mouse.wheel(0, -300);
  await page.keyboard.up("Control");
  await expect.poll(async () => (await barX(page, 3)) - (await barX(page, 2))).toBeGreaterThan(x3 - x2 + 20); // compasso mais largo
  expect(Math.abs((await barX(page, 3)) - x3)).toBeLessThan(6); // compasso 3 continua sob o mouse
  // afastar volta a estreitar
  await page.keyboard.down("Control");
  await page.mouse.wheel(0, 600);
  await page.keyboard.up("Control");
  await expect.poll(async () => (await barX(page, 3)) - (await barX(page, 2))).toBeLessThan(x3 - x2);
});

test("Alt+roda muda a altura de todas as faixas; a alça muda só uma", async ({ page }) => {
  await newProjectWithTracks(page);
  const h0 = (await page.getByTestId("lane-0").boundingBox())!.height;
  const lane = await page.getByTestId("lane-0").boundingBox();
  await page.mouse.move(lane!.x + 100, lane!.y + 20);
  await page.keyboard.down("Alt");
  await page.mouse.wheel(0, -300);
  await page.keyboard.up("Alt");
  await expect.poll(async () => (await page.getByTestId("lane-1").boundingBox())!.height).toBeGreaterThan(h0 + 20);
  // soltar o Alt depois da roda não leva o foco ao menu principal
  expect(await page.evaluate(() => document.activeElement?.closest("[role=menubar]") !== null)).toBe(false);
  const hAll = (await page.getByTestId("lane-1").boundingBox())!.height;
  // arrastar a borda de baixo do cabeçalho da faixa 1
  const handle = page.getByRole("separator", { name: "Altura de Faixa 1" });
  const hb = (await handle.boundingBox())!;
  await page.mouse.move(hb.x + 20, hb.y + hb.height / 2);
  await page.mouse.down();
  await page.mouse.move(hb.x + 20, hb.y + 60, { steps: 4 });
  await page.mouse.up();
  await expect.poll(async () => (await page.getByTestId("lane-0").boundingBox())!.height).toBeGreaterThan(hAll + 40);
  expect((await page.getByTestId("lane-1").boundingBox())!.height).toBeCloseTo(hAll, 0);
  // cabeçalho acompanha a faixa
  const header = (await page.getByTestId("track-header-0").boundingBox())!.height;
  expect(Math.abs(header - (await page.getByTestId("lane-0").boundingBox())!.height)).toBeLessThan(2);
});

test("arrastar clipe para outra faixa respeita alturas diferentes", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Arraste");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await expect(page.getByTestId("lane-1").locator(".clip")).toHaveCount(1);
  // faixa 1 bem alta: o alvo do arraste depende das alturas reais, não de 72 px fixos
  const handle = page.getByRole("separator").first();
  const hb = (await handle.boundingBox())!;
  await page.mouse.move(hb.x + 20, hb.y + hb.height / 2);
  await page.mouse.down();
  await page.mouse.move(hb.x + 20, hb.y + 120, { steps: 4 });
  await page.mouse.up();
  const clip = page.getByTestId("lane-0").locator(".clip").first();
  let cb = (await clip.boundingBox())!;
  // arrastar 130 px para baixo DENTRO da faixa alta: continua na mesma faixa
  // (com a conta antiga de 72 px fixos, 130 px já pulavam para a faixa de baixo)
  await page.mouse.move(cb.x + cb.width / 2, cb.y + 20);
  await page.mouse.down();
  await page.mouse.move(cb.x + cb.width / 2, cb.y + 150, { steps: 8 });
  await page.mouse.up();
  await expect(page.getByTestId("lane-0").locator(".clip")).toHaveCount(1);
  await expect(page.getByTestId("lane-1").locator(".clip")).toHaveCount(1);
  cb = (await clip.boundingBox())!;
  const lane1 = (await page.getByTestId("lane-1").boundingBox())!;
  await page.mouse.move(cb.x + cb.width / 2, cb.y + 20);
  await page.mouse.down();
  await page.mouse.move(cb.x + cb.width / 2, lane1.y + lane1.height / 2, { steps: 8 });
  await page.mouse.up();
  await expect(page.getByTestId("lane-1").locator(".clip")).toHaveCount(2);
  await expect(page.getByTestId("lane-0").locator(".clip")).toHaveCount(0);
});
