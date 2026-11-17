import { test, expect, type Page } from "@playwright/test";
import { PANEL_CATALOG } from "../src/panels/catalog";

async function newProject(page: Page, name: string) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill(name);
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

const tab = (page: Page, title: string) => page.locator(".dv-tab").filter({ hasText: new RegExp(`^${title}$`) });

test("menu Janela lista os painéis e reabre um painel fechado pelo teclado", async ({ page }) => {
  await newProject(page, "Paineis");
  await tab(page, "Mixer").click();
  await tab(page, "Mixer").locator(".dv-default-tab-action").click();
  await expect(tab(page, "Mixer")).toHaveCount(0);

  // F10 leva o teclado ao menu principal; → até "Janela"; Enter abre
  await page.keyboard.press("F10");
  await expect(page.getByRole("menuitem", { name: "Menu do Ebiki" })).toBeFocused();
  for (let i = 0; i < 8; i++) await page.keyboard.press("ArrowRight");
  const janela = page.getByRole("menuitem", { name: "Janela" });
  await expect(janela).toBeFocused();
  await page.keyboard.press("Enter");
  const menu = page.getByRole("menu", { name: "Janela" });
  await expect(menu.getByRole("menuitemcheckbox")).toHaveText(PANEL_CATALOG.map((p) => new RegExp(`${p.title}`)));
  await expect(menu.getByRole("menuitemcheckbox", { name: "Mixer" })).toHaveAttribute("aria-checked", "false");
  await expect(menu.getByRole("menuitemcheckbox", { name: "Arranjo" })).toBeFocused();

  // navega até "Mixer" com as setas e ativa
  await page.keyboard.press("ArrowDown");
  await expect(menu.getByRole("menuitemcheckbox", { name: "Mixer" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(menu).toHaveCount(0);
  await expect(tab(page, "Mixer")).toHaveCount(1);
  await expect(page.locator(".mixer")).toBeVisible();

  // aberto: aparece marcado; Esc fecha o menu e devolve o foco ao botão
  await janela.click();
  await expect(menu.getByRole("menuitemcheckbox", { name: "Mixer" })).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
  await expect(janela).toBeFocused();

  // Ferramentas traz um painel para a frente
  await page.getByRole("menuitem", { name: "Ferramentas" }).click();
  await page.getByRole("menuitem", { name: "Letra sincronizada" }).click();
  await expect(page.locator(".dv-tab.dv-active-tab").filter({ hasText: /^Letra$/ })).toHaveCount(1);

  // painel fechado continua fechado ao recarregar (o layout é lembrado)
  await tab(page, "Bases").click();
  await tab(page, "Bases").locator(".dv-default-tab-action").click();
  await expect(tab(page, "Bases")).toHaveCount(0);
  await page.reload();
  await page.getByLabel("Nome do projeto").fill("Paineis 2");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(tab(page, "Mixer")).toHaveCount(1);
  await expect(tab(page, "Bases")).toHaveCount(0);
});

test("renomeia o projeto clicando no nome na barra superior", async ({ page }) => {
  await newProject(page, "Antigo");
  await page.getByRole("button", { name: "Renomear projeto Antigo" }).click();
  const input = page.getByRole("textbox", { name: "Novo nome do projeto" });
  await input.fill("Novo nome");
  await input.press("Enter");
  await expect(page.getByRole("button", { name: "Renomear projeto Novo nome" })).toBeVisible();
  await page.getByRole("button", { name: "Renomear projeto Novo nome" }).focus();
  await page.keyboard.press("Enter");
  await page.getByRole("textbox", { name: "Novo nome do projeto" }).fill("Descartado");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: "Renomear projeto Novo nome" })).toBeVisible();
});

test("bus: renomeia e remove com confirmação no Mixer", async ({ page }) => {
  await newProject(page, "Buses");
  await tab(page, "Mixer").click();
  await page.getByRole("button", { name: "+ Bus" }).click();
  await page.getByRole("button", { name: "Renomear Bus 1" }).click();
  await page.getByRole("textbox", { name: "Novo nome de Bus 1" }).fill("Reverb");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: "Renomear Reverb" })).toBeVisible();

  await page.getByRole("button", { name: "Remover Reverb" }).click();
  const dialog = page.getByRole("dialog", { name: "Remover Reverb?" });
  await dialog.getByRole("button", { name: "Cancelar" }).click();
  await expect(page.getByRole("button", { name: "Renomear Reverb" })).toBeVisible();
  await page.getByRole("button", { name: "Remover Reverb" }).click();
  await dialog.getByRole("button", { name: "Remover", exact: true }).click();
  await expect(page.getByRole("button", { name: "Renomear Reverb" })).toHaveCount(0);
  await page.getByRole("button", { name: "Desfazer" }).click();
  await expect(page.getByRole("button", { name: "Renomear Reverb" })).toBeVisible();
});

test("faixas: reordena pelo teclado (mover para cima/baixo)", async ({ page }) => {
  await newProject(page, "Ordem");
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await expect(page.getByTestId("track-header-0")).toContainText("voz");
  await expect(page.getByTestId("track-header-1")).toContainText("violao");
  await expect(page.getByRole("button", { name: "Mover voz para cima" })).toBeDisabled();
  await page.getByRole("button", { name: "Mover voz para baixo" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("track-header-0")).toContainText("violao");
  await expect(page.getByTestId("track-header-1")).toContainText("voz");
  // o mixer segue a mesma ordem
  await tab(page, "Mixer").click();
  await expect(page.locator(".strip-name").first()).toHaveText("violao");
});

test("atalho L cria loop de 4 compassos a partir do compasso do cursor", async ({ page }) => {
  await newProject(page, "Loop");
  await page.locator(".timeline").focus();
  await page.keyboard.press("l");
  const loop = page.locator(".ruler-range--loop");
  await expect(loop).toHaveCount(1);
  // zoom padrão: 40 px por tempo → 4 compassos 4/4 = 16 tempos = 640 px, começando no compasso 1
  const box = await loop.evaluate((el) => ({ left: (el as HTMLElement).style.left, width: (el as HTMLElement).style.width }));
  expect(box).toEqual({ left: "0px", width: "640px" });
  await page.keyboard.press("l");
  await expect(loop).toHaveCount(0);
});

test("atalhos: tecla repetida avisa e substitui; restaurar padrões; Espaço em PT-BR (UX-13)", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Atalhos");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator("body").click({ position: { x: 5, y: 300 } });
  await page.keyboard.press("Control+/");
  const dialog = page.getByRole("dialog", { name: "Atalhos de teclado" });
  await expect(dialog.getByRole("button", { name: "Alterar atalho: Tocar / pausar" })).toHaveText("Espaço");
  await dialog.getByRole("button", { name: "Alterar atalho: Metrônomo" }).click();
  await page.keyboard.press("l");
  await expect(dialog.getByRole("alert")).toContainText("já é de “Loop”");
  await dialog.getByRole("button", { name: "Substituir" }).click();
  await expect(dialog.getByRole("button", { name: "Alterar atalho: Metrônomo" })).toHaveText("L");
  await expect(dialog.getByRole("button", { name: "Alterar atalho: Loop" })).toHaveText("—");
  await dialog.getByRole("button", { name: "Restaurar padrões" }).click();
  await expect(dialog.getByRole("button", { name: "Alterar atalho: Loop" })).toHaveText("L");
  await expect(dialog.getByRole("button", { name: "Alterar atalho: Metrônomo" })).toHaveText("C");
});

test("Delete exclui o clipe selecionado mesmo com o foco fora do Arranjo (UX-14)", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Excluir");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  const before = await page.locator(".clip").count();
  await page.locator(".clip").first().click();
  // foco vai para outro painel (aba Mixer) e Delete continua valendo
  await page.locator(".dv-tab").filter({ hasText: /^Mixer$/ }).first().click();
  await page.keyboard.press("Delete");
  await expect(page.locator(".clip")).toHaveCount(before - 1);
  await expect(page.locator(".toasts").getByRole("alert").locator(".toast")).toHaveCount(0); // sem exclusão dupla
});

test("abas do dock em PT-BR: botão fechar diz 'Fechar Mixer' (UX-16)", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Dock");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("button", { name: "Fechar Mixer" })).toHaveCount(1);
  await expect(page.getByRole("button", { name: /^Close / })).toHaveCount(0);
});

async function freshProject(page: import("@playwright/test").Page, name: string) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill(name);
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
}

test("BPM editável no transporte (UX-20)", async ({ page }) => {
  await freshProject(page, "Bpm");
  await page.getByRole("button", { name: /^Andamento: 120,0 BPM/ }).click();
  const input = page.getByLabel("Andamento (BPM)");
  await input.fill("96,5");
  await input.press("Enter");
  await expect(page.getByRole("button", { name: /^Andamento: 96,5 BPM/ })).toBeVisible();
  // Esc cancela
  await page.getByRole("button", { name: /^Andamento/ }).click();
  await page.getByLabel("Andamento (BPM)").fill("200");
  await page.getByLabel("Andamento (BPM)").press("Escape");
  await expect(page.getByRole("button", { name: /^Andamento: 96,5 BPM/ })).toBeVisible();
});

test("remover faixa: aviso com Desfazer (UX-22)", async ({ page }) => {
  await freshProject(page, "Remover");
  await page.getByRole("button", { name: "+ Faixa" }).click();
  await expect(page.getByTestId("lane-0")).toBeVisible();
  await page.getByRole("button", { name: "Remover Faixa 1" }).click();
  await expect(page.getByTestId("lane-0")).toHaveCount(0);
  await page.locator(".toasts").getByRole("status").getByRole("button", { name: "Desfazer" }).click();
  await expect(page.getByTestId("lane-0")).toBeVisible();
});

test("exportar: trecho do loop e aviso com o caminho (UX-19)", async ({ page }) => {
  await freshProject(page, "Exportar");
  await page.getByRole("button", { name: "Importar", exact: true }).click();
  await page.locator("body").click({ position: { x: 5, y: 300 } });
  await page.keyboard.press("l"); // liga o loop (4 compassos a partir do cursor)
  await page.keyboard.press("Control+e");
  const dialog = page.getByRole("dialog", { name: "Exportar" });
  await dialog.getByRole("radio", { name: /Trecho do loop/ }).click();
  await expect(dialog.getByRole("radio", { name: /Trecho do loop/ })).toHaveAttribute("aria-checked", "true");
  await dialog.getByRole("button", { name: "Exportar", exact: true }).click();
  const toasts = page.locator(".toasts").getByRole("status");
  await expect(toasts).toContainText("Mixagem exportada em");
  await toasts.getByRole("button", { name: "Mostrar na pasta" }).click();
  await expect.poll(() => page.evaluate(() => (globalThis as { __ebikiLastReveal?: string }).__ebikiLastReveal)).toMatch(/Exportar\.wav$/);
  const sent = await page.evaluate(() => (globalThis as { __ebikiLastExport?: { range: [number, number] | null } }).__ebikiLastExport);
  expect(sent?.range).toEqual([0, 960 * 4 * 4]);
});
