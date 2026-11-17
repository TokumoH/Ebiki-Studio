// UX-10: textos com as cores de estado e a cor "faint" passam no contraste AA (4,5:1) sobre os fundos
// dos dois temas. Lê tokens.css: se alguém trocar uma cor por outra que não passa, este teste acusa.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(__dirname, "tokens.css"), "utf8");

function block(selector: string): Record<string, string> {
  const start = css.indexOf(selector);
  const body = css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
  return Object.fromEntries([...body.matchAll(/--([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})/g)].map((m) => [m[1]!, m[2]!]));
}

function luminance(hex: string): number {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((x) => (x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4));
  return 0.2126 * c[0]! + 0.7152 * c[1]! + 0.0722 * c[2]!;
}

function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x! + 0.05) / (y! + 0.05);
}

const TEXT = ["fg", "fg-muted", "fg-faint", "ok", "warn"];
const BACKGROUNDS = ["bg", "bg-raised", "bg-sunken", "panel", "panel-2"];

describe.each([
  ["escuro (Odoriko)", block(":root {")],
  ["claro (Agridoce)", { ...block(":root {"), ...block(':root[data-theme="light"]') }],
])("tema %s", (_name, t) => {
  for (const fg of TEXT)
    for (const bg of BACKGROUNDS)
      it(`--${fg} sobre --${bg} ≥ 4,5:1`, () => {
        expect(t[fg], `--${fg} definido`).toBeTruthy();
        expect(contrast(t[fg]!, t[bg]!)).toBeGreaterThanOrEqual(4.5);
      });
});
