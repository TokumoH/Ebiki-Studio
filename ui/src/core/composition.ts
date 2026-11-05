// Estado de UI compartilhado entre os contextos de composição (Harmonia → Bases), sem que um
// contexto importe o outro. O projeto não guarda tom no núcleo: a Harmonia publica o último tom
// escolhido e o diálogo de inserção de base usa-o como "tom do projeto" padrão.
import { create } from "zustand";

export interface ChosenKey {
  tonic: number;
  /** Modo do catálogo de Harmonia (ex.: "ionian", "aeolian"). */
  mode: string;
  /** Terça menor (define maior/menor para as bases). */
  isMinor: boolean;
}

interface CompositionState {
  key: ChosenKey;
  setKey: (k: ChosenKey) => void;
}

export const useComposition = create<CompositionState>((set) => ({
  key: { tonic: 0, mode: "ionian", isMinor: false },
  setKey: (key) => set({ key }),
}));
