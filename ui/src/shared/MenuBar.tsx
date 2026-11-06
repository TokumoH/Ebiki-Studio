// Barra de menus fixa (estilo Photoshop / VS Code), dirigida por dados e acessível por teclado:
// Alt ou F10 foca a barra; ←/→ trocam de menu; ↓/Enter abrem; ↑/↓ percorrem; → abre submenu,
// ← fecha; Enter/Espaço ativam; Esc fecha. Não conhece IPC nem domínio (design system).
import { useCallback, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

export type MenuEntry =
  | { kind: "item"; id: string; label: string; keys?: string; disabled?: boolean; checked?: boolean; radio?: boolean; run: () => void }
  | { kind: "submenu"; id: string; label: string; items: MenuEntry[] }
  | { kind: "separator" };

export interface MenuDef {
  id: string;
  label: string;
  items: MenuEntry[];
}

type Actionable = Exclude<MenuEntry, { kind: "separator" }>;

const actionable = (items: MenuEntry[]) => items.filter((i): i is Actionable => i.kind !== "separator");

/** Itens navegáveis diretos de um menu (inclui o gatilho de submenu, que fica num invólucro). */
function menuItems(root: Element | null): HTMLButtonElement[] {
  if (!root) return [];
  return [
    ...root.querySelectorAll<HTMLButtonElement>(
      ":scope > [role^='menuitem']:not([aria-disabled='true']), :scope > .menubar-subwrap > [role^='menuitem']",
    ),
  ];
}

export function MenuBar({ menus, label = "Menu principal" }: { menus: MenuDef[]; label?: string }) {
  const [open, setOpen] = useState<number | null>(null);
  const [sub, setSub] = useState<string | null>(null);
  const bar = useRef<HTMLDivElement>(null);
  const tops = useRef<(HTMLButtonElement | null)[]>([]);

  const close = useCallback((refocus: number | null = null) => {
    setOpen(null);
    setSub(null);
    if (refocus !== null) tops.current[refocus]?.focus();
  }, []);

  // Alt (sozinho) ou F10 levam o foco ao primeiro menu; clique fora fecha
  useEffect(() => {
    let altAlone = false;
    const down = (e: KeyboardEvent) => {
      if (e.key === "F10" && !e.shiftKey) {
        e.preventDefault();
        tops.current[0]?.focus();
        return;
      }
      altAlone = e.key === "Alt";
    };
    const up = (e: KeyboardEvent) => {
      if (e.key === "Alt" && altAlone && !document.querySelector(".dialog-backdrop")) {
        e.preventDefault();
        // alterna: dentro da barra, Alt sai dela; fora, leva o foco ao primeiro menu
        if (bar.current?.contains(document.activeElement)) {
          close();
          (document.activeElement as HTMLElement | null)?.blur();
        } else tops.current[0]?.focus();
      }
      altAlone = false;
    };
    const pointer = (e: PointerEvent) => {
      if (!bar.current?.contains(e.target as Node)) close();
    };
    window.addEventListener("keydown", down, true);
    window.addEventListener("keyup", up, true);
    window.addEventListener("pointerdown", pointer, true);
    return () => {
      window.removeEventListener("keydown", down, true);
      window.removeEventListener("keyup", up, true);
      window.removeEventListener("pointerdown", pointer, true);
    };
  }, [close]);

  const focusItem = (root: HTMLElement | null, which: "first" | "last" | number) => {
    const items = menuItems(root);
    if (!items.length) return;
    const i = which === "first" ? 0 : which === "last" ? items.length - 1 : (which + items.length) % items.length;
    items[i]?.focus();
  };

  const openMenu = (i: number, focus: "first" | "last" | null) => {
    const n = (i + menus.length) % menus.length;
    setOpen(n);
    setSub(null);
    tops.current[n]?.focus();
    if (focus) requestAnimationFrame(() => focusItem(document.getElementById(`menu-${menus[n]!.id}`), focus));
  };

  const onTopKey = (e: ReactKeyboardEvent, i: number) => {
    const k = e.key;
    if (k === "ArrowRight" || k === "ArrowLeft") {
      e.preventDefault();
      const n = i + (k === "ArrowRight" ? 1 : -1);
      if (open !== null) openMenu(n, "first");
      else tops.current[(n + menus.length) % menus.length]?.focus();
    } else if (k === "ArrowDown" || k === "Enter" || k === " ") {
      e.preventDefault();
      openMenu(i, "first");
    } else if (k === "ArrowUp") {
      e.preventDefault();
      openMenu(i, "last");
    } else if (k === "Escape") {
      e.preventDefault();
      // com menu aberto, Esc só fecha o menu (o foco fica no botão); sem menu, sai da barra
      if (open !== null) close(i);
      else (document.activeElement as HTMLElement | null)?.blur();
    }
  };

  const onMenuKey = (e: ReactKeyboardEvent<HTMLDivElement>, menuIndex: number, inSub: boolean) => {
    const list = e.currentTarget;
    const items = menuItems(list);
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const k = e.key;
    if (k === "ArrowDown") focusItem(list, i < 0 ? "first" : i + 1);
    else if (k === "ArrowUp") focusItem(list, i < 0 ? "last" : i - 1);
    else if (k === "Home") focusItem(list, "first");
    else if (k === "End") focusItem(list, "last");
    else if (k === "ArrowLeft") {
      if (inSub) {
        setSub(null);
        (list.parentElement?.querySelector<HTMLButtonElement>(":scope > [aria-haspopup='menu']") ?? null)?.focus();
      } else openMenu(menuIndex - 1, "first");
    } else if (k === "ArrowRight") {
      const cur = document.activeElement as HTMLElement | null;
      if (cur?.getAttribute("aria-haspopup") === "menu") cur.click();
      else if (!inSub) openMenu(menuIndex + 1, "first");
      else return;
    } else if (k === "Escape") {
      if (inSub) {
        setSub(null);
        (list.parentElement?.querySelector<HTMLButtonElement>(":scope > [aria-haspopup='menu']") ?? null)?.focus();
      } else close(menuIndex);
    } else if (k === "Tab") {
      close();
      return;
    } else return;
    e.preventDefault();
    e.stopPropagation();
  };

  const activate = (item: Extract<MenuEntry, { kind: "item" }>, menuIndex: number) => {
    if (item.disabled) return;
    close(menuIndex);
    // depois de fechar: a ação pode abrir diálogos que tomam o foco
    setTimeout(item.run, 0);
  };

  const renderEntries = (items: MenuEntry[], menuIndex: number) =>
    items.map((it, k) => {
      if (it.kind === "separator") return <div key={`sep-${k}`} role="separator" className="menubar-sep" />;
      if (it.kind === "submenu") {
        const expanded = sub === it.id;
        return (
          <div key={it.id} className="menubar-subwrap" onPointerEnter={() => setSub(it.id)}>
            <button
              type="button"
              role="menuitem"
              className="menubar-item"
              aria-haspopup="menu"
              aria-expanded={expanded}
              onClick={() => {
                setSub(it.id);
                requestAnimationFrame(() => focusItem(document.getElementById(`menu-${it.id}`), "first"));
              }}
            >
              <span className="menubar-check" aria-hidden="true" />
              <span className="menubar-label">{it.label}</span>
              <span className="menubar-keys menubar-arrow" aria-hidden="true">›</span>
            </button>
            {expanded ? (
              <div id={`menu-${it.id}`} role="menu" aria-label={it.label} className="menubar-menu menubar-submenu" onKeyDown={(e) => onMenuKey(e, menuIndex, true)}>
                {renderEntries(it.items, menuIndex)}
              </div>
            ) : null}
          </div>
        );
      }
      const role = it.checked === undefined ? "menuitem" : it.radio ? "menuitemradio" : "menuitemcheckbox";
      return (
        <button
          key={it.id}
          type="button"
          role={role}
          className="menubar-item"
          aria-checked={it.checked === undefined ? undefined : it.checked}
          aria-disabled={it.disabled || undefined}
          aria-keyshortcuts={it.keys}
          data-command={it.id}
          onPointerEnter={() => setSub(null)}
          onClick={() => activate(it, menuIndex)}
        >
          <span className="menubar-check" aria-hidden="true">
            {it.checked ? (it.radio ? "●" : "✓") : ""}
          </span>
          <span className="menubar-label">{it.label}</span>
          <span className="menubar-keys">{it.keys ?? ""}</span>
        </button>
      );
    });

  return (
    <div ref={bar} role="menubar" aria-label={label} className="menubar">
      {menus.map((m, i) => (
        <div key={m.id} className="menubar-top">
          <button
            ref={(el) => {
              tops.current[i] = el;
            }}
            type="button"
            role="menuitem"
            className="menubar-button"
            data-menu-id={m.id}
            aria-haspopup="menu"
            aria-expanded={open === i}
            tabIndex={i === 0 ? 0 : -1}
            onClick={() => (open === i ? close() : openMenu(i, null))}
            onPointerEnter={() => open !== null && open !== i && openMenu(i, null)}
            onKeyDown={(e) => onTopKey(e, i)}
          >
            {m.label}
          </button>
          {open === i ? (
            <div id={`menu-${m.id}`} role="menu" aria-label={m.label} className="menubar-menu" onKeyDown={(e) => onMenuKey(e, i, false)}>
              {renderEntries(m.items, i)}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}

/** Quantidade de entradas acionáveis (útil em testes e para validar modelos de menu). */
export function countActionable(items: MenuEntry[]): number {
  return actionable(items).reduce((n, i) => n + (i.kind === "submenu" ? countActionable(i.items) : 1), 0);
}
