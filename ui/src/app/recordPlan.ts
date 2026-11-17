// O que fazer ao apertar Gravar (UX-09): gravar direto, armar a faixa selecionada (como Ableton/Logic)
// ou explicar o que falta, com um atalho para resolver. Função pura, testada sem a interface.
import type { SessionView } from "../ipc/types";

export type RecordPlan =
  | { kind: "record" }
  | { kind: "arm"; trackId: string; trackName: string }
  | { kind: "setup"; message: string };

export function recordPlan(session: SessionView | null, selectedTrackId: string | null, inputChannels: number): RecordPlan {
  if (!session) return { kind: "setup", message: "Abra um projeto para gravar." };
  const armed = session.tracks.some((t) => t.recording.armed && t.recording.input);
  if (armed) return { kind: "record" };
  if (inputChannels <= 0) return { kind: "setup", message: "Para gravar, escolha o microfone (entrada) e arme uma faixa." };
  const track = session.tracks.find((t) => t.id === selectedTrackId);
  if (!track) return { kind: "setup", message: "Para gravar, crie ou selecione uma faixa e arme-a (⏺ no cabeçalho)." };
  return { kind: "arm", trackId: track.id, trackName: track.name };
}
