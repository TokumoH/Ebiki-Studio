// U4: regras de dependência da UI (arquitetura de componentes).
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const SRC = dirname(fileURLToPath(import.meta.url));

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((f: string) => {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) return f === "__fixtures__" ? [] : files(p);
    return /\.(ts|tsx)$/.test(f) ? [p] : [];
  });
}

function imports(file: string): string[] {
  const src = readFileSync(file, "utf8");
  return [...src.matchAll(/from\s+["'](\.[^"']+)["']/g)].map((m) => relative(SRC, resolve(dirname(file), m[1]!)));
}

const layer = (p: string) => p.split(/[\\/]/)[0] ?? "";
const context = (p: string) => (p.startsWith("contexts") ? p.split(/[\\/]/)[1] : null);

describe("arquitetura da UI", () => {
  const all = files(SRC).filter((f) => !f.endsWith(".test.ts") && !f.endsWith(".test.tsx"));

  it("um contexto nunca importa outro contexto", () => {
    const bad: string[] = [];
    for (const f of all) {
      const me = context(relative(SRC, f));
      if (!me) continue;
      for (const i of imports(f)) {
        const other = context(i);
        if (other && other !== me) bad.push(`${relative(SRC, f)} → ${i}`);
      }
    }
    expect(bad).toEqual([]);
  });

  it("shared (design system) não depende de regra de negócio", () => {
    const bad = all.filter((f) => layer(relative(SRC, f)) === "shared").flatMap((f) => imports(f).filter((i) => ["core", "ipc", "contexts", "panels", "app"].includes(layer(i))));
    expect(bad).toEqual([]);
  });

  it("somente ipc/ conhece o Tauri", () => {
    const bad = all.filter((f) => layer(relative(SRC, f)) !== "ipc" && /@tauri-apps\//.test(readFileSync(f, "utf8")));
    expect(bad.map((f) => relative(SRC, f))).toEqual([]);
  });

  it("core e ipc não dependem de camadas de apresentação", () => {
    const bad = all
      .filter((f) => ["core", "ipc"].includes(layer(relative(SRC, f))))
      .flatMap((f) => imports(f).filter((i) => ["contexts", "panels", "app", "shared"].includes(layer(i))).map((i) => `${relative(SRC, f)} → ${i}`));
    expect(bad).toEqual([]);
  });
});
