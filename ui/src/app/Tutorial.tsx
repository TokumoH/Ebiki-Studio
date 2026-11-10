// Tutorial de início pulável: destaca áreas reais da interface (seletores `data-tour`) em 7 passos.
// Teclado: → / Enter avançam, ← volta, Esc pula. Lembra "não mostrar ao abrir" (localStorage).
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { create } from "zustand";
import { openPanel } from "../panels/dockControl";

const STORAGE_KEY = "ebiki.tutorial.v1";

export interface TourStep {
  title: string;
  body: string;
  /** Seletor do elemento destacado; sem alvo, o cartão fica centralizado. */
  target?: string;
  /** Preparação antes de medir o alvo (ex.: trazer um painel para a frente). */
  prepare?: () => void;
}

export const TOUR_STEPS: TourStep[] = [
  {
    title: "Boas-vindas ao Ebiki Studio",
    body: "Um estúdio completo para gravar, mixar, editar, compor e afinar — offline. Este tour mostra as áreas principais. Use → para avançar ou Esc para pular.",
  },
  {
    title: "Arranjo: faixas e clipes",
    body: "Arraste arquivos de áudio para cá ou use Arquivo › Importar. Arraste clipes para mover, as bordas para aparar e os cantos para fazer fades. S divide no cursor; Ctrl+Z desfaz tudo.",
    target: '[data-tour="arranjo"]',
    prepare: () => openPanel("timeline"),
  },
  {
    title: "Transporte e gravação",
    body: "Espaço toca e pausa, R grava, C liga o metrônomo e L faz loop de 4 compassos. Arme a faixa no ● e escolha a entrada no painel Gravação.",
    target: '[data-tour="transporte"]',
  },
  {
    title: "Mixer",
    body: "Volume, pan, efeitos e envios para buses. Os painéis de baixo (Mixer, Gravação, Harmonia, Partitura, Letra, Afinação…) ficam em abas encaixáveis.",
    target: '[data-tour="mixer"]',
    prepare: () => openPanel("mixer"),
  },
  {
    title: "Menu principal",
    body: "Clique na logo para o essencial (exportar, configurações, legendas, ajuda). Ao lado, todas as ações: Arquivo, Editar, Exibir, Transporte, Faixa, Clipe, Ferramentas, Janela e Ajuda. Alt ou F10 levam o teclado até ele.",
    target: '[data-tour="menu"]',
  },
  {
    title: "Exibição e painéis",
    body: "Em Exibir: tema, zoom, forma de onda ou espectrograma e restaurar o layout. Em Janela: abra, feche ou traga qualquer painel para a frente.",
    target: '[data-menu-id="exibir"], [data-menu-id="janela"]',
  },
  {
    title: "Ajuda sempre à mão",
    body: "F1 abre a central de ajuda com busca e atalhos. Este tour pode ser refeito em Ajuda › Tutorial de início. Boa música!",
    target: '[data-menu-id="ajuda"]',
  },
];

interface TourState {
  step: number | null;
  start: (step?: number) => void;
  stop: () => void;
}

export const useTour = create<TourState>((set) => ({
  step: null,
  start: (step = 1) => set({ step: Math.min(TOUR_STEPS.length, Math.max(1, step)) }),
  stop: () => set({ step: null }),
}));

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function tutorialDone(): boolean {
  return storage()?.getItem(STORAGE_KEY) === "done";
}

export function setTutorialDone(done: boolean) {
  try {
    if (done) storage()?.setItem(STORAGE_KEY, "done");
    else storage()?.removeItem(STORAGE_KEY);
  } catch {
    /* armazenamento indisponível: o tour volta a aparecer, sem prejuízo */
  }
}

/**
 * Mostrar o tour ao abrir o primeiro projeto? Não em automação (testes E2E/unitários),
 * onde ele cobriria a interface; lá ele é aberto explicitamente pelo menu Ajuda.
 */
export function shouldAutoStartTutorial(): boolean {
  if (tutorialDone()) return false;
  if (typeof navigator !== "undefined" && navigator.webdriver) return false;
  if (import.meta.env.MODE === "test") return false;
  return true;
}

interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

/** União dos retângulos dos elementos que casam com o seletor (com folga). */
function measure(selector: string | undefined): Box | null {
  if (!selector) return null;
  const rects = [...document.querySelectorAll(selector)].map((el) => el.getBoundingClientRect()).filter((r) => r.width > 0 && r.height > 0);
  if (!rects.length) return null;
  const pad = 6;
  const top = Math.min(...rects.map((r) => r.top)) - pad;
  const left = Math.min(...rects.map((r) => r.left)) - pad;
  const bottom = Math.max(...rects.map((r) => r.bottom)) + pad;
  const right = Math.max(...rects.map((r) => r.right)) + pad;
  return { top: Math.max(0, top), left: Math.max(0, left), width: right - Math.max(0, left), height: bottom - Math.max(0, top) };
}

const CARD_W = 380;

/** Posição do cartão: abaixo do alvo se couber, senão acima; centralizado sem alvo. */
export function cardPosition(box: Box | null, viewport: { width: number; height: number }, cardH: number) {
  if (!box) return { top: Math.max(16, (viewport.height - cardH) / 2), left: Math.max(16, (viewport.width - CARD_W) / 2) };
  const below = box.top + box.height + 14;
  const top = below + cardH <= viewport.height - 16 ? below : Math.max(16, box.top - cardH - 14);
  const left = Math.min(Math.max(16, box.left), Math.max(16, viewport.width - CARD_W - 16));
  return { top, left };
}

export function Tutorial() {
  const { step, start, stop } = useTour();
  const [box, setBox] = useState<Box | null>(null);
  const [dontShow, setDontShow] = useState(true);
  const card = useRef<HTMLElement>(null);
  const [cardH, setCardH] = useState(220);
  const current = step ? TOUR_STEPS[step - 1] : undefined;

  const finish = useCallback(() => {
    setTutorialDone(dontShow);
    stop();
  }, [dontShow, stop]);

  useLayoutEffect(() => {
    if (!current) return;
    current.prepare?.();
    let frame = 0;
    const update = () => {
      setBox(measure(current.target));
      if (card.current) setCardH(card.current.offsetHeight);
    };
    // o painel preparado pode levar um quadro para aparecer
    frame = requestAnimationFrame(() => requestAnimationFrame(update));
    window.addEventListener("resize", update);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", update);
    };
  }, [current]);

  useEffect(() => {
    if (!step) return;
    card.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish();
      else if (e.key === "ArrowRight") (step < TOUR_STEPS.length ? start(step + 1) : finish());
      else if (e.key === "ArrowLeft" && step > 1) start(step - 1);
      else return;
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [step, start, finish]);

  if (!step || !current) return null;
  const last = step === TOUR_STEPS.length;
  const pos = cardPosition(box, { width: window.innerWidth, height: window.innerHeight }, cardH);
  return (
    <div className="tour" data-testid="tour">
      {box ? (
        <div className="tour-spot" style={{ top: box.top, left: box.left, width: box.width, height: box.height }} />
      ) : (
        <div className="tour-dim" />
      )}
      <section ref={card} className="tour-card" role="dialog" aria-label={`Tutorial: passo ${step} de ${TOUR_STEPS.length}`} tabIndex={-1} style={{ top: pos.top, left: pos.left, width: CARD_W }}>
        <div className="tour-head">
          <span className="tour-count">
            Passo {step} de {TOUR_STEPS.length}
          </span>
          {!last ? (
            <button type="button" className="tour-skip" onClick={finish}>
              Pular tutorial
            </button>
          ) : null}
        </div>
        {step === 1 ? <span className="brand-mark tour-logo" role="img" aria-label="Ebiki Studio" /> : null}
        <h3>{current.title}</h3>
        <p>{current.body}</p>
        <div className="tour-dots" aria-hidden="true">
          {TOUR_STEPS.map((_, i) => (
            <span key={i} className={i + 1 === step ? "on" : i + 1 < step ? "done" : ""} />
          ))}
        </div>
        <div className="tour-actions">
          <label className="tour-dont">
            <input type="checkbox" checked={dontShow} onChange={(e) => setDontShow(e.target.checked)} /> Não mostrar ao abrir
          </label>
          {step > 1 ? (
            <button type="button" className="btn btn--sm" onClick={() => start(step - 1)}>
              Voltar
            </button>
          ) : null}
          <button type="button" className="btn btn--sm btn--primary" onClick={() => (last ? finish() : start(step + 1))}>
            {last ? "Concluir" : "Próximo"}
          </button>
        </div>
      </section>
    </div>
  );
}
