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

test("Gravação: o bloco Entrada de áudio recolhe para uma linha de resumo e lembra a escolha", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Recolher");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator(".dv-tab", { hasText: "Gravação" }).click();
  const section = page.getByRole("region", { name: "Entrada de áudio" });
  await section.getByLabel("Dispositivo de entrada da gravação").selectOption("sim-in");
  await section.getByRole("button", { name: "Ouvir entrada" }).click();
  const toggle = section.getByRole("button", { name: /Entrada de áudio/ });
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  // resumo: dispositivo, medidor resumido e "ouvindo"; controles completos escondidos
  await expect(section).toContainText("Entrada simulada");
  await expect(section).toContainText("🎧 ouvindo");
  await expect(section.getByRole("meter", { name: "Nível da entrada (resumo)" })).toBeVisible();
  await expect(section.getByLabel("Dispositivo de entrada da gravação")).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("ebiki.recording.inputCollapsed"))).toBe("1");
  // o painel de faixas continua lá, mais perto do topo
  await expect(page.getByRole("heading", { name: "Faixas" })).toBeVisible();
  // teclado abre de novo
  await toggle.focus();
  await page.keyboard.press("Enter");
  await expect(section.getByLabel("Dispositivo de entrada da gravação")).toBeVisible();
});
