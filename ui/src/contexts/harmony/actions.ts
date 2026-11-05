// Ações do contexto Harmonia que tocam o núcleo (audição e inserção no arranjo).
import { compositionApi } from "../../ipc/api";
import type { HarmonyKey, ProgressionRef } from "../../ipc/types";
import { audition } from "../../core/audition";
import { useLive } from "../../core/live";
import { errorMessage, run, useSession } from "../../core/session";
import { notify } from "../../core/toasts";

export type InsertTarget = "new" | "selected";

export function refId(ref: ProgressionRef): string {
  switch (ref.kind) {
    case "builtin":
      return `prog:${ref.slug}`;
    case "user":
      return `user:${ref.id}`;
    case "roman":
      return `roman:${ref.steps}`;
    case "cifra":
      return `cifra:${ref.chords}`;
  }
}

/** Ouve a progressão no tom (WAV renderizado pelo núcleo no BPM do projeto). */
export async function auditionProgression(progression: ProgressionRef, key: HarmonyKey) {
  try {
    await audition(refId(progression), () => compositionApi.harmonyAudition({ progression, key }));
  } catch (e) {
    notify.error(errorMessage(e));
  }
}

/** Insere a progressão no cursor (nova faixa ou faixa selecionada) como um único passo desfazível. */
export function insertProgression(progression: ProgressionRef, key: HarmonyKey, target: InsertTarget): Promise<boolean> {
  const atTick = Math.max(0, useLive.getState().transport?.tick ?? 0);
  const selected = useSession.getState().selectedTrackId;
  const trackId = target === "selected" ? selected : null;
  return run(
    async () => {
      const r = await compositionApi.harmonyInsertProgression({ realize: { progression, key }, atTick, trackId });
      useSession.getState().select({ trackId: r.trackId, clipId: r.clipId });
      return r.view;
    },
    { success: "Progressão inserida no arranjo.", busy: true },
  );
}
