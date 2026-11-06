import { useEffect, useId, useRef, useState } from "react";
import { Button } from "../shared/components";
import "./panels.css";
import { PANEL_CATALOG } from "./catalog";
import { openPanel, useDockState } from "./dockControl";

/** Menu "Painéis": lista todos os painéis do catálogo e reabre/foca o escolhido (teclado: ↑ ↓ Home End Enter Esc). */
export function PanelsMenu() {
  const open = useDockState((s) => s.open);
  const [expanded, setExpanded] = useState(false);
  const [pos, setPos] = useState({ top: 0, left: 0 });
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuId = useId();

  const items = () => Array.from(menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitemcheckbox"]') ?? []);
  const close = (refocus: boolean) => {
    setExpanded(false);
    if (refocus) buttonRef.current?.focus();
  };
  const show = (focus: "first" | "last") => {
    const r = buttonRef.current?.getBoundingClientRect();
    if (r) setPos({ top: r.bottom + 4, left: Math.max(4, Math.min(r.left, window.innerWidth - 230)) });
    setExpanded(true);
    requestAnimationFrame(() => {
      const list = items();
      (focus === "first" ? list[0] : list[list.length - 1])?.focus();
    });
  };

  useEffect(() => {
    if (!expanded) return;
    const onDown = (e: PointerEvent) => {
      const t = e.target as Node;
      if (!menuRef.current?.contains(t) && !buttonRef.current?.contains(t)) setExpanded(false);
    };
    // Esc fecha o menu onde quer que esteja o foco (inclusive antes de ele entrar no menu)
    const onEsc = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      setExpanded(false);
      buttonRef.current?.focus();
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", onEsc, true);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", onEsc, true);
    };
  }, [expanded]);

  const onMenuKey = (e: React.KeyboardEvent) => {
    const list = items();
    const i = list.indexOf(document.activeElement as HTMLElement);
    const go = (n: number) => list[(n + list.length) % list.length]?.focus();
    if (e.key === "ArrowDown") go(i + 1);
    else if (e.key === "ArrowUp") go(i - 1);
    else if (e.key === "Home") go(0);
    else if (e.key === "End") go(list.length - 1);
    else if (e.key === "Tab") return close(false);
    else return;
    e.preventDefault();
    e.stopPropagation();
  };

  return (
    <>
      <Button
        ref={buttonRef}
        size="sm"
        variant="ghost"
        aria-haspopup="menu"
        aria-expanded={expanded}
        aria-controls={expanded ? menuId : undefined}
        title="Mostrar painéis"
        onClick={() => (expanded ? close(false) : show("first"))}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            show(e.key === "ArrowDown" ? "first" : "last");
          }
        }}
      >
        Painéis ▾
      </Button>
      {expanded ? (
        <div id={menuId} ref={menuRef} className="menu" role="menu" aria-label="Painéis" style={{ top: pos.top, left: pos.left }} onKeyDown={onMenuKey}>
          {PANEL_CATALOG.map((p) => {
            const isOpen = open.includes(p.id);
            return (
              <button
                key={p.id}
                type="button"
                role="menuitemcheckbox"
                aria-checked={isOpen}
                tabIndex={-1}
                className="menu-item"
                onClick={() => {
                  openPanel(p.id);
                  setExpanded(false);
                }}
              >
                <span className="menu-check" aria-hidden="true">
                  {isOpen ? "✓" : ""}
                </span>
                {p.title}
              </button>
            );
          })}
        </div>
      ) : null}
    </>
  );
}
