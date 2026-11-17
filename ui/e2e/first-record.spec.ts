import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Primeira");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.getByRole("button", { name: "+ Faixa" }).click();
  await expect(page.getByTestId("lane-0")).toBeVisible();
}

test("gravar sem microfone: um aviso só, com atalho para a Gravação (UX-09)", async ({ page }) => {
  await newProject(page);
  await page.locator("body").click({ position: { x: 5, y: 300 } });
  await page.keyboard.press("r");
  await page.keyboard.press("r");
  const alert = page.locator(".toasts").getByRole("alert");
  await expect(alert.locator(".toast")).toHaveCount(1);
  await expect(alert).toContainText("escolha o microfone");
  await alert.getByRole("button", { name: "Abrir Gravação" }).click();
  await expect(page.locator(".dv-tab.dv-active-tab", { hasText: "Gravação" })).toBeVisible();
  await expect(alert.locator(".toast")).toHaveCount(0);
});

test("gravar com microfone e nada armado: arma a faixa selecionada e grava (UX-09)", async ({ page }) => {
  await newProject(page);
  await page.locator(".dv-tab", { hasText: "Gravação" }).click();
  await page.getByLabel("Dispositivo de entrada da gravação").selectOption("sim-in");
  // UX-12: o botão do metrônomo diz o estado real
  await expect(page.locator(".recording-side").getByRole("button", { name: "Metrônomo ligado" })).toHaveText("Desligado");
  await page.locator("body").click({ position: { x: 5, y: 300 } });
  await page.keyboard.press("r");
  await expect(page.getByRole("status").filter({ hasText: "armada para gravar" })).toBeVisible();
  await expect(page.locator(".badge-rec")).toBeVisible();
  await page.keyboard.press("r");
  await expect(page.locator(".badge-rec")).toHaveCount(0);
});

test("trabalhando: barra de progresso e botões desativados (UX-05)", async ({ page }) => {
  await newProject(page);
  // simula uma operação longa: o estado busy vem do core/session
  await page.evaluate(async () => {
    const url = "/src/core/session.ts"; // mesmo módulo que o app usa (servidor do Vite)
    const mod = (await import(/* @vite-ignore */ url)) as { useSession: { setState: (s: { busy: boolean }) => void } };
    mod.useSession.setState({ busy: true });
  });
  await expect(page.getByRole("progressbar", { name: "Trabalhando…" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Salvar" })).toBeDisabled();
  await page.evaluate(async () => {
    const url = "/src/core/session.ts";
    const mod = (await import(/* @vite-ignore */ url)) as { useSession: { setState: (s: { busy: boolean }) => void } };
    mod.useSession.setState({ busy: false });
  });
  await expect(page.getByRole("progressbar")).toHaveCount(0);
});
