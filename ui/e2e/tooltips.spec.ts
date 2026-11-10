import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Legendas");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

test("legendas: aba do setor, parâmetro de efeito e botão com título", async ({ page }) => {
  await newProject(page);
  const tip = page.getByRole("tooltip");
  await page.locator(".dv-tab", { hasText: "Gravação" }).hover();
  await expect(tip).toContainText("Entrada de áudio, faixas armadas");
  // efeito no master: o parâmetro explica o que faz e a faixa típica
  await page.mouse.move(5, 5);
  await expect(tip).toHaveCount(0);
  await page.locator(".dv-tab", { hasText: "Mixer" }).click();
  await page.getByRole("button", { name: "Adicionar efeito em Master" }).click();
  await page.getByRole("button", { name: "Compressor", exact: true }).hover();
  await expect(tip).toContainText("Diminui a diferença entre partes fortes e fracas");
  await page.getByRole("button", { name: "Compressor", exact: true }).click(); // adiciona e fecha
  await page.getByRole("button", { name: "Editar Compressor" }).click();
  await page.locator(".param", { hasText: "Ataque" }).hover();
  await expect(tip).toContainText("Tempo para o compressor agir");
  await expect(tip).toContainText("voz: 5–15 ms");
  await page.keyboard.press("Escape");
  // título nativo vira legenda (e o atributo some: sem balão duplicado do navegador)
  const salvar = page.getByRole("button", { name: "Salvar", exact: true });
  await salvar.hover();
  await expect(tip).toContainText("Salvar (Ctrl+S)");
  await expect(salvar).not.toHaveAttribute("title", /.+/);
});

test("legendas podem ser desligadas em Exibir", async ({ page }) => {
  await newProject(page);
  await page.getByRole("menuitem", { name: "Exibir", exact: true }).click();
  await page.getByRole("menuitemcheckbox", { name: "Legendas ao passar o mouse" }).click();
  await page.locator(".dv-tab", { hasText: "Gravação" }).hover();
  await page.waitForTimeout(800);
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  // a escolha fica guardada para as próximas aberturas
  expect(await page.evaluate(() => localStorage.getItem("ebiki.tooltips.v1"))).toBe("off");
});
