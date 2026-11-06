import { test, expect, type Page } from "@playwright/test";

async function newScore(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.goto("/");
  await page.getByLabel("Nome do projeto").fill("Notas");
  await page.getByRole("button", { name: "Escolher…" }).click();
  await page.getByRole("button", { name: "Criar projeto" }).click();
  await page.locator(".dv-tab", { hasText: "Partitura" }).click();
  await page.getByRole("button", { name: "+ Nova partitura" }).click();
  await page.getByRole("button", { name: "Criar partitura" }).click();
  await expect(page.getByRole("application", { name: "Editor de partitura" })).toBeFocused();
}

test("Digitar notas: o exemplo C,C,C,x,x,x,D,D,D entra de uma vez e sai com um Ctrl+Z", async ({ page }) => {
  await newScore(page);
  const box = page.getByLabel("Notas em texto");
  await box.fill("C,C,C,x,x,x,D,D,D");
  const prev = page.getByLabel("Prévia");
  await expect(prev).toContainText("Dó4 ♩");
  await expect(prev).toContainText("pausa ♩");
  await expect(page.getByRole("status").filter({ hasText: "9 eventos" })).toContainText("2 compassos e pouco");
  await box.press("Enter");
  await expect(box).toHaveValue("");
  // 4/4: compasso 1 = C C C x; compasso 2 = x x D D; compasso 3 = D …
  const status = page.getByTestId("score-status");
  await expect(status).toContainText("Comp. 3");
  await page.keyboard.press("ArrowLeft");
  await expect(page.getByRole("list", { name: /Eventos do compasso/ })).toContainText("D4");
  // um único Ctrl+Z desfaz o lote inteiro
  await page.getByRole("application", { name: "Editor de partitura" }).focus();
  await page.keyboard.press("Control+z");
  // volta ao estado antes do lote: os 8 compassos vazios da partitura nova (nenhum criado sobra)
  for (let m = 0; m < 3; m++) {
    await expect(page.locator(".event-strip li")).toHaveCount(1);
    await expect(page.locator(".event-strip")).toContainText("pausa");
    await page.keyboard.press("ArrowLeft");
  }
  await expect(status).toContainText("Comp. 1");
  await expect(page.getByLabel("Partituras")).toBeVisible();
});

test("Digitar notas: erro aponta o trecho e nada é inserido", async ({ page }) => {
  await newScore(page);
  const box = page.getByLabel("Notas em texto");
  await box.fill("C, D, H, E");
  await expect(page.getByRole("region", { name: "Digitar notas" }).getByRole("alert")).toContainText('"H" na posição 3');
  await expect(page.getByRole("button", { name: "Inserir" })).toBeDisabled();
  await box.press("Enter");
  await expect(page.locator(".event-strip")).not.toContainText("C4");
  // durações e acordes
  await box.fill("E5:8 F#:8 C+E+G:2");
  // depois de Fá♯5, o Dó mais próximo é o Dó5 (empate com Dó6: fica o mais grave)
  await expect(page.getByLabel("Prévia")).toContainText("Dó5+Mi5+Sol5 𝅗𝅥");
});

test("Inserir notas: a altura do ponteiro define a nota e o clique insere no compasso", async ({ page }) => {
  await newScore(page);
  await expect(page.locator(".score-at svg").nth(1)).toBeVisible({ timeout: 15_000 });
  await page.getByRole("radio", { name: /Inserir notas/ }).click();
  // linha de cima da pauta (clave de Sol) = Fá5, perto do início do compasso 1
  const target = await page.evaluate(() => {
    const host = document.querySelector(".score-at")!.getBoundingClientRect();
    const lines = [...document.querySelectorAll(".score-at svg rect")]
      .map((e) => e.getBoundingClientRect())
      .filter((b) => b.height <= 2.5 && b.width > 20)
      .sort((a, b) => a.y - b.y);
    const top = lines[0]!;
    return { x: top.x + 140, y: top.y + top.height / 2, hostX: host.x };
  });
  await page.mouse.move(target.x, target.y);
  const tip = page.locator(".score-tip");
  await expect(tip).toContainText("Fá5 · semínima");
  // com sustenido escolhido, o balão acompanha
  await page.getByRole("radio", { name: "Sustenido" }).click();
  await page.mouse.move(target.x, target.y + 1);
  await expect(tip).toContainText("Fá♯5");
  await page.mouse.click(target.x, target.y);
  await expect(page.locator(".event-strip")).toContainText("F♯5");
});
