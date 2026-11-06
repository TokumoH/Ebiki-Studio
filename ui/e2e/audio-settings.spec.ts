import { test, expect } from "@playwright/test";

test("configurações de áudio: o foco fica no seletor escolhido enquanto a telemetria chega", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Dispositivos");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
  await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  const input = page.getByLabel("Dispositivo de entrada");
  await input.focus();
  // a telemetria (medidores) re-renderiza o Workspace ~30x/s; o foco não pode ser roubado
  await page.waitForTimeout(400);
  await expect(input).toBeFocused();
  await input.selectOption("sim-in");
  await page.waitForTimeout(200);
  await expect(input).toHaveValue("sim-in");
  await expect(input).toBeFocused();
});

test("configurações de áudio: reabrir mostra a entrada e o buffer escolhidos", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Dispositivos 2");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  const open = async () => {
    await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
    await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  };
  await open();
  await page.getByLabel("Dispositivo de entrada").selectOption("sim-in");
  await page.getByLabel("Tamanho do buffer").selectOption("128");
  await page.getByRole("button", { name: "Aplicar" }).click();
  await expect(page.getByRole("dialog", { name: "Configurações de áudio" })).toHaveCount(0);
  await open();
  await expect(page.getByLabel("Dispositivo de entrada")).toHaveValue("sim-in");
  await expect(page.getByLabel("Tamanho do buffer")).toHaveValue("128");
  await expect(page.getByLabel("Dispositivo de saída")).toHaveValue("");
});
