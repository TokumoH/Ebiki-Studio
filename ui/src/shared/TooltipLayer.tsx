// Camada única de legendas (design system): escuta o documento, espera o ponteiro parar ~0,5 s sobre
// um elemento (ou o foco por teclado) e mostra a legenda que `resolve` devolver. Não conhece o
// conteúdo: quem monta a camada fornece o resolvedor (glossário). `title` nativo vira legenda (e é
// retirado para não aparecer duas vezes).
import { useEffect, useLayoutEffect, useRef, useState } from "react";

export interface TipContent {
  title?: string;
  body: string;
  meta?: string;
}

export type TipResolver = (el: Element) => TipContent | null;

const SHOW_DELAY = 500;
const FOCUS_DELAY = 250;
const MAX_DEPTH = 6;
const TIP_ID = "ebiki-tooltip";

/** Primeiro ancestral (até MAX_DEPTH) com legenda. */
export function findTipTarget(start: Element | null, resolve: TipResolver): { el: Element; tip: TipContent } | null {
  let el: Element | null = start;
  for (let i = 0; el && i < MAX_DEPTH; i++, el = el.parentElement) {
    if (el.closest("[data-tip-off]")) return null;
    // o título nativo vira texto de legenda (e sai do atributo, para o navegador não mostrar o dele)
    const title = el.getAttribute("title");
    if (title) {
      el.setAttribute("data-tip-text", title);
      el.removeAttribute("title");
    }
    const tip = resolve(el);
    if (tip) return { el, tip };
  }
  return null;
}

/** Posição da legenda: abaixo do alvo se couber, senão acima; centrada e presa à janela. */
export function placeTip(target: DOMRect, size: { width: number; height: number }, viewport: { width: number; height: number }) {
  const gap = 8;
  const below = target.bottom + gap;
  const top = below + size.height <= viewport.height - gap ? below : Math.max(gap, target.top - gap - size.height);
  const left = Math.min(Math.max(gap, target.left + target.width / 2 - size.width / 2), Math.max(gap, viewport.width - size.width - gap));
  return { top, left, above: top < target.top };
}

export function TooltipLayer({ resolve, enabled = true }: { resolve: TipResolver; enabled?: boolean }) {
  const [shown, setShown] = useState<{ el: Element; tip: TipContent } | null>(null);
  const [pos, setPos] = useState<{ top: number; left: number; above: boolean } | null>(null);
  const bubble = useRef<HTMLDivElement>(null);
  const timer = useRef<number | undefined>(undefined);
  const resolveRef = useRef(resolve);
  resolveRef.current = resolve;

  useEffect(() => {
    if (!enabled) {
      setShown(null);
      return;
    }
    const clear = () => window.clearTimeout(timer.current);
    const hide = () => {
      clear();
      setShown(null);
    };
    const schedule = (target: Element | null, delay: number) => {
      clear();
      const found = findTipTarget(target, resolveRef.current);
      if (!found) return setShown((s) => (s && target && s.el.contains(target) ? s : null));
      setShown((s) => (s?.el === found.el ? s : null));
      timer.current = window.setTimeout(() => {
        if (found.el.isConnected) setShown(found);
      }, delay);
    };
    const over = (e: PointerEvent) => {
      if (e.pointerType === "touch") return;
      schedule(e.target as Element, SHOW_DELAY);
    };
    const out = (e: PointerEvent) => {
      if (!e.relatedTarget) hide();
    };
    const focusIn = (e: FocusEvent) => {
      const t = e.target as Element;
      if (t.matches?.(":focus-visible")) schedule(t, FOCUS_DELAY);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    document.addEventListener("pointerover", over, true);
    document.addEventListener("pointerout", out, true);
    document.addEventListener("pointerdown", hide, true);
    document.addEventListener("focusin", focusIn, true);
    document.addEventListener("focusout", hide, true);
    document.addEventListener("keydown", key, true);
    window.addEventListener("wheel", hide, { passive: true, capture: true });
    window.addEventListener("blur", hide);
    return () => {
      clear();
      document.removeEventListener("pointerover", over, true);
      document.removeEventListener("pointerout", out, true);
      document.removeEventListener("pointerdown", hide, true);
      document.removeEventListener("focusin", focusIn, true);
      document.removeEventListener("focusout", hide, true);
      document.removeEventListener("keydown", key, true);
      window.removeEventListener("wheel", hide, true);
      window.removeEventListener("blur", hide);
    };
  }, [enabled]);

  // posição depois de medir a legenda; liga a legenda ao alvo para leitores de tela
  useLayoutEffect(() => {
    if (!shown || !bubble.current) return setPos(null);
    const b = bubble.current.getBoundingClientRect();
    setPos(placeTip(shown.el.getBoundingClientRect(), { width: b.width, height: b.height }, { width: window.innerWidth, height: window.innerHeight }));
    const el = shown.el;
    const prev = el.getAttribute("aria-describedby");
    el.setAttribute("aria-describedby", prev ? `${prev} ${TIP_ID}` : TIP_ID);
    return () => {
      if (prev) el.setAttribute("aria-describedby", prev);
      else el.removeAttribute("aria-describedby");
    };
  }, [shown]);

  if (!shown) return null;
  const { tip } = shown;
  return (
    <div
      ref={bubble}
      id={TIP_ID}
      role="tooltip"
      className={`tooltip${pos?.above ? " tooltip--above" : ""}`}
      style={pos ? { top: pos.top, left: pos.left } : { top: -9999, left: -9999 }}
    >
      {tip.title ? <strong>{tip.title}</strong> : null}
      <span>{tip.body}</span>
      {tip.meta ? <small>{tip.meta}</small> : null}
    </div>
  );
}
