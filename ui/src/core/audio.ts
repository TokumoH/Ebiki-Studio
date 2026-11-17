// Estado do dispositivo de áudio compartilhado pela interface (barra de status, Configurações de
// áudio, painel Gravação, afinador): uma troca de dispositivo em qualquer lugar atualiza todos.
import { create } from "zustand";
import { api } from "../ipc/api";
import type { AudioStatus } from "../ipc/types";

interface AudioState {
  status: AudioStatus | null;
  setStatus: (s: AudioStatus) => void;
}

export const useAudio = create<AudioState>((set) => ({
  status: null,
  setStatus: (status) => set({ status }),
}));

/** Relê o estado do dispositivo (ao abrir o workspace ou um painel). */
export async function refreshAudio(): Promise<AudioStatus> {
  const s = await api.audioStatus();
  useAudio.getState().setStatus(s);
  return s;
}

/**
 * Troca só a entrada, mantendo saída e buffer atuais. `inputId` vazio/null = sem entrada.
 * Devolve o novo estado (o dispositivo reinicia; a monitoração é reaplicada se a rota existir).
 */
export async function setInputDevice(inputId: string | null): Promise<AudioStatus> {
  const cur = useAudio.getState().status ?? (await api.audioStatus());
  const s = await api.audioConfigure({ outputId: cur.outputId ?? null, inputId: inputId || null, bufferSize: validBuffer(cur.bufferSize) });
  useAudio.getState().setStatus(s);
  return s;
}

/** Ganho de entrada do microfone (dB, 0..+24): aplica na engine e atualiza o estado compartilhado. */
export async function setInputGain(gainDb: number): Promise<AudioStatus> {
  const s = await api.audioInputGainSet(gainDb);
  useAudio.getState().setStatus(s);
  return s;
}

/**
 * Abertura do diálogo "Configurações de áudio" — único lugar onde se escolhem o microfone e o fone.
 * Estado de interface no núcleo para que menus e painéis (ex.: Gravação) abram o mesmo diálogo sem
 * depender da camada `app`.
 */
interface AudioSettingsUiState {
  open: boolean;
  setOpen: (open: boolean) => void;
}

export const useAudioSettingsUi = create<AudioSettingsUiState>((set) => ({
  open: false,
  setOpen: (open) => set({ open }),
}));

/** Abre as Configurações de áudio (microfone e fone). */
export function openAudioSettings(): void {
  useAudioSettingsUi.getState().setOpen(true);
}

/** O backend só aceita potências de 2 entre 16 e 8192; outro valor (relatado pelo driver) = padrão. */
export function validBuffer(b: number | null | undefined): number | null {
  return b && b >= 16 && b <= 8192 && (b & (b - 1)) === 0 ? b : null;
}
