import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page, scenario: Record<string, string>) {
  await page.goto("/");
  await page.evaluate((s) => {
    localStorage.clear();
    for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v);
  }, scenario);
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Atualizações");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

const checkFromMenu = async (page: Page) => {
  await page.getByRole("menubar", { name: "Menu principal" }).getByRole("menuitem", { name: "Ajuda", exact: true }).click();
  await page.getByRole("menuitem", { name: "Verificar atualizações…" }).click();
};

const NEW = JSON.stringify({ version: "9.0.0", notes: "### Novidades\n\n- Automação de volume" });

test("Ajuda › Verificar atualizações: instala, salva e reinicia", async ({ page }) => {
  await newProject(page, { "ebiki.mock.update": NEW });
  await checkFromMenu(page);
  const dialog = page.getByRole("dialog", { name: "Atualizações" });
  await expect(dialog).toContainText("Atualização disponível");
  await expect(dialog).toContainText("• Automação de volume");
  await dialog.getByRole("button", { name: "Instalar e reiniciar" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __ebikiRelaunched?: boolean }).__ebikiRelaunched)).toBe(true);
});

test("sem versão nova e sem internet: mensagens claras, nada é verificado sozinho", async ({ page }) => {
  await newProject(page, { "ebiki.mock.update": NEW });
  // desligado por padrão: abrir o app não consulta nada nem mostra aviso
  await expect(page.getByRole("button", { name: /Versão 9.0.0 disponível/ })).toHaveCount(0);
  await page.evaluate(() => localStorage.removeItem("ebiki.mock.update"));
  await checkFromMenu(page);
  const dialog = page.getByRole("dialog", { name: "Atualizações" });
  await expect(dialog).toContainText("Você está na versão mais recente");
  await page.evaluate(() => localStorage.setItem("ebiki.mock.update", "offline"));
  await dialog.getByRole("button", { name: "Fechar" }).click();
  await checkFromMenu(page);
  await expect(dialog).toContainText("Sem conexão com a internet");
});

test("verificar ao abrir: aviso discreto na barra de status abre o diálogo", async ({ page }) => {
  await newProject(page, { "ebiki.mock.update": NEW, "ebiki.updates.v1": JSON.stringify({ checkOnOpen: true, skipped: null }) });
  const notice = page.getByRole("button", { name: /Versão 9.0.0 disponível/ });
  await expect(notice).toBeVisible();
  await expect(page.getByRole("dialog", { name: "Atualizações" })).toHaveCount(0);
  await notice.click();
  await page.getByRole("dialog", { name: "Atualizações" }).getByRole("button", { name: "Pular esta versão" }).click();
  await expect(notice).toHaveCount(0);
});

test("instalação .deb/.rpm abre a página de download", async ({ page }) => {
  await newProject(page, { "ebiki.mock.update": NEW, "ebiki.mock.updateMode": "manual" });
  await checkFromMenu(page);
  await page.getByRole("button", { name: "Abrir página de download" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __ebikiOpenedUrl?: string }).__ebikiOpenedUrl)).toContain("/releases/latest");
});
