import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { withCompositionMock } from "../../ipc/mockComposition";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import { useComposition } from "../../core/composition";
import { HarmonyPanel } from "./HarmonyPanel";

describe("HarmonyPanel", () => {
  beforeEach(async () => {
    setTransport(withCompositionMock(createMockTransport()));
    useComposition.setState({ key: { tonic: 0, mode: "ionian", isMinor: false } });
    useSession.getState().setSession(await api.projectCreate({ dir: "/tmp", name: "Harmonia", sampleRate: 48000, bitDepth: 24 }));
  });

  it("mostra o campo, alterna para graus/tétrades e troca o modo", async () => {
    render(<HarmonyPanel />);
    const field = await screen.findByRole("list", { name: /Campo harmônico/ });
    await waitFor(() => expect(within(field).getAllByRole("listitem")).toHaveLength(7));
    expect(within(field).getByText("Bdim")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: "Tétrades" }));
    expect(await within(field).findByText("G7")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: "Graus" }));
    expect(within(field).getAllByText("V7")[0]).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Modo" }).querySelectorAll("option")).toHaveLength(21));
    fireEvent.change(screen.getByRole("combobox", { name: "Modo" }), { target: { value: "aeolian" } });
    await waitFor(() => expect(useComposition.getState().key).toEqual({ tonic: 0, mode: "aeolian", isMinor: true }));
  });

  it("filtra por gênero e insere no arranjo (desfazer remove)", async () => {
    render(<HarmonyPanel />);
    await screen.findByRole("list", { name: "Progressões sugeridas" });
    await waitFor(() => expect(screen.getByRole("combobox", { name: "Gênero" }).querySelectorAll("option").length).toBe(16));
    fireEvent.change(screen.getByRole("combobox", { name: "Gênero" }), { target: { value: "samba" } });
    const card = await screen.findByRole("listitem", { name: /Progressão Samba/ });
    expect(within(card).getByLabelText("Acordes")).toHaveTextContent("CA7Dm7G7");
    fireEvent.click(within(card).getByRole("button", { name: /no arranjo/ }));
    await waitFor(() => expect(useSession.getState().session?.tracks).toHaveLength(1));
    expect(useSession.getState().selectedTrackId).toBe(useSession.getState().session?.tracks[0]?.id);
    useSession.getState().setSession(await api.undo());
    expect(useSession.getState().session?.tracks).toHaveLength(0);
  });

  it("monta sequência com o próximo acorde (após G7 vem Dó)", async () => {
    render(<HarmonyPanel />);
    fireEvent.click(screen.getByRole("tab", { name: "Próximo acorde" }));
    const list = await screen.findByRole("list", { name: "Próximos acordes sugeridos" });
    fireEvent.click(await within(list).findByRole("button", { name: /^Adicionar G / }));
    await screen.findByText("Sugestões após G");
    const first = await within(screen.getByRole("list", { name: "Próximos acordes sugeridos" })).findAllByRole("button");
    await waitFor(() => expect(first[0]).toHaveAccessibleName(/^Adicionar C — Cadência autêntica/));
  });
});
