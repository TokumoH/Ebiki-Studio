import { act, fireEvent, render, screen } from "@testing-library/react";
import { MenuBar, countActionable, type MenuDef } from "./MenuBar";

function setup() {
  const calls: string[] = [];
  const run = (id: string) => () => calls.push(id);
  const menus: MenuDef[] = [
    {
      id: "arquivo",
      label: "Arquivo",
      items: [
        { kind: "item", id: "novo", label: "Novo", keys: "Ctrl+N", run: run("novo") },
        { kind: "separator" },
        { kind: "item", id: "off", label: "Indisponível", disabled: true, run: run("off") },
        { kind: "item", id: "salvar", label: "Salvar", run: run("salvar") },
      ],
    },
    {
      id: "exibir",
      label: "Exibir",
      items: [
        { kind: "item", id: "escuro", label: "Escuro", checked: true, radio: true, run: run("escuro") },
        { kind: "submenu", id: "paineis", label: "Painéis", items: [{ kind: "item", id: "mixer", label: "Mixer", checked: false, run: run("mixer") }] },
      ],
    },
  ];
  render(<MenuBar menus={menus} />);
  return { calls, menus };
}

const flush = () => act(async () => new Promise((r) => requestAnimationFrame(() => r(undefined))));

describe("MenuBar", () => {
  it("F10 foca o primeiro menu; setas trocam de menu", () => {
    setup();
    fireEvent.keyDown(window, { key: "F10" });
    expect(screen.getByRole("menuitem", { name: "Arquivo" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Arquivo" }), { key: "ArrowRight" });
    expect(screen.getByRole("menuitem", { name: "Exibir" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Exibir" }), { key: "ArrowRight" });
    expect(screen.getByRole("menuitem", { name: "Arquivo" })).toHaveFocus();
  });

  it("Enter abre e foca o primeiro item; ↓ pula separadores e itens desativados; Enter executa e fecha", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout"] });
    const { calls } = setup();
    const top = screen.getByRole("menuitem", { name: "Arquivo" });
    top.focus();
    fireEvent.keyDown(top, { key: "Enter" });
    await flush();
    const menu = screen.getByRole("menu", { name: "Arquivo" });
    expect(screen.getByRole("menuitem", { name: /Novo/ })).toHaveFocus();
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(screen.getByRole("menuitem", { name: /Salvar/ })).toHaveFocus();
    fireEvent.click(screen.getByRole("menuitem", { name: /Salvar/ }));
    vi.runAllTimers();
    expect(calls).toEqual(["salvar"]);
    expect(screen.queryByRole("menu")).toBeNull();
    expect(top).toHaveFocus();
    vi.useRealTimers();
  });

  it("item desativado não executa; atalho aparece e é anunciado", async () => {
    const { calls } = setup();
    fireEvent.click(screen.getByRole("menuitem", { name: "Arquivo" }));
    const off = screen.getByRole("menuitem", { name: /Indisponível/ });
    expect(off).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(off);
    expect(calls).toEqual([]);
    expect(screen.getByRole("menuitem", { name: /Novo/ })).toHaveAttribute("aria-keyshortcuts", "Ctrl+N");
  });

  it("estados marcados usam os papéis certos; submenu abre com → e fecha com ←", async () => {
    setup();
    fireEvent.click(screen.getByRole("menuitem", { name: "Exibir" }));
    expect(screen.getByRole("menuitemradio", { name: /Escuro/ })).toHaveAttribute("aria-checked", "true");
    const trigger = screen.getByRole("menuitem", { name: /Painéis/ });
    trigger.focus();
    fireEvent.keyDown(screen.getByRole("menu", { name: "Exibir" }), { key: "ArrowRight" });
    await flush();
    expect(screen.getByRole("menu", { name: "Painéis" })).toBeInTheDocument();
    expect(screen.getByRole("menuitemcheckbox", { name: /Mixer/ })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menu", { name: "Painéis" }), { key: "ArrowLeft" });
    expect(screen.queryByRole("menu", { name: "Painéis" })).toBeNull();
    expect(trigger).toHaveFocus();
  });

  it("Esc fecha o menu e devolve o foco ao botão do menu", () => {
    setup();
    const top = screen.getByRole("menuitem", { name: "Arquivo" });
    fireEvent.click(top);
    fireEvent.keyDown(screen.getByRole("menu", { name: "Arquivo" }), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(top).toHaveFocus();
  });

  it("conta entradas acionáveis (inclui submenus, ignora separadores)", () => {
    const { menus } = setup();
    expect(countActionable(menus[0]!.items)).toBe(3);
    expect(countActionable(menus[1]!.items)).toBe(2);
  });
});

it("menu aberto pelo mouse: Esc fecha e mantém o foco no botão do menu", () => {
  setup();
  const top = screen.getByRole("menuitem", { name: "Exibir" });
  fireEvent.click(top);
  expect(top).toHaveFocus();
  fireEvent.keyDown(top, { key: "Escape" });
  expect(screen.queryByRole("menu")).toBeNull();
  expect(top).toHaveFocus();
  fireEvent.keyDown(top, { key: "Escape" });
  expect(top).not.toHaveFocus();
});
