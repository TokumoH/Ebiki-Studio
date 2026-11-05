import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { withCompositionMock } from "../../ipc/mockComposition";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import { useComposition } from "../../core/composition";
import { BasesPanel } from "./BasesPanel";

describe("BasesPanel", () => {
  beforeEach(async () => {
    setTransport(withCompositionMock(createMockTransport()));
    useComposition.setState({ key: { tonic: 0, mode: "ionian", isMinor: false } });
    useSession.getState().setSession(await api.projectCreate({ dir: "/tmp", name: "Bases", sampleRate: 48000, bitDepth: 24 }));
  });

  it("busca por texto e tipo", async () => {
    render(<BasesPanel />);
    const table = await screen.findByRole("table", { name: "Resultados de bases" });
    await waitFor(() => expect(within(table).getAllByRole("row")).toHaveLength(5));
    fireEvent.change(screen.getByLabelText("Buscar bases"), { target: { value: "loop" } });
    await waitFor(() => expect(within(table).getAllByRole("row")).toHaveLength(2));
    fireEvent.change(screen.getByLabelText("Filtrar por tipo"), { target: { value: "generated" } });
    expect(await screen.findByText("Nenhuma base encontrada.")).toBeInTheDocument();
  });

  it("inserir com divergência pergunta e adapta", async () => {
    render(<BasesPanel />);
    const row = await screen.findByRole("row", { name: /Base Loop de bateria/ });
    fireEvent.click(within(row).getByRole("button", { name: /^Inserir/ }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("A base diverge do projeto:")).toBeInTheDocument();
    expect(within(dialog).getByText(/base 96 BPM → projeto 120 BPM/)).toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: "Inserir" })).toBeNull();
    fireEvent.click(within(dialog).getByRole("button", { name: "Adaptar ao projeto" }));
    await waitFor(() => expect(useSession.getState().session?.tracks).toHaveLength(1));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("remove com desfazer da biblioteca e mostra o SoundFont", async () => {
    render(<BasesPanel />);
    const row = await screen.findByRole("row", { name: /Base Valsa em Sol/ });
    fireEvent.click(within(row).getByRole("button", { name: /^Remover/ }));
    await waitFor(() => expect(screen.queryByRole("row", { name: /Base Valsa em Sol/ })).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "Desfazer na biblioteca" }));
    expect(await screen.findByRole("row", { name: /Base Valsa em Sol/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "SoundFont…" }));
    expect(await screen.findByText("Sintetizador simples (embutido)")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Escolher arquivo .sf2…" }));
    expect(await screen.findByText("SoundFont: Banco simulado · 128 presets")).toBeInTheDocument();
  });
});
