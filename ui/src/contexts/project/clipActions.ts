// Ações de edição de clipe (Fase 3) compartilhadas por menu de contexto, inspetor e atalhos.
import { editApi } from "../../ipc/api";
import type { Clip, SessionView, Track } from "../../ipc/types";
import { run, useSession } from "../../core/session";
import { useLive } from "../../core/live";
import { notify } from "../../core/toasts";
import { canSplitAt, overlappingNeighbor } from "./clipEdit";

export function findClip(s: SessionView | null, clipId: string | null): { track: Track; clip: Clip } | null {
  if (!s || !clipId) return null;
  for (const track of s.tracks) {
    const clip = track.clips.find((c) => c.id === clipId);
    if (clip) return { track, clip };
  }
  return null;
}

/** Clipe alvo: o selecionado; senão, o da faixa selecionada sob o cursor. */
export function splitTarget(s: SessionView | null, selectedClipId: string | null, selectedTrackId: string | null, tick: number): Clip | null {
  const sel = findClip(s, selectedClipId)?.clip;
  if (sel) return sel;
  const t = s?.tracks.find((x) => x.id === selectedTrackId);
  return t?.clips.find((c) => canSplitAt(c, tick)) ?? null;
}

/** Divide no cursor (atalho S). */
export async function splitAtCursor(clipId?: string): Promise<boolean> {
  const { session, selectedClipId, selectedTrackId } = useSession.getState();
  const tick = Math.round(useLive.getState().transport?.tick ?? 0);
  const c = clipId ? findClip(session, clipId)?.clip : splitTarget(session, selectedClipId, selectedTrackId, tick);
  if (!c) {
    notify.error("Selecione um clipe para dividir.");
    return false;
  }
  if (!canSplitAt(c, tick)) {
    notify.error("Posicione o cursor dentro do clipe para dividir.");
    return false;
  }
  return run(() => editApi.clipSplit(c.id, tick));
}

export const normalizeClip = (clipId: string) => run(() => editApi.clipNormalize(clipId), { success: "Clipe normalizado (−1 dBFS)." });
export const setPolarity = (clipId: string, inverted: boolean) => run(() => editApi.clipSetPolarity(clipId, inverted));
export const setReversed = (clipId: string, reversed: boolean) => run(() => editApi.clipSetReversed(clipId, reversed));

/** Crossfade (equal power) com o clipe que se sobrepõe ao dado na mesma faixa. */
export async function crossfadeWithNeighbor(clipId: string): Promise<boolean> {
  const found = findClip(useSession.getState().session, clipId);
  const other = found ? overlappingNeighbor(found.track, clipId) : null;
  if (!other) {
    notify.error("Sobreponha dois clipes da mesma faixa para criar um crossfade.");
    return false;
  }
  return run(() => editApi.clipCrossfade(clipId, other.id, "equalPower"));
}
