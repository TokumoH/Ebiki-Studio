import { test, expect, type Page } from "@playwright/test";

async function newProject(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Dispositivos");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await expect(page.getByRole("toolbar", { name: "Transporte" })).toBeVisible();
}

/** Escolhe o microfone simulado em Configurações de áudio (o afinador precisa de um). */
async function chooseMic(page: import("@playwright/test").Page) {
  await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
  await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  await page.getByLabel("Dispositivo de entrada").selectOption("sim-in");
  await page.getByRole("button", { name: "Aplicar" }).click();
  await page.getByRole("dialog").waitFor({ state: "detached" });
}

test("microfone salvo que sumiu: o áudio segue sem entrada e avisa (não fica mudo)", async ({ page }) => {
  await newProject(page);
  await page.getByRole("menuitem", { name: "Arquivo", exact: true }).click();
  await page.getByRole("menuitem", { name: "Configurações de áudio…" }).click();
  // simula a escolha de um microfone que não existe mais (id antigo salvo)
  await page.evaluate(async () => {
    const url = "/src/ipc/api.ts";
    const { api } = (await import(/* @vite-ignore */ url)) as { api: { audioConfigure: (r: unknown) => Promise<{ warning: string | null; error: string | null; inputChannels: number }> } };
    const s = await api.audioConfigure({ outputId: null, inputId: "mic-desconectado", bufferSize: 256 });
    (globalThis as { __lastStatus?: unknown }).__lastStatus = s;
  });
  const s = await page.evaluate(() => (globalThis as { __lastStatus?: { warning: string | null; error: string | null; inputChannels: number } }).__lastStatus);
  expect(s?.error).toBeNull();
  expect(s?.inputChannels).toBe(0);
  expect(s?.warning).toContain("Microfone indisponível");
});

test("dispositivo desconectado com o app aberto: aviso e barra de status atualizada", async ({ page }) => {
  await newProject(page);
  await page.evaluate(() => {
    const emit = (globalThis as { __ebikiMockEmit?: (ev: string, p: unknown) => void }).__ebikiMockEmit!;
    emit("audio-reopened", {
      message: "O microfone foi desconectado: o áudio segue sem entrada até ele voltar.",
      status: { mode: "device", outputName: "Fone USB", inputName: null, outputId: null, inputId: "mic", sampleRate: 48000, bufferSize: 256, inputChannels: 0, estimatedLatencyMs: 10.7, inputGainDb: 0, error: null, warning: "Microfone indisponível (microfone não encontrado (desconectado?)): o áudio segue sem entrada." },
    });
  });
  await expect(page.locator(".toasts").getByRole("status")).toContainText("microfone foi desconectado");
  const bar = page.getByRole("contentinfo", { name: "Status" }).locator(".statusbar-audio");
  await expect(bar).toContainText("Fone: Fone USB · Microfone: nenhum");
  await expect(bar).toContainText("Microfone indisponível");
});

test("afinador: microfone desconectado com o afinador ligado desliga e explica (AU-08)", async ({ page }) => {
  await newProject(page);
  await chooseMic(page);
  await page.locator(".dv-tab", { hasText: "Afinação" }).click();
  await page.getByRole("button", { name: "Ligar afinador" }).click();
  await expect(page.getByRole("button", { name: "Ligar afinador" })).toHaveAttribute("aria-pressed", "true");
  await page.evaluate(() => {
    const emit = (globalThis as { __ebikiMockEmit?: (ev: string, p: unknown) => void }).__ebikiMockEmit!;
    emit("audio-reopened", {
      message: "O microfone foi desconectado: o áudio segue sem entrada até ele voltar.",
      status: { mode: "device", outputName: "Saída simulada", inputName: null, outputId: null, inputId: null, sampleRate: 48000, bufferSize: 256, inputChannels: 0, estimatedLatencyMs: 10.7, inputGainDb: 0, error: null, warning: null },
    });
  });
  await expect(page.getByRole("button", { name: "Ligar afinador" })).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator(".toasts")).toContainText("afinador desligado");
  await expect(page.getByLabel("Canal de entrada do afinador")).toBeDisabled();
});
