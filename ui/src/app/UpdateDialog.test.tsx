import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { setUpdater, type Updater } from "../ipc/updater";
import { plainNotes, UpdateDialog } from "./UpdateDialog";
import { loadUpdatePrefs, useUpdates } from "./updates";

const initial = useUpdates.getState();

const NOTES = `## [9.0.0] - 2026-12-12

Texto de abertura.

### Novidades

- **Automação** de volume e pan
- Exportação em \`AAC\` — veja o [manual](https://x/manual)

### Downloads

| Sistema | Arquivo |`;

function fake(over: Partial<Updater> = {}): Updater {
  return {
    mode: async () => "inApp",
    check: async () => ({ version: "9.0.0", date: "2026-12-12T12:00:00Z", notes: NOTES }),
    download: async () => {},
    install: async () => {},
    relaunch: async () => {},
    openReleasePage: vi.fn(async () => {}),
    ...over,
  };
}

beforeEach(() => {
  localStorage.clear();
  useUpdates.setState({ ...initial, prefs: loadUpdatePrefs() }, true);
});

describe("plainNotes", () => {
  it("usa só a seção Novidades e remove a marcação", () => {
    expect(plainNotes(NOTES)).toBe("• Automação de volume e pan\n• Exportação em AAC — veja o manual");
  });
  it("sem seção Novidades, usa o texto todo", () => {
    expect(plainNotes("- item **forte**")).toBe("• item forte");
  });
  it("não interpreta HTML (fica como texto)", () => {
    render(<p>{plainNotes("<img src=x onerror=alert(1)>")}</p>);
    expect(document.querySelector("img")).toBeNull();
  });
});

describe("UpdateDialog", () => {
  it("mostra a versão nova, as novidades e instala", async () => {
    const install = vi.fn(async () => {});
    setUpdater(fake({ install }));
    const save = vi.fn(async () => {});
    render(<UpdateDialog beforeInstall={save} />);
    await useUpdates.getState().check({ manual: true });
    expect(await screen.findByText("Atualização disponível")).toBeTruthy();
    expect(screen.getByText(/Automação de volume e pan/)).toBeTruthy();
    expect(screen.queryByText(/Sistema/)).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Instalar e reiniciar" }));
    expect(install).toHaveBeenCalled();
    expect(save).toHaveBeenCalled();
  });

  it("instalação .deb/.rpm oferece a página de download", async () => {
    const openReleasePage = vi.fn(async () => {});
    setUpdater(fake({ mode: async () => "manual", openReleasePage }));
    render(<UpdateDialog beforeInstall={async () => {}} />);
    await useUpdates.getState().check({ manual: true });
    expect(screen.queryByRole("button", { name: "Instalar e reiniciar" })).toBeNull();
    await userEvent.click(await screen.findByRole("button", { name: "Abrir página de download" }));
    expect(openReleasePage).toHaveBeenCalled();
  });

  it("a caixa 'Verificar ao abrir' grava a preferência", async () => {
    setUpdater(fake({ check: async () => null }));
    render(<UpdateDialog beforeInstall={async () => {}} />);
    await useUpdates.getState().check({ manual: true });
    expect(await screen.findByText("Você está na versão mais recente")).toBeTruthy();
    const box = screen.getByRole("checkbox", { name: "Verificar ao abrir o app" });
    expect((box as HTMLInputElement).checked).toBe(false);
    await userEvent.click(box);
    expect(loadUpdatePrefs().checkOnOpen).toBe(true);
  });
});
