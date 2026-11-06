import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { setTransport, type Transport } from "../ipc/transport";
import { createMockTransport } from "../ipc/mock";
import { api } from "../ipc/api";
import { useSession } from "../core/session";
import { CloseProjectDialog } from "./CloseProjectDialog";

/** Transporte simulado em que salvar pode falhar sob demanda. */
function failingSave(): { t: Transport; failSave: (v: boolean) => void } {
  const base = createMockTransport();
  let fail = false;
  const t: Transport = {
    ...base,
    kind: base.kind,
    invoke: <T,>(cmd: string, args?: Record<string, unknown>) =>
      fail && cmd === "project_save" ? Promise.reject({ code: "persistence.io", message: "disco cheio" }) : base.invoke<T>(cmd, args),
    invokeBinary: base.invokeBinary.bind(base),
    listen: base.listen.bind(base),
    pickOpen: base.pickOpen.bind(base),
    pickSave: base.pickSave.bind(base),
  };
  return { t, failSave: (v) => (fail = v) };
}

describe("CloseProjectDialog", () => {
  let ctl: ReturnType<typeof failingSave>;
  beforeEach(async () => {
    ctl = failingSave();
    setTransport(ctl.t);
    useSession.getState().setSession(await api.projectCreate({ dir: "/tmp", name: "Fechar", sampleRate: 48000, bitDepth: 24 }));
    useSession.getState().setSession(await api.trackCreate("Voz"));
  });

  it("não fecha o diálogo nem o projeto quando salvar falha", async () => {
    ctl.failSave(true);
    const onClose = vi.fn();
    render(<CloseProjectDialog open onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Salvar e fechar" }));
    await waitFor(() => expect(useSession.getState().busy).toBe(false));
    await new Promise((r) => setTimeout(r, 20));
    expect(onClose).not.toHaveBeenCalled();
    expect(useSession.getState().session?.project.name).toBe("Fechar");
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("salva, fecha o projeto e o diálogo quando dá certo", async () => {
    const onClose = vi.fn();
    render(<CloseProjectDialog open onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Salvar e fechar" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    expect(useSession.getState().session).toBeNull();
  });

  it("fechar sem salvar descarta e fecha", async () => {
    const onClose = vi.fn();
    render(<CloseProjectDialog open onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Fechar sem salvar" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    expect(useSession.getState().session).toBeNull();
  });
});
