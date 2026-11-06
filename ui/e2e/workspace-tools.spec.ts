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

test("menu Painéis lista todos os painéis e reabre um painel fechado pelo teclado", async ({ page }) => {
  await newProject(page, "Paineis");
  await tab(page, "Mixer").click();
  await tab(page, "Mixer").locator(".dv-default-tab-action").click();
  await expect(tab(page, "Mixer")).toHaveCount(0);

  const button = page.getByRole("button", { name: /Painéis/ });
  await button.focus();
  await page.keyboard.press("Enter");
  const menu = page.getByRole("menu", { name: "Painéis" });
  await expect(menu.getByRole("menuitemcheckbox")).toHaveText(PANEL_CATALOG.map((p) => new RegExp(`${p.title}$`)));
  await expect(menu.getByRole("menuitemcheckbox", { name: "Mixer" })).toHaveAttribute("aria-checked", "false");
  await expect(menu.getByRole("menuitemcheckbox", { name: "Arranjo" })).toBeFocused();

  // navega até "Mixer" com as setas e ativa
  await page.keyboard.press("ArrowDown");
  await expect(menu.getByRole("menuitemcheckbox", { name: "Mixer" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(menu).toHaveCount(0);
  await expect(tab(page, "Mixer")).toHaveCount(1);
  await expect(page.locator(".mixer")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Mixer" })).toHaveAttribute("aria-selected", "true");

  // painel já aberto: o menu só o traz para a frente; Esc fecha e devolve o foco
  await button.click();
  await expect(menu.getByRole("menuitemcheckbox", { name: "Mixer" })).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
  await expect(button).toBeFocused();
  await button.click();
  await menu.getByRole("menuitemcheckbox", { name: "Letra" }).click();
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
