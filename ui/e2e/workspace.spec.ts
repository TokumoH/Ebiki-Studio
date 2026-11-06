import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page, name = "Ensaio") {
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill(name);
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

async function noHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
}

test("tela inicial valida o nome e não estoura em 1280×720", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("   ");
  await expect(page.getByText("Informe um nome.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Criar projeto" })).toBeDisabled();
  await noHorizontalOverflow(page);
});

test("fluxo: criar → importar → mixar → desfazer → salvar", async ({ page }) => {
  await newProject(page);
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await expect(page.getByTestId("track-header-0")).toContainText("voz");
  await expect(page.getByTestId("track-header-1")).toContainText("violao");
  await expect(page.getByRole("button", { name: "Clipe voz.wav" })).toBeVisible();

  const fader = page.getByRole("slider", { name: "Volume voz" });
  await fader.focus();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await expect(fader).toHaveAttribute("aria-valuetext", "-2.0 dB");
  await expect(page.getByText("●", { exact: false }).first()).toBeVisible();

  await page.getByRole("button", { name: "Desfazer" }).click();
  await expect(fader).toHaveAttribute("aria-valuetext", "-1.0 dB");

  await page.getByRole("button", { name: "Solo voz" }).first().click();
  await expect(page.getByRole("button", { name: "Solo voz" }).first()).toHaveAttribute("aria-pressed", "true");

  await page.keyboard.press("Control+s");
  await expect(page.getByText("Projeto salvo.")).toBeVisible();
  await noHorizontalOverflow(page);
});

test("efeitos: adicionar compressor, editar e remover", async ({ page }) => {
  await newProject(page, "Efeitos");
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await page.getByRole("button", { name: "Adicionar efeito em voz" }).click();
  await page.getByRole("button", { name: "Compressor" }).click();
  await page.getByRole("button", { name: "Editar Compressor" }).click();
  const ratio = page.getByRole("slider", { name: "Razão" });
  await ratio.focus();
  await page.keyboard.press("ArrowRight");
  await page.getByRole("dialog").getByRole("button", { name: "Remover", exact: true }).click();
  await expect(page.getByRole("button", { name: "Editar Compressor" })).toHaveCount(0);
});

test("transporte por teclado e metrônomo", async ({ page }) => {
  await newProject(page, "Transporte");
  await page.locator("body").click({ position: { x: 5, y: 700 } });
  await page.keyboard.press("Space");
  await expect(page.getByRole("button", { name: "Pausar" })).toBeVisible();
  await expect.poll(async () => page.getByTestId("bar-beat").textContent()).not.toBe("001.1.000");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: "Tocar" })).toBeVisible();
  await page.keyboard.press("c");
  await expect(page.getByRole("button", { name: "Metrônomo" })).toHaveAttribute("aria-pressed", "true");
});

test("gravar sem faixa armada mostra erro claro", async ({ page }) => {
  await newProject(page, "Gravar");
  await page.getByRole("button", { name: "Gravar", exact: true }).click();
  await expect(page.getByText("Arme ao menos uma faixa")).toBeVisible();
});

test("painéis inferiores renderizam sem estouro", async ({ page }) => {
  await newProject(page, "Paineis");
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  for (const tab of ["Gravação", "Mapa de tempo", "Análise da mixagem", "Mixer"]) {
    await page.locator(".dv-tab", { hasText: tab }).click();
    await noHorizontalOverflow(page);
  }
  await page.locator(".dv-tab", { hasText: "Análise da mixagem" }).click();
  await page.getByRole("button", { name: "Analisar mixagem" }).click();
  await expect(page.getByText("O master está clipando")).toBeVisible();
  await page.screenshot({ path: "test-results/workspace.png" });
});
