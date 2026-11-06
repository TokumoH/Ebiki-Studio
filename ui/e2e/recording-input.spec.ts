import { test, expect } from "@playwright/test";

test("Gravação: escolher a entrada no painel, ver o nível e ouvir a entrada", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Entrada");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator(".dv-tab", { hasText: "Gravação" }).click();
  const section = page.getByRole("region", { name: "Entrada de áudio" });
  await expect(section).toContainText("Escolha um dispositivo");
  await expect(section.getByRole("button", { name: "Ouvir entrada" })).toHaveCount(0);

  await section.getByLabel("Dispositivo de entrada da gravação").selectOption("sim-in");
  await expect(section).toContainText("2 canais");
  // nível da entrada 1 chega pela telemetria
  const meter = section.getByRole("meter", { name: "Nível da entrada 1" });
  await expect.poll(async () => Number(await meter.getAttribute("aria-valuenow"))).toBeGreaterThan(-30);
  // as entradas das faixas passam a existir sem ir às Configurações de áudio
  await expect(page.getByText("Nenhuma entrada de áudio ativa")).toHaveCount(0);

  const ouvir = section.getByRole("button", { name: "Ouvir entrada" });
  await section.getByLabel("Canais para ouvir").selectOption("0:1");
  await ouvir.click();
  await expect(ouvir).toHaveAttribute("aria-pressed", "true");

  // o diálogo de Configurações de áudio vê a mesma entrada (estado compartilhado)
  await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
  await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  await expect(page.getByLabel("Dispositivo de entrada", { exact: true })).toHaveValue("sim-in");
  await page.getByRole("button", { name: "Cancelar" }).click();

  // sem entrada, a monitoração desliga e os medidores somem
  await section.getByLabel("Dispositivo de entrada da gravação").selectOption("");
  await expect(section.getByRole("button", { name: "Ouvir entrada" })).toHaveCount(0);
});
