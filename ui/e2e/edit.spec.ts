import { test, expect } from "@playwright/test";

// Fase 3 — edição: fade pelo inspetor, dividir no cursor (S), menu de contexto e desfazer.
test("editar clipe: fade + dividir no cursor + desfazer", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Edição");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "Importar", exact: true }).click();

  const lane = page.getByTestId("lane-0");
  const clips = lane.getByRole("button", { name: "Clipe voz.wav" });
  await expect(clips).toHaveCount(1);
  await clips.first().click({ position: { x: 60, y: 40 } });

  // fade de entrada pelo inspetor (painel "Clipe")
  await page.locator(".dv-tab").filter({ hasText: /^Clipe$/ }).first().click();
  const fadeIn = page.getByRole("textbox", { name: "Fade de entrada (ms)" });
  await fadeIn.fill("250");
  await fadeIn.press("Enter");
  const handle = page.getByRole("slider", { name: "Fade de entrada — voz.wav" });
  await expect(handle).toHaveAttribute("aria-valuetext", "250 ms");
  // ajuste fino pelo teclado na alça (+10 ms)
  await handle.focus();
  await page.keyboard.press("ArrowRight");
  await expect(handle).toHaveAttribute("aria-valuetext", "260 ms");

  // cursor dentro do clipe (régua: 40 px por tempo → 200 px = 5 tempos) e S divide
  const ruler = page.getByLabel("Régua (clique para posicionar)");
  await ruler.click({ position: { x: 200, y: 10 } });
  await expect.poll(async () => page.getByTestId("bar-beat").textContent()).toContain("002.2");
  await page.locator(".timeline").focus();
  await page.keyboard.press("s");
  await expect(clips).toHaveCount(2);
  await expect(page.getByRole("button", { name: "Desfazer" })).toHaveAttribute("title", /Dividir clipe/);

  // menu de contexto: inverter fase da segunda parte pelo teclado
  await clips.nth(1).click({ button: "right", position: { x: 40, y: 40 } });
  const menu = page.getByRole("menu", { name: /Ações do clipe/ });
  await expect(menu).toBeVisible();
  await page.getByRole("menuitemcheckbox", { name: /Inverter fase/ }).click();
  await expect(menu).toHaveCount(0);
  await expect(clips.nth(1)).toHaveAttribute("title", /fase invertida/);

  // desfazer: fase → divisão → fades
  await page.keyboard.press("Control+z");
  await expect(clips.nth(1)).not.toHaveAttribute("title", /fase invertida/);
  await page.keyboard.press("Control+z");
  await expect(clips).toHaveCount(1);
  await page.keyboard.press("Control+z");
  await expect(handle).toHaveAttribute("aria-valuetext", "250 ms");
  await page.keyboard.press("Control+z");
  await expect(handle).toHaveAttribute("aria-valuetext", "0 ms");

  // alternar para espectrograma na faixa
  await page.getByRole("button", { name: "Espectrograma voz" }).click();
  await expect(page.getByRole("button", { name: "Espectrograma voz" })).toHaveAttribute("aria-pressed", "true");
  await expect(clips.first()).toHaveClass(/clip--spectrum/);
});
