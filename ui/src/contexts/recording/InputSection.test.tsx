import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { api } from "../../ipc/api";
import { useAudio, useAudioSettingsUi } from "../../core/audio";
import { useLive } from "../../core/live";
import { DEFAULT_MONITOR_GAIN_DB, formatGainDb, InputSection } from "./InputSection";

describe("InputSection — Microfone (entrada)", () => {
  beforeEach(() => {
    setTransport(createMockTransport());
    window.localStorage.clear();
    useLive.setState({ meters: null, transport: null });
    useAudioSettingsUi.getState().setOpen(false);
  });

  const withMic = async (gainDb = 0) => {
    await api.audioConfigure({ outputId: null, inputId: "sim-in", bufferSize: 256 });
    useAudio.getState().setStatus(await api.audioInputGainSet(gainDb));
  };

  it("usa o nome Microfone (entrada) e mostra microfone e ganho só para leitura", async () => {
    await withMic(12);
    render(<InputSection />);
    const section = screen.getByRole("region", { name: "Microfone (entrada)" });
    expect(within(section).getByRole("button", { name: /Microfone \(entrada\)/ })).toHaveAttribute("aria-expanded", "true");
    expect(within(section).getByText("Entrada simulada")).toBeInTheDocument();
    expect(within(section).getByText("+12 dB")).toBeInTheDocument();
    // o microfone é escolhido num lugar só: nada de seletor de dispositivo nem de ganho aqui
    expect(within(section).queryByRole("combobox", { name: /Dispositivo/ })).toBeNull();
    expect(within(section).queryByLabelText("Ganho de entrada")).toBeNull();
    expect(screen.queryByText(/Entrada de áudio/)).toBeNull();
  });

  it("o botão Configurar microfone e fone… abre as Configurações de áudio", async () => {
    await withMic();
    render(<InputSection />);
    fireEvent.click(screen.getByRole("button", { name: "Configurar microfone e fone…" }));
    expect(useAudioSettingsUi.getState().open).toBe(true);
  });

  it("ouvir o microfone no fone começa em 0 dB", async () => {
    expect(DEFAULT_MONITOR_GAIN_DB).toBe(0);
    await withMic();
    render(<InputSection />);
    expect(screen.getByLabelText("Volume de monitoração")).toHaveValue("0");
    fireEvent.click(screen.getByRole("button", { name: "Ouvir o microfone no fone" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Ouvir o microfone no fone" })).toHaveAttribute("aria-pressed", "true"));
    expect((await api.audioMonitorGet())?.gainDb).toBe(0);
  });

  it("ao fechar as Configurações de áudio, relê o retorno ligado lá", async () => {
    await withMic();
    render(<InputSection />);
    const ouvir = screen.getByRole("button", { name: "Ouvir o microfone no fone" });
    expect(ouvir).toHaveAttribute("aria-pressed", "false");
    act(() => useAudioSettingsUi.getState().setOpen(true));
    await api.audioMonitorSet({ left: 0, right: null, gainDb: 0 }); // ligado no diálogo
    act(() => useAudioSettingsUi.getState().setOpen(false));
    await waitFor(() => expect(ouvir).toHaveAttribute("aria-pressed", "true"));
  });

  it("medidores com a mesma faixa-alvo verde e o aviso de sinal baixo", async () => {
    await withMic();
    render(<InputSection />);
    act(() => useLive.setState({ meters: { master: [0, 0], tracks: [], buses: [], inputs: [0.01, 0.01], xruns: 0, recordOverflow: 0 } }));
    await waitFor(() => expect(screen.getByText(/Sinal baixo/)).toBeInTheDocument());
    expect(screen.getByRole("meter", { name: "Nível da entrada 1" }).querySelector(".input-meter-target")).not.toBeNull();
  });

  it("sem microfone, orienta a abrir as configurações", async () => {
    useAudio.getState().setStatus(await api.audioStatus());
    render(<InputSection />);
    expect(screen.getByText(/Escolha o microfone em “Configurar microfone e fone…”/)).toBeInTheDocument();
    expect(formatGainDb(0)).toBe("0 dB");
    expect(formatGainDb(6.5)).toBe("+6,5 dB");
  });
});
