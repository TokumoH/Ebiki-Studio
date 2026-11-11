import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { Clip } from "../../ipc/types";

export interface ClipMenuItem {
  id: string;
  label: string;
  shortcut?: string;
  /** Item de alternância (Inverter fase, Reverter). */
  checked?: boolean;
  disabled?: boolean;
  run: () => void;
}

/** Itens do menu do clipe (puro: testável sem DOM). */
export function clipMenuItems(clip: Clip, a: { canSplit: boolean; hasOverlap: boolean; split: () => void; normalize: () => void; polarity: (v: boolean) => void; reverse: (v: boolean) => void; crossfade: () => void; remove: () => void }): ClipMenuItem[] {
  return [
    { id: "split", label: "Dividir no cursor", shortcut: "S", disabled: !a.canSplit, run: a.split },
    { id: "normalize", label: "Normalizar (−1 dBFS)", run: a.normalize },
    { id: "polarity", label: "Inverter fase", checked: clip.polarityInverted, run: () => a.polarity(!clip.polarityInverted) },
    { id: "reverse", label: "Reverter", checked: clip.reversed, run: () => a.reverse(!clip.reversed) },
    { id: "crossfade", label: "Crossfade com clipe sobreposto", disabled: !a.hasOverlap, run: a.crossfade },
    { id: "delete", label: "Remover clipe", shortcut: "Del", run: a.remove },
  ];
}

/** Menu de contexto acessível (setas, Home/End, Enter/Espaço, Esc). */
/**
 * Posição do menu (coordenadas da tela) dentro da janela: abre para baixo/direita do ponteiro e vira
 * para cima/esquerda quando não cabe.
 */
export function menuPosition(x: number, y: number, size: { width: number; height: number }, viewport: { width: number; height: number }) {
  const m = 8;
  const left = x + size.width + m <= viewport.width ? x : Math.max(m, x - size.width);
  const top = y + size.height + m <= viewport.height ? y : Math.max(m, y - size.height);
  return { left, top };
}

/** Menu de contexto do clipe. `x`/`y` em coordenadas da tela; vai para o body (painéis cortariam). */
export function ClipContextMenu({ x, y, items, onClose, label }: { x: number; y: number; items: ClipMenuItem[]; onClose: () => void; label: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  useLayoutEffect(() => {
    const r = ref.current?.getBoundingClientRect();
    if (r) setPos(menuPosition(x, y, { width: r.width, height: r.height }, { width: window.innerWidth, height: window.innerHeight }));
  }, [x, y]);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])')?.focus();
    const away = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener("pointerdown", away, true);
    return () => window.removeEventListener("pointerdown", away, true);
  }, [onClose]);
  const focusable = () => Array.from(ref.current?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])') ?? []);
  const move = (dir: number | "first" | "last") => {
    const list = focusable();
    if (!list.length) return;
    const i = list.indexOf(document.activeElement as HTMLElement);
    const next = dir === "first" ? 0 : dir === "last" ? list.length - 1 : (i + dir + list.length) % list.length;
    list[next]?.focus();
  };
  return createPortal(
    <div
      ref={ref}
      className="clip-menu"
      role="menu"
      aria-label={label}
      style={pos ?? { left: x, top: y, opacity: 0 }}
      onKeyDown={(e) => {
        const keys: Record<string, () => void> = { ArrowDown: () => move(1), ArrowUp: () => move(-1), Home: () => move("first"), End: () => move("last"), Escape: onClose, Tab: onClose };
        const k = keys[e.key];
        if (!k) return;
        e.preventDefault();
        e.stopPropagation();
        k();
      }}
    >
      {items.map((it) => (
        <div
          key={it.id}
          role={it.checked === undefined ? "menuitem" : "menuitemcheckbox"}
          aria-checked={it.checked}
          aria-disabled={it.disabled || undefined}
          tabIndex={-1}
          className="clip-menu-item"
          onClick={() => {
            if (it.disabled) return;
            onClose();
            it.run();
          }}
          onKeyDown={(e) => {
            if ((e.key === "Enter" || e.key === " ") && !it.disabled) {
              e.preventDefault();
              e.stopPropagation();
              onClose();
              it.run();
            }
          }}
        >
          <span className="clip-menu-check" aria-hidden="true">{it.checked ? "✓" : ""}</span>
          <span className="clip-menu-label">{it.label}</span>
          {it.shortcut ? <kbd className="clip-menu-kbd">{it.shortcut}</kbd> : null}
        </div>
      ))}
    </div>,
    document.body,
  );
}
