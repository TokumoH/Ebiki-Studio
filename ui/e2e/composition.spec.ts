import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page, name: string) {
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

async function openTab(page: Page, title: string) {
  await page.locator(".dv-tab", { hasText: title }).click();
}

test("harmonia: tom + gênero → ouvir → inserir → desfazer", async ({ page }) => {
  await newProject(page, "Harmonia");
  await openTab(page, "Harmonia");
  const panel = page.getByLabel("Harmonia", { exact: true });
  await panel.getByRole("combobox", { name: "Tônica" }).selectOption({ label: "D" });
  await expect(panel.getByRole("list", { name: /Campo harmônico/ })).toContainText("F#m");
  await panel.getByRole("combobox", { name: "Gênero" }).selectOption({ label: "Samba" });
  const card = panel.getByRole("listitem", { name: /Progressão Samba/ });
  await expect(card.getByLabel("Acordes")).toHaveText(/D\s*B7\s*Em7\s*A7/);
  await page.screenshot({ path: "test-results/harmony-progressions.png" });

  await card.getByRole("button", { name: /^Ouvir/ }).click();
  await expect(card.getByRole("button", { name: /^Parar/ })).toBeVisible();
  await page.getByRole("button", { name: "Parar audição" }).click();
  await expect(card.getByRole("button", { name: /^Ouvir/ })).toBeVisible();

  await card.getByRole("button", { name: /no arranjo/ }).click();
  await expect(page.getByText("Progressão inserida no arranjo.")).toBeVisible();
  await expect(page.getByTestId("track-header-0")).toContainText("Progressão Samba");
  await page.getByRole("button", { name: "Desfazer" }).first().click();
  await expect(page.getByTestId("track-header-0")).toHaveCount(0);
  await noHorizontalOverflow(page);
});

test("harmonia: próximo acorde monta sequência e insere", async ({ page }) => {
  await newProject(page, "Sequencia");
  await openTab(page, "Harmonia");
  const panel = page.getByLabel("Harmonia", { exact: true });
  await panel.getByRole("radio", { name: "Tétrades" }).click();
  await panel.getByRole("tab", { name: "Próximo acorde" }).click();
  const suggestions = panel.getByRole("list", { name: "Próximos acordes sugeridos" });
  await suggestions.getByRole("button", { name: /^Adicionar Dm / }).click();
  await suggestions.getByRole("button", { name: /^Adicionar G / }).click();
  await expect(panel.getByText("Sugestões após G")).toBeVisible();
  await expect(suggestions.getByRole("button").first()).toHaveAccessibleName(/^Adicionar C — Cadência autêntica/);
  await suggestions.getByRole("button").first().click();
  await expect(panel.getByRole("list", { name: "Sequência de acordes" })).toHaveText(/Dm\s*G\s*C/);
  await panel.getByRole("radio", { name: "Graus" }).click();
  await expect(panel.getByRole("list", { name: "Sequência de acordes" })).toHaveText(/ii\s*V\s*I/);
  await panel.getByRole("button", { name: "Ouvir sequência" }).click();
  await panel.getByRole("button", { name: "Inserir sequência no arranjo" }).click();
  await expect(page.getByTestId("track-header-0")).toContainText("Progressão");
  await page.screenshot({ path: "test-results/harmony-panel.png" });
  await noHorizontalOverflow(page);
});

test("harmonia: progressão do usuário com desfazer da biblioteca", async ({ page }) => {
  await newProject(page, "Minhas");
  await openTab(page, "Harmonia");
  const panel = page.getByLabel("Harmonia", { exact: true });
  await panel.getByRole("tab", { name: "Minhas progressões" }).click();
  await panel.getByRole("button", { name: "+ Nova progressão" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Nome da progressão").fill("Minha samba");
  await dialog.getByLabel("Graus").fill("I VI7 ii7 V7");
  await dialog.getByRole("checkbox", { name: "Samba" }).check();
  await page.screenshot({ path: "test-results/harmony-user-dialog.png" });
  await dialog.getByRole("button", { name: "Salvar" }).click();
  await expect(panel.getByRole("listitem", { name: "Progressão Minha samba" })).toBeVisible();
  await panel.getByRole("button", { name: "Remover Minha samba" }).click();
  await expect(panel.getByRole("listitem", { name: "Progressão Minha samba" })).toHaveCount(0);
  await panel.getByRole("button", { name: "Desfazer na biblioteca" }).click();
  await expect(panel.getByRole("listitem", { name: "Progressão Minha samba" })).toBeVisible();
});

test("bases: gerar, salvar e inserir com divergência → Adaptar", async ({ page }) => {
  await newProject(page, "Bases");
  await openTab(page, "Bases");
  const panel = page.getByLabel("Bases", { exact: true });
  await expect(panel.getByRole("row", { name: /Base Loop de bateria/ })).toBeVisible();

  await panel.getByRole("button", { name: "Gerar base…" }).click();
  const gen = page.getByRole("dialog", { name: "Gerar base" });
  await gen.getByLabel("BPM da base").fill("90");
  await gen.getByRole("combobox", { name: "Estilo" }).selectOption({ label: "Valsa (3/4)" });
  await gen.getByRole("button", { name: "Gerar", exact: true }).click();
  await expect(gen.getByRole("status")).toContainText("3/4");
  await page.screenshot({ path: "test-results/bases-generate-dialog.png" });
  await gen.getByRole("button", { name: "Ouvir base gerada" }).click();
  await gen.getByLabel("Nome da base").fill("Valsa minha");
  await gen.getByRole("button", { name: "Salvar na biblioteca" }).click();
  await expect(gen).toHaveCount(0);
  const row = panel.getByRole("row", { name: /Base Valsa minha/ });
  await expect(row).toContainText("3/4");

  await row.getByRole("button", { name: /^Inserir/ }).click();
  const ins = page.getByRole("dialog", { name: /Inserir "Valsa minha"/ });
  await expect(ins.getByLabel("Divergência")).toContainText("A base diverge do projeto");
  await expect(ins.getByLabel("Divergência")).toContainText("base 90 BPM → projeto 120 BPM");
  await expect(ins.getByRole("button", { name: "Manter original" })).toBeVisible();
  await page.screenshot({ path: "test-results/bases-insert-dialog.png" });
  await ins.getByRole("button", { name: "Adaptar ao projeto" }).click();
  await expect(ins).toHaveCount(0);
  await expect(page.getByText('Base "Valsa minha" inserida adaptada ao projeto.')).toBeVisible();
  await expect(page.getByTestId("track-header-0")).toContainText("Base Valsa minha");
  await page.screenshot({ path: "test-results/bases-panel.png" });
  await noHorizontalOverflow(page);
});

test("bases: filtros, SoundFont e sem estouro em 1280×720", async ({ page }) => {
  await newProject(page, "Filtros");
  await openTab(page, "Bases");
  const panel = page.getByLabel("Bases", { exact: true });
  await panel.getByLabel("BPM mínimo").fill("120");
  await expect(panel.getByRole("row", { name: /Base Bossa/ })).toBeVisible();
  await expect(panel.getByRole("row", { name: /Base Loop/ })).toHaveCount(0);
  await panel.getByRole("button", { name: "Limpar" }).click();
  await panel.getByRole("button", { name: "SoundFont…" }).click();
  const sf = page.getByRole("dialog", { name: "Sintetizador (SoundFont)" });
  await sf.getByRole("button", { name: "Escolher arquivo .sf2…" }).click();
  await expect(sf.getByLabel("Estado do sintetizador")).toContainText("SoundFont: Banco simulado");
  await sf.getByRole("button", { name: "Usar sintetizador simples" }).click();
  await expect(sf.getByLabel("Estado do sintetizador")).toContainText("Sintetizador simples");
  await sf.getByRole("button", { name: "Fechar" }).click();
  for (const tab of ["Harmonia", "Bases"]) {
    await openTab(page, tab);
    await noHorizontalOverflow(page);
  }
});
