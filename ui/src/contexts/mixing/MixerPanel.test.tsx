import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import { MixerPanel } from "./MixerPanel";

describe("MixerPanel — bus", () => {
  beforeEach(async () => {
    setTransport(createMockTransport());
    useSession.getState().setSession(await api.projectCreate({ dir: "/tmp", name: "Mixer", sampleRate: 48000, bitDepth: 24 }));
    useSession.getState().setSession(await api.trackCreate("Voz"));
    const v = await api.busCreate("Bus 1");
    const bus = v.buses[0]!.id;
    const track = v.tracks[0]!.id;
    useSession.getState().setSession(await api.mixerEdit({ kind: "track", id: track }, { op: "output", value: { kind: "bus", id: bus } }));
  });

  it("renomeia o bus clicando no nome", async () => {
    render(<MixerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Renomear Bus 1" }));
    const input = screen.getByRole("textbox", { name: "Novo nome de Bus 1" });
    fireEvent.change(input, { target: { value: "Reverb" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(useSession.getState().session?.buses[0]?.name).toBe("Reverb"));
    expect(await screen.findByRole("button", { name: "Renomear Reverb" })).toBeInTheDocument();
  });

  it("remove o bus só depois de confirmar; a faixa volta ao Master", async () => {
    render(<MixerPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Remover Bus 1" }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(/voltam para o Master/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(useSession.getState().session?.buses).toHaveLength(1);

    fireEvent.click(screen.getByRole("button", { name: "Remover Bus 1" }));
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Remover" }));
    await waitFor(() => expect(useSession.getState().session?.buses).toHaveLength(0));
    expect(useSession.getState().session?.tracks[0]?.strip.output).toEqual({ kind: "master" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("canais de faixa e Master não oferecem renomear/remover", () => {
    render(<MixerPanel />);
    expect(screen.queryByRole("button", { name: "Remover Voz" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Renomear Master" })).not.toBeInTheDocument();
  });
});
