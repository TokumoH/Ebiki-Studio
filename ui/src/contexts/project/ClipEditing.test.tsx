import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ClipContextMenu, clipMenuItems } from "./ClipContextMenu";
import { ClipInspector } from "./ClipInspector";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import type { Clip } from "../../ipc/types";

const base = { canSplit: true, hasOverlap: false, split: vi.fn(), normalize: vi.fn(), polarity: vi.fn(), reverse: vi.fn(), crossfade: vi.fn(), remove: vi.fn() };
const clip = { polarityInverted: true, reversed: false } as Clip;

describe("menu de contexto do clipe", () => {
  it("itens com estado e desabilitados conforme o contexto", () => {
    const items = clipMenuItems(clip, base);
    expect(items.map((i) => i.label)).toEqual(["Dividir no cursor", "Normalizar (−1 dBFS)", "Inverter fase", "Reverter", "Crossfade com clipe sobreposto", "Remover clipe"]);
    expect(items.find((i) => i.id === "polarity")?.checked).toBe(true);
    expect(items.find((i) => i.id === "crossfade")?.disabled).toBe(true);
    items.find((i) => i.id === "polarity")!.run();
    expect(base.polarity).toHaveBeenCalledWith(false);
  });

  it("navegação por teclado: setas pulam itens desabilitados, Enter executa, Esc fecha", () => {
    const onClose = vi.fn();
    const items = clipMenuItems(clip, { ...base, canSplit: false });
    render(<ClipContextMenu x={0} y={0} items={items} label="Ações" onClose={onClose} />);
    const menu = screen.getByRole("menu", { name: "Ações" });
    expect(document.activeElement?.textContent).toContain("Normalizar");
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(document.activeElement?.textContent).toContain("Remover clipe");
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(document.activeElement).toHaveAttribute("role", "menuitemcheckbox");
    fireEvent.keyDown(document.activeElement!, { key: "Enter" });
    expect(base.polarity).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});

describe("inspetor do clipe", () => {
  beforeEach(() => {
    setTransport(createMockTransport());
    useSession.setState({ session: null, selectedClipId: null, selectedTrackId: null });
  });

  it("sem seleção mostra orientação; com clipe edita fades, fase e ganho", async () => {
    render(<ClipInspector />);
    expect(screen.getByText("Nenhum clipe selecionado")).toBeInTheDocument();
    await act(async () => {
      await api.projectCreate({ dir: "/tmp", name: "Insp", sampleRate: 48000, bitDepth: 24 });
      const v = await api.audioImport(["/x/voz.wav"], null, 0);
      useSession.getState().setSession(v);
      useSession.getState().select({ clipId: v.tracks[0]!.clips[0]!.id });
    });
    const fadeIn = screen.getByRole("textbox", { name: "Fade de entrada (ms)" });
    fireEvent.change(fadeIn, { target: { value: "250" } });
    fireEvent.keyDown(fadeIn, { key: "Enter" });
    await waitFor(() => expect(useSession.getState().session?.tracks[0]?.clips[0]?.fadeInFrames).toBe(12_000));
    fireEvent.click(screen.getByRole("button", { name: "Inverter fase" }));
    await waitFor(() => expect(useSession.getState().session?.tracks[0]?.clips[0]?.polarityInverted).toBe(true));
    expect(screen.getByRole("button", { name: "Inverter fase" })).toHaveAttribute("aria-pressed", "true");
    fireEvent.change(screen.getByRole("slider", { name: "Ganho do clipe" }), { target: { value: "-6" } });
    await waitFor(() => expect(useSession.getState().session?.tracks[0]?.clips[0]?.gain).toBe(-6));
    expect(screen.getByRole("button", { name: "Crossfade" })).toBeDisabled();
  });
});
