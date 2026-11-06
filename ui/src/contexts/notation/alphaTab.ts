// Adaptador do alphaTab (renderização de partitura/tablatura e conversão GP3–GP6 → GP7, ADR-0020).
// Carregado sob demanda (chunk próprio); fonte Bravura empacotada pelo Vite (funciona offline,
// CSP `font-src 'self' data:`); sem workers e sem player (a audição vem do núcleo).
import bravuraWoff2 from "@coderline/alphatab/font/Bravura.woff2?url";

type AlphaTabModule = typeof import("@coderline/alphatab");

let mod: Promise<AlphaTabModule> | null = null;

export function loadAlphaTab(): Promise<AlphaTabModule> {
  mod ??= import("@coderline/alphatab");
  return mod;
}

/** Configuração offline do alphaTab (JSON de settings). */
export function alphaTabSettings(scale = 0.9) {
  return {
    core: {
      useWorkers: false,
      enableLazyLoading: false,
      fontDirectory: null,
      smuflFontSources: new Map([["woff2", bravuraWoff2]]) as Map<"woff2", string>,
      logLevel: "warning" as const,
    },
    display: { scale, layoutMode: "page" as const, staveProfile: "default" as const },
    player: { enablePlayer: false },
  };
}

/** GP3/GP4/GP5/GPX: formatos binários que o núcleo não lê (convertidos aqui para GP7). */
export function isLegacyGuitarPro(fileName: string): boolean {
  return /\.(gp3|gp4|gp5|gpx)$/i.test(fileName);
}

/** Converte um arquivo Guitar Pro antigo para GP7 (`.gp`) com o `Gp7Exporter` do alphaTab. */
export async function convertToGp7(bytes: Uint8Array): Promise<Uint8Array> {
  const at = await loadAlphaTab();
  const settings = new at.Settings();
  const score = at.importer.ScoreLoader.loadScoreFromBytes(bytes, settings);
  return new at.exporter.Gp7Exporter().export(score, settings);
}
