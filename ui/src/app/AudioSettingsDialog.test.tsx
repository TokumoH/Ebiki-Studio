import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { setTransport, type Transport } from "../ipc/transport";
import { createMockTransport } from "../ipc/mock";
import { api } from "../ipc/api";
import { useAudio, useAudioSettingsUi } from "../core/audio";
import { useLive } from "../core/live";
import type { MetersView } from "../ipc/types";
import { AudioSettingsDialog, conversionNote, formatRate, roundTripMs } from "./AudioSettingsDialog";

/** Transporte simulado que registra os comandos chamados. */
function spyTransport(): { t: Transport; calls: { cmd: string; args?: Record<string, unknown> }[] } {
  const base = createMockTransport();
  const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
  const t: Transport = {
    ...base,
    kind: base.kind,
    invoke: <T,>(cmd: string, args?: Record<string, unknown>) => {
      calls.push({ cmd, args });
      return base.invoke<T>(cmd, args);
    },
    invokeBinary: base.invokeBinary.bind(base),
    listen: base.listen.bind(base),
    pickOpen: base.pickOpen.bind(base),
    pickSave: base.pickSave.bind(base),
  };
  return { t, calls };
}

const meters = (inputs: number[]): MetersView => ({ master: [0, 0], tracks: [], buses: [], inputs, xruns: 0, recordOverflow: 0 });

describe("AudioSettingsDialog", () => {
  let spy: ReturnType<typeof spyTransport>;
  beforeEach(async () => {
    spy = spyTransport();
    setTransport(spy.t);
    useLive.setState({ meters: null, transport: null });
    useAudioSettingsUi.getState().setOpen(false);
  });

  const openDialog = async () => {
    render(<AudioSettingsDialog />);
    act(() => useAudioSettingsUi.getState().setOpen(true));
    await waitFor(() => expect(screen.getByText(/Sistema de áudio: Simulado/)).toBeInTheDocument());
  };

  it("separa o que você ouve (fone) do que você grava (microfone), sem o jargão Driver", async () => {
    await openDialog();
    const dialog = screen.getByRole("dialog", { name: "Configurações de áudio" });
    expect(within(dialog).queryByText(/Driver/)).toBeNull();
    expect(screen.getByText("Sistema de áudio: Simulado · 48 kHz")).toBeInTheDocument();
    const out = screen.getByRole("region", { name: "Fone / alto-falantes — o que você ouve" });
    expect(within(out).getByLabelText("Dispositivo de saída")).toBeInTheDocument();
    expect(within(out).getByRole("button", { name: "Testar som" })).toBeInTheDocument();
    expect(within(out).getByText(/o Ebiki não reduz o volume/)).toBeInTheDocument();
    const mic = screen.getByRole("region", { name: "Microfone / interface — o que você grava" });
    expect(within(mic).getByLabelText("Dispositivo de entrada")).toBeInTheDocument();
    expect(within(mic).getByLabelText("Ganho de entrada")).toHaveAttribute("max", "24");
    expect(within(mic).getByLabelText("Ouvir o microfone no fone enquanto grava")).toBeInTheDocument();
    // desempenho recolhido
    const perf = screen.getByText("Desempenho").closest("details")!;
    expect(perf).not.toHaveAttribute("open");
    expect(screen.getByRole("button", { name: "Cancelar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Aplicar" })).toBeInTheDocument();
  });

  it("calcula a latência com a taxa REAL do dispositivo (não 48 kHz fixo)", async () => {
    await api.projectCreate({ dir: "/tmp", name: "Taxa", sampleRate: 44100, bitDepth: 24 });
    await openDialog();
    expect(screen.getByText("Sistema de áudio: Simulado · 44,1 kHz")).toBeInTheDocument();
    // padrão do driver: latência pelo callback real (480 amostras no simulado)
    expect(screen.getByText("Buffer — ~21,8 ms de ida e volta a 44,1 kHz (driver: 480 amostras)")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Tamanho do buffer"), { target: { value: "256" } });
    expect(screen.getByText("Buffer — ~11,6 ms de ida e volta a 44,1 kHz")).toBeInTheDocument();
    expect(roundTripMs(256, 48000)).toBeCloseTo(10.67, 2);
    expect(roundTripMs(256, 96000)).toBeCloseTo(5.33, 2);
    expect(roundTripMs(256, 0)).toBeCloseTo(10.67, 2); // taxa desconhecida: 48 kHz
    expect(formatRate(48000)).toBe("48 kHz");
  });

  it("fone em outra taxa: mostra a conversão e mede a latência na taxa do fone (S-1)", async () => {
    // projeto a 48 kHz, fone Bluetooth que só abre a 44,1 kHz
    const base = spy.t;
    setTransport({
      ...base,
      invoke: async <T,>(cmd: string, args?: Record<string, unknown>) => {
        const r = await base.invoke<T>(cmd, args);
        return (cmd === "audio_status" ? { ...(r as object), deviceSampleRate: 44100 } : r) as T;
      },
    });
    await openDialog();
    expect(screen.getByText("Sistema de áudio: Simulado · 48 kHz · convertendo para 44,1 kHz")).toBeInTheDocument();
    // 480 quadros do driver a 44,1 kHz (não a 48 kHz)
    expect(screen.getByText("Buffer — ~21,8 ms de ida e volta a 44,1 kHz (driver: 480 amostras)")).toBeInTheDocument();
    expect(conversionNote(48000, 48000)).toBe("");
    expect(conversionNote(48000, null)).toBe("");
    expect(conversionNote(48000, 16000)).toBe(" · convertendo para 16 kHz");
  });

  it("buffer no padrão do driver: Aplicar não grava um tamanho fixo (AU-04)", async () => {
    await openDialog();
    expect((screen.getByLabelText("Tamanho do buffer") as HTMLSelectElement).value).toBe("0");
    fireEvent.click(screen.getByRole("button", { name: "Aplicar" }));
    await waitFor(() => expect(spy.calls.some((c) => c.cmd === "audio_configure")).toBe(true));
    const call = spy.calls.find((c) => c.cmd === "audio_configure")!;
    expect((call.args as { req: { bufferSize: number | null } }).req.bufferSize).toBeNull();
  });

  it("Testar som chama a engine e mostra o indicador", async () => {
    await openDialog();
    fireEvent.click(screen.getByRole("button", { name: "Testar som" }));
    await waitFor(() => expect(screen.getByRole("status", { name: "Tom de teste" })).toHaveTextContent("Tocando…"));
    expect(spy.calls.some((c) => c.cmd === "audio_test_tone")).toBe(true);
  });

  it("mudar o ganho chama a API e atualiza o estado compartilhado", async () => {
    await openDialog();
    fireEvent.change(screen.getByLabelText("Ganho de entrada"), { target: { value: "12" } });
    await waitFor(() => expect(useAudio.getState().status?.inputGainDb).toBe(12));
    expect(spy.calls.find((c) => c.cmd === "audio_input_gain_set")?.args).toEqual({ gainDb: 12 });
    expect(screen.getByText("+12 dB")).toBeInTheDocument();
  });

  it("medidores com faixa-alvo e aviso de sinal baixo quando o microfone está fraco", async () => {
    await api.audioConfigure({ outputId: null, inputId: "sim-in", bufferSize: 256 });
    await openDialog();
    act(() => useLive.setState({ meters: meters([0.01, 0.005]) })); // −40 dB
    const mic = screen.getByRole("region", { name: "Microfone / interface — o que você grava" });
    await waitFor(() => expect(within(mic).getByText(/Sinal baixo: fale no volume normal e suba o ganho/)).toBeInTheDocument());
    expect(within(mic).getByRole("meter", { name: "Nível da entrada 1" })).toHaveAttribute("data-zone", "low");
    act(() => useLive.setState({ meters: meters([0.25, 0.25]) })); // −12 dB: na faixa verde
    await waitFor(() => expect(within(mic).getByRole("meter", { name: "Nível da entrada 1" })).toHaveAttribute("data-zone", "target"));
    expect(within(mic).queryByText(/Sinal baixo/)).toBeNull();
    act(() => useLive.setState({ meters: meters([1, 1]) })); // 0 dB: distorcendo
    await waitFor(() => expect(within(mic).getAllByText("Muito alto").length).toBeGreaterThan(0));
  });

  it("ouvir o microfone no fone começa em 0 dB", async () => {
    await api.audioConfigure({ outputId: null, inputId: "sim-in", bufferSize: 256 });
    await openDialog();
    fireEvent.click(screen.getByLabelText("Ouvir o microfone no fone enquanto grava"));
    await waitFor(() => expect(spy.calls.some((c) => c.cmd === "audio_monitor_set")).toBe(true));
    expect(spy.calls.find((c) => c.cmd === "audio_monitor_set")?.args).toEqual({ req: { left: 0, right: null, gainDb: 0 } });
  });

  it("Cancelar fecha sem reconfigurar o dispositivo", async () => {
    await openDialog();
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(useAudioSettingsUi.getState().open).toBe(false);
    expect(spy.calls.some((c) => c.cmd === "audio_configure")).toBe(false);
  });
});
