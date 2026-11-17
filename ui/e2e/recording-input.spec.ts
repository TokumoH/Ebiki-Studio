import { test, expect, type Page } from "@playwright/test";

/** Escolhe o microfone pelo botão do painel (o microfone é escolhido num lugar só: as Configurações de áudio). */
async function chooseMic(page: Page, id: string) {
  await page.getByRole("button", { name: "Configurar microfone e fone…" }).click();
  const dialog = page.getByRole("dialog", { name: "Configurações de áudio" });
  await dialog.getByLabel("Dispositivo de entrada", { exact: true }).selectOption(id);
  await dialog.getByRole("button", { name: "Aplicar" }).click();
  await expect(dialog).toHaveCount(0);
}

test("Gravação: Microfone (entrada) mostra o microfone, o ganho e o nível, e ouve no fone", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Entrada");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator(".dv-tab", { hasText: "Gravação" }).click();
  const section = page.getByRole("region", { name: "Microfone (entrada)" });
  await expect(section).toContainText("Escolha o microfone");
  await expect(section.getByRole("button", { name: "Ouvir o microfone no fone" })).toHaveCount(0);
  // sem seletor de dispositivo no painel
  await expect(section.getByRole("combobox", { name: /Dispositivo/ })).toHaveCount(0);

  await chooseMic(page, "sim-in");
  await expect(section).toContainText("Entrada simulada");
  await expect(section).toContainText("2 canais");
  await expect(section).toContainText("0 dB");
  // nível da entrada 1 chega pela telemetria; a faixa-alvo verde aparece no medidor
  const meter = section.getByRole("meter", { name: "Nível da entrada 1" });
  await expect.poll(async () => Number(await meter.getAttribute("aria-valuenow"))).toBeGreaterThan(-30);
  await expect(meter.locator(".input-meter-target")).toHaveCount(1);
  // as entradas das faixas passam a existir
  await expect(page.getByText("Nenhuma entrada de áudio ativa")).toHaveCount(0);

  const ouvir = section.getByRole("button", { name: "Ouvir o microfone no fone" });
  await expect(section.getByLabel("Volume de monitoração")).toHaveValue("0");
  await section.getByLabel("Canais para ouvir").selectOption("0:1");
  await ouvir.click();
  await expect(ouvir).toHaveAttribute("aria-pressed", "true");

  // o ganho escolhido no diálogo aparece no painel (estado compartilhado)
  await page.getByRole("button", { name: "Configurar microfone e fone…" }).click();
  const dialog = page.getByRole("dialog", { name: "Configurações de áudio" });
  await expect(dialog.getByLabel("Dispositivo de entrada", { exact: true })).toHaveValue("sim-in");
  await expect(dialog.getByLabel("Ouvir o microfone no fone enquanto grava")).toBeChecked();
  await dialog.getByLabel("Ganho de entrada").fill("12");
  await expect(dialog).toContainText("+12 dB");
  await dialog.getByRole("button", { name: "Cancelar" }).click();
  await expect(section).toContainText("+12 dB");

  // sem entrada, a monitoração desliga e os medidores somem
  await chooseMic(page, "");
  await expect(section.getByRole("button", { name: "Ouvir o microfone no fone" })).toHaveCount(0);
});

test("Gravação: o bloco Microfone (entrada) recolhe para uma linha de resumo e lembra a escolha", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Recolher");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator(".dv-tab", { hasText: "Gravação" }).click();
  const section = page.getByRole("region", { name: "Microfone (entrada)" });
  await chooseMic(page, "sim-in");
  await section.getByRole("button", { name: "Ouvir o microfone no fone" }).click();
  const toggle = section.getByRole("button", { name: /Microfone \(entrada\)/ });
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  // resumo: microfone, medidor resumido e "ouvindo"; controles completos escondidos
  await expect(section).toContainText("Entrada simulada");
  await expect(section).toContainText("🎧 ouvindo");
  await expect(section.getByRole("meter", { name: "Nível da entrada (resumo)" })).toBeVisible();
  await expect(section.getByRole("button", { name: "Ouvir o microfone no fone" })).toHaveCount(0);
  expect(await page.evaluate(() => localStorage.getItem("ebiki.recording.inputCollapsed"))).toBe("1");
  // o painel de faixas continua lá, mais perto do topo
  await expect(page.getByRole("heading", { name: "Faixas" })).toBeVisible();
  // teclado abre de novo (chegando ao botão pelo Tab; depois de um clique, Enter é do atalho global — UX-01)
  await toggle.focus();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Shift+Tab");
  await expect(toggle).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(section.getByRole("button", { name: "Ouvir o microfone no fone" })).toBeVisible();
});
