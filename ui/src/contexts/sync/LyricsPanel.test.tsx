import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import { LyricsPanel } from "./LyricsPanel";

describe("LyricsPanel", () => {
  beforeEach(async () => {
    setTransport(createMockTransport());
    const v = await api.projectCreate({ dir: "/tmp", name: "Letra", sampleRate: 48000, bitDepth: 24 });
    act(() => useSession.getState().setSession(v));
  });

  it("aplica o texto, lista as unidades e define o tempo da selecionada", async () => {
    render(<LyricsPanel />);
    fireEvent.change(screen.getByLabelText("Texto da letra"), { target: { value: "Can-to a-qui" } });
    fireEvent.click(screen.getByRole("button", { name: "Aplicar texto" }));
    const list = await screen.findByRole("list", { name: "Unidades da letra" });
    await waitFor(() => expect(list.querySelectorAll("button.lyric-unit")).toHaveLength(2));
    expect(list).toHaveTextContent("Can-to");
    fireEvent.click(screen.getByRole("button", { name: /^a-qui/ }));
    fireEvent.change(screen.getByLabelText("Início em tempos"), { target: { value: "4" } });
    fireEvent.change(screen.getByLabelText("Fim em tempos"), { target: { value: "6" } });
    fireEvent.click(screen.getByRole("button", { name: "Definir" }));
    await waitFor(() => expect(screen.getByRole("button", { name: /^a-qui/ })).toHaveTextContent("2.1"));
    expect(useSession.getState().session?.lyrics.lines[0]?.words[1]?.syllables[1]?.span?.end).toBe(6 * 960);
  });
});
