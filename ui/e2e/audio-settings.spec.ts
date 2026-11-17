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
  // o buffer fica em "Desempenho", recolhido por padrão
  await expect(page.getByLabel("Tamanho do buffer")).toBeHidden();
  await page.getByText("Desempenho", { exact: true }).click();
  await page.getByLabel("Tamanho do buffer").selectOption("128");
  await expect(page.getByText("Buffer — ~5,3 ms de ida e volta a 48 kHz")).toBeVisible();
  await page.getByRole("button", { name: "Aplicar" }).click();
  await expect(page.getByRole("dialog", { name: "Configurações de áudio" })).toHaveCount(0);
  await open();
  await expect(page.getByLabel("Dispositivo de entrada")).toHaveValue("sim-in");
  await page.getByText("Desempenho", { exact: true }).click();
  await expect(page.getByLabel("Tamanho do buffer")).toHaveValue("128");
  await expect(page.getByLabel("Dispositivo de saída")).toHaveValue("");
});

test("configurações de áudio: fone (o que você ouve) e microfone (o que você grava), com Testar som e ganho", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Fone e microfone");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
  await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  const dialog = page.getByRole("dialog", { name: "Configurações de áudio" });
  await expect(dialog).toContainText("Sistema de áudio: Simulado · 48 kHz");
  await expect(dialog).not.toContainText("Driver");
  const fone = dialog.getByRole("region", { name: "Fone / alto-falantes — o que você ouve" });
  await fone.getByRole("button", { name: "Testar som" }).click();
  await expect(fone.getByRole("status", { name: "Tom de teste" })).toContainText("Tocando…");
  await expect(fone).toContainText("o Ebiki não reduz o volume");
  const mic = dialog.getByRole("region", { name: "Microfone / interface — o que você grava" });
  await mic.getByLabel("Dispositivo de entrada").selectOption("sim-in");
  await expect(mic).toContainText("Clique em Aplicar");
  await dialog.getByRole("button", { name: "Aplicar" }).click();
  await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
  await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  // medidores com a faixa-alvo; o ganho sobe o nível medido
  const meter = mic.getByRole("meter", { name: "Nível da entrada 2" });
  await expect.poll(async () => Number(await meter.getAttribute("aria-valuenow"))).toBeLessThan(-30);
  await mic.getByLabel("Ganho de entrada").fill("24");
  await expect(mic).toContainText("+24 dB");
  await expect.poll(async () => Number(await meter.getAttribute("aria-valuenow"))).toBeGreaterThan(-20);
  await expect(meter.locator(".input-meter-target")).toHaveCount(1);
});

test("barra de status: mostra fone e microfone e abre as configurações (UX-18)", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Status");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  const status = page.getByRole("contentinfo", { name: "Status" });
  const audio = status.locator(".statusbar-audio");
  await expect(audio).toContainText(/Fone: .* · Microfone: |Sem dispositivo de áudio/);
  await audio.click();
  await expect(page.getByRole("dialog", { name: "Configurações de áudio" })).toBeVisible();
});
