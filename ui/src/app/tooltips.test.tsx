import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GLOSSARY, tipByKey, tipByName } from "../core/glossary";
import { EFFECT_LABELS, PARAMS } from "../contexts/mixing/effectsMeta";
import { PANEL_CATALOG } from "../panels/catalog";
import { findTipTarget, placeTip, TooltipLayer } from "../shared/TooltipLayer";
import { resolveTip } from "./tooltips";

describe("cobertura do glossário", () => {
  it("todo setor, efeito e parâmetro de efeito tem legenda", () => {
    const missing: string[] = [];
    for (const p of PANEL_CATALOG) if (!tipByKey(`panel.${p.id}`)) missing.push(`panel.${p.id}`);
    for (const t of Object.keys(EFFECT_LABELS)) if (!tipByKey(`effect.${t}`)) missing.push(`effect.${t}`);
    for (const [t, params] of Object.entries(PARAMS)) for (const p of params) if (!tipByKey(`param.${t}.${p.key}`)) missing.push(`param.${t}.${p.key}`);
    expect(missing).toEqual([]);
  });
  it("textos completos: título e explicação em todas as entradas", () => {
    for (const [k, v] of Object.entries(GLOSSARY)) {
      expect(v.title.length, k).toBeGreaterThan(0);
      expect(v.body.length, k).toBeGreaterThan(15);
    }
  });
});

describe("legenda pelo nome do controle", () => {
  it.each([
    ["Armar Voz", "Armar"],
    ["Mute Baixo", "Mute"],
    ["Solo Master", "Solo"],
    ["Pan Voz", "Pan"],
    ["Volume Voz", "Volume (fader)"],
    ["Tocar (Espaço)", "Tocar / pausar"],
    ["Pausar", "Tocar / pausar"],
    ["Loop", "Loop"],
    ["Metrônomo", "Metrônomo"],
    ["Envio de Voz", "Envio"],
    ["Editar Compressor", "Efeito (insert)"],
    ["Compressor", "Compressor"],
    ["Nível da entrada 2", "Nível da entrada"],
    ["Tamanho do buffer", "Buffer"],
  ])("%s → %s", (name, title) => {
    expect(tipByName(name)?.title).toBe(title);
  });
  it("não confunde o volume de monitoração com o fader nem adivinha parâmetros repetidos", () => {
    expect(tipByName("Volume de monitoração")?.title).toBe("Volume de monitoração");
    expect(tipByName("Limiar")).toBeUndefined(); // parâmetros usam chave explícita (compressor × gate)
    expect(tipByName("qualquer coisa")).toBeUndefined();
  });
});

describe("resolvedor do app", () => {
  const el = (html: string) => {
    const d = document.createElement("div");
    d.innerHTML = html;
    document.body.append(d);
    return d.firstElementChild!;
  };
  afterEach(() => (document.body.innerHTML = ""));
  it("chave explícita, aba do setor, nome acessível e título nativo, nessa ordem", () => {
    expect(resolveTip(el('<div data-tip="param.gate.thresholdDb"></div>'))?.meta).toContain("−90");
    expect(resolveTip(el('<div class="dv-tab">Gravação</div>'))?.title).toBe("Gravação");
    expect(resolveTip(el('<button aria-label="Solo Voz">S</button>'))?.title).toBe("Solo");
    expect(resolveTip(el('<button data-tip-text="Salvar (Ctrl+S)">Salvar</button>'))).toEqual({ body: "Salvar (Ctrl+S)" });
    expect(resolveTip(el("<span>texto solto</span>"))).toBeNull();
    // chave inexistente não aparece crua para o usuário
    expect(resolveTip(el('<div data-tip="nao.existe"></div>'))).toBeNull();
  });
  it("título nativo vira legenda e sai do atributo (sem legenda dupla)", () => {
    const b = el('<button title="Salvar (Ctrl+S)">Salvar</button>');
    const found = findTipTarget(b, resolveTip);
    expect(found?.tip.body).toBe("Salvar (Ctrl+S)");
    expect(b.hasAttribute("title")).toBe(false);
  });
  it("sobe até o ancestral com legenda e respeita data-tip-off", () => {
    const wrap = el('<div data-tip="effect.reverb"><span><b id="in">x</b></span></div>');
    expect(findTipTarget(wrap.querySelector("#in"), resolveTip)?.el).toBe(wrap);
    const off = el('<div data-tip-off><div data-tip="effect.reverb"><b id="in2">x</b></div></div>');
    expect(findTipTarget(off.querySelector("#in2"), resolveTip)).toBeNull();
  });
});

describe("posição", () => {
  const rect = (top: number, left: number, w = 40, h = 20) => ({ top, left, width: w, height: h, bottom: top + h, right: left + w, x: left, y: top, toJSON: () => ({}) }) as DOMRect;
  it("abaixo quando cabe, acima perto do rodapé, sempre dentro da janela", () => {
    expect(placeTip(rect(100, 100), { width: 200, height: 60 }, { width: 1000, height: 800 })).toMatchObject({ top: 128, above: false });
    expect(placeTip(rect(760, 100), { width: 200, height: 60 }, { width: 1000, height: 800 })).toMatchObject({ top: 692, above: true });
    expect(placeTip(rect(100, 980), { width: 200, height: 60 }, { width: 1000, height: 800 }).left).toBe(792);
    expect(placeTip(rect(100, 0), { width: 200, height: 60 }, { width: 1000, height: 800 }).left).toBe(8);
  });
});

describe("TooltipLayer", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = "";
  });
  it("mostra depois de parar o ponteiro, some ao sair, com Esc e quando desligada", () => {
    const { rerender } = render(
      <>
        <button aria-label="Solo Voz">S</button>
        <TooltipLayer resolve={resolveTip} />
      </>,
    );
    const b = screen.getByRole("button", { name: "Solo Voz" });
    act(() => void b.dispatchEvent(new PointerEvent("pointerover", { bubbles: true })));
    expect(screen.queryByRole("tooltip")).toBeNull();
    act(() => void vi.advanceTimersByTime(600));
    expect(screen.getByRole("tooltip").textContent).toContain("Ouve só os canais em solo");
    expect(b.getAttribute("aria-describedby")).toBe("ebiki-tooltip");
    act(() => void document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    expect(screen.queryByRole("tooltip")).toBeNull();
    rerender(
      <>
        <button aria-label="Solo Voz">S</button>
        <TooltipLayer resolve={resolveTip} enabled={false} />
      </>,
    );
    act(() => void screen.getByRole("button", { name: "Solo Voz" }).dispatchEvent(new PointerEvent("pointerover", { bubbles: true })));
    act(() => void vi.advanceTimersByTime(600));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });
});
