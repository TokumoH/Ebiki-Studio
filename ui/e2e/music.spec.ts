import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page, name: string) {
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill(name);
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

async function openTab(page: Page, title: string) {
  await page.locator(".dv-tab", { hasText: title }).click();
}

async function noHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
}

test("partitura: criar e entrar notas pelo teclado, render alphaTab, validar", async ({ page }) => {
  await newProject(page, "Partitura");
  await openTab(page, "Partitura");
  await page.getByRole("button", { name: "+ Nova partitura" }).click();
  await page.getByLabel("Título da partitura").fill("Estudo");
  await page.getByRole("button", { name: "Criar partitura" }).click();
  const editor = page.getByRole("application", { name: "Editor de partitura" });
  await expect(editor).toBeFocused();
  const status = page.getByTestId("score-status");
  await expect(status).toContainText("Comp. 1");
  // semínimas (5) Dó Ré Mi Fá completam o 4/4 e o cursor passa ao compasso 2
  for (const k of ["5", "c", "d", "e", "f"]) await page.keyboard.press(k);
  await expect(status).toContainText("Comp. 2");
  await page.keyboard.press("ArrowLeft");
  await expect(status).toContainText("Fá4 semínima");
  await page.keyboard.press("ArrowUp");
  await expect(status).toContainText("Fá♯4 semínima");
  // colcheia pontuada não cabe sozinha? cabe: 6 (mínima) no compasso 2 e uma pausa
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("6");
  await page.keyboard.press("g");
  await expect(page.getByRole("list", { name: "Eventos do compasso 2" })).toContainText("G4");
  // articulação e dinâmica no evento atual
  await page.keyboard.press("ArrowLeft");
  await page.getByRole("button", { name: "Staccato" }).click();
  await page.getByLabel("Dinâmica").selectOption("mf");
  await expect(status).toContainText("(Staccato) mf");
  // alphaTab desenhou a partitura (SVG)
  await expect(page.locator(".score-at svg").first()).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { name: "Validar" }).click();
  // Fá♯ em Dó maior: fora da escala da armadura; compassos completos
  await expect(page.getByLabel("Relatório de validação")).toContainText("Nota fora da escala: Fá♯4 (Melodia, compasso 1)");
  await expect(page.getByLabel("Relatório de validação")).not.toContainText("faltam");
  // desfazer pelo atalho global volta a edição
  await page.getByRole("button", { name: "Desfazer" }).click();
  await expect(status).not.toContainText("mf");
  await noHorizontalOverflow(page);
  await page.waitForTimeout(500);
  await page.screenshot({ path: "test-results/panel-partitura.png" });
});

test("letra: colar texto, sincronizar por toque e exportar LRC", async ({ page }) => {
  await newProject(page, "Letra");
  await openTab(page, "Letra");
  await page.getByLabel("Texto da letra").fill("Can-to a-qui\nou-tra li-nha");
  await page.getByRole("button", { name: "Aplicar texto" }).click();
  const units = page.getByRole("list", { name: "Unidades da letra" });
  await expect(units.getByRole("button")).toHaveCount(4);
  await expect(units).toContainText("Can-to");
  await page.getByRole("button", { name: "Sincronizar por toque" }).click();
  await expect(page.getByTestId("tap-progress")).toHaveText("0/4 toques");
  for (let i = 0; i < 4; i++) {
    await page.waitForTimeout(250);
    await page.keyboard.press("Space");
  }
  await expect(page.getByTestId("tap-progress")).toHaveText("4/4 toques");
  await page.waitForTimeout(250);
  await page.keyboard.press("Enter");
  await expect(page.getByText("Sincronização aplicada (4 palavras).")).toBeVisible();
  await expect(units.getByRole("button").first()).not.toContainText("—");
  await expect(units.getByRole("button").last()).not.toContainText("—");
  // tempo manual de uma sílaba
  await page.getByRole("radio", { name: "Sílaba" }).click();
  await units.getByRole("button", { name: /^qui/ }).click();
  await page.getByLabel("Início em tempos").fill("8");
  await page.getByLabel("Fim em tempos").fill("9");
  await page.getByRole("button", { name: "Definir", exact: true }).click();
  await expect(units.getByRole("button", { name: /^qui/ })).toContainText("3.1");
  await page.getByLabel("Formato de exportação da letra").selectOption("lrc-word");
  await page.getByRole("button", { name: "Exportar…" }).click();
  await expect(page.getByText("Letra exportada (LRC por palavra).")).toBeVisible();
  await noHorizontalOverflow(page);
  await page.screenshot({ path: "test-results/panel-letra.png" });
});

test("afinação: afinador mostra a nota; editor detecta notas, corrige, aplica e desfaz", async ({ page }) => {
  await newProject(page, "Afinacao");
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await page.getByRole("button", { name: "Clipe voz.wav" }).click();
  await openTab(page, "Afinação");
  await page.getByRole("button", { name: "Ligar afinador" }).click();
  await expect(page.getByTestId("tuner-note")).toContainText("A4");
  await expect(page.getByTestId("tuner-note")).toContainText("Lá");
  await expect(page.locator(".tuner-readout")).toContainText("cents");

  await page.getByRole("button", { name: "Analisar clipe" }).click();
  const roll = page.getByTestId("pitch-roll");
  await expect(roll).toHaveAttribute("aria-label", "6 notas detectadas");
  const n1 = page.getByRole("slider", { name: /^Nota 1:/ });
  await n1.focus();
  await page.keyboard.press("ArrowUp");
  await expect(n1).toHaveAttribute("aria-valuetext", /corrigida para/);
  // arrastar a nota 2 para cima (semitons)
  const n2 = page.getByRole("slider", { name: /^Nota 2:/ });
  const box = (await n2.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 30, { steps: 4 });
  await page.mouse.up();
  await expect(n2).toHaveAttribute("aria-valuetext", /corrigida para/);
  await noHorizontalOverflow(page);
  await page.screenshot({ path: "test-results/panel-afinacao.png" });

  await page.getByRole("button", { name: "Aplicar", exact: true }).click();
  await expect(page.getByText("Correção aplicada")).toBeVisible();
  await expect(page.getByRole("region", { name: "Editor de afinação" })).toContainText("(afinado)");
  await expect(page.getByRole("button", { name: "Analisar clipe" })).toBeVisible();
  await page.getByRole("button", { name: "Desfazer" }).click();
  await expect(roll).toHaveAttribute("aria-label", "6 notas detectadas");
  await expect(page.getByRole("region", { name: "Editor de afinação" })).not.toContainText("(afinado)");
  await page.getByRole("button", { name: "Ligar afinador" }).click();
});

test("mixer: correção de afinação como efeito com escala e classes", async ({ page }) => {
  await newProject(page, "PitchFx");
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await openTab(page, "Mixer");
  await page.getByRole("button", { name: "Adicionar efeito em voz" }).click();
  await page.getByRole("button", { name: "Correção de afinação" }).click();
  await page.getByRole("button", { name: "Editar Correção de afinação" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Tônica").selectOption("7");
  await dialog.getByRole("button", { name: "Maior" }).click();
  await expect(dialog.getByRole("button", { name: "Permitir Fá♯" })).toHaveAttribute("aria-pressed", "true");
  await expect(dialog.getByRole("button", { name: "Permitir Fá", exact: true })).toHaveAttribute("aria-pressed", "false");
  await dialog.getByRole("button", { name: "Permitir Fá", exact: true }).click();
  await expect(dialog.getByRole("button", { name: "Permitir Fá", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(dialog.getByRole("slider", { name: "Lá4" })).toBeVisible();
  await page.screenshot({ path: "test-results/panel-pitch-corrector.png" });
});

test("painéis de música sem rolagem horizontal em 1280×720", async ({ page }) => {
  await newProject(page, "Overflow");
  for (const tab of ["Partitura", "Letra", "Afinação"]) {
    await openTab(page, tab);
    await noHorizontalOverflow(page);
  }
});
