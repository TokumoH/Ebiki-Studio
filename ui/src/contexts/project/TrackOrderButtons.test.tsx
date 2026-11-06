import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import { TrackOrderButtons } from "./TrackOrderButtons";

const names = () => useSession.getState().session?.tracks.map((t) => t.name);

describe("TrackOrderButtons", () => {
  beforeEach(async () => {
    setTransport(createMockTransport());
    useSession.getState().setSession(await api.projectCreate({ dir: "/tmp", name: "Ordem", sampleRate: 48000, bitDepth: 24 }));
    for (const n of ["A", "B", "C"]) useSession.getState().setSession(await api.trackCreate(n));
  });

  function Headers() {
    const tracks = useSession((s) => s.session?.tracks ?? []);
    return (
      <>
        {tracks.map((t, i) => (
          <TrackOrderButtons key={t.id} trackId={t.id} name={t.name} index={i} />
        ))}
      </>
    );
  }

  it("primeira não sobe e última não desce", () => {
    render(<Headers />);
    expect(screen.getByRole("button", { name: "Mover A para cima" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Mover C para baixo" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Mover B para cima" })).toBeEnabled();
  });

  it("move para cima e para baixo", async () => {
    render(<Headers />);
    fireEvent.click(screen.getByRole("button", { name: "Mover C para cima" }));
    await waitFor(() => expect(names()).toEqual(["A", "C", "B"]));
    fireEvent.click(screen.getByRole("button", { name: "Mover A para baixo" }));
    await waitFor(() => expect(names()).toEqual(["C", "A", "B"]));
  });
});
