import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { setTransport } from "../../ipc/transport";
import { createMockTransport } from "../../ipc/mock";
import { api } from "../../ipc/api";
import { useSession } from "../../core/session";
import { gridLines, PPQ } from "./grid";
import { TempoPanel } from "./TempoPanel";

const BAR_44 = PPQ * 4;
// troca para 3/4 no meio do compasso 2 (tempo 3): o compasso 2 fica cortado e o 3/4 começa o compasso 3
const CUT = BAR_44 + PPQ * 2;

const tempo = () => useSession.getState().session!.tempo;
/** Tick de cada compasso como a régua numera (fonte de verdade da numeração). */
const rulerTick = (bar: number) => gridLines(tempo(), 0, BAR_44 * 16).find((l) => l.isBar && l.bar === bar)!.tick;

describe("TempoPanel — numeração de compassos igual à régua", () => {
  beforeEach(async () => {
    setTransport(createMockTransport());
    useSession.getState().setSession(await api.projectCreate({ dir: "/tmp", name: "Mapa", sampleRate: 48000, bitDepth: 24 }));
    useSession.getState().setSession(await api.meterSet(CUT, 3, 4));
  });

  it("mostra a mudança de fórmula no compasso da régua (3), não em compasso fracionário", () => {
    render(<TempoPanel />);
    expect(rulerTick(3)).toBe(CUT);
    const meterTable = screen.getAllByRole("table")[1]!;
    const cells = within(meterTable).getAllByRole("row").slice(1).map((r) => within(r).getAllByRole("cell")[0]!.textContent);
    expect(cells).toEqual(["1", "3"]);
    expect(screen.getByRole("button", { name: "Remover fórmula no compasso 3" })).toBeInTheDocument();
  });

  it("andamento digitado no compasso 4 cai no início do compasso 4 da régua", async () => {
    render(<TempoPanel />);
    fireEvent.change(screen.getAllByLabelText("Compasso")[0]!, { target: { value: "4" } });
    fireEvent.change(screen.getByLabelText("BPM"), { target: { value: "90" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Definir" })[0]!);
    await waitFor(() => expect(tempo().tempos).toHaveLength(2));
    expect(tempo().tempos[1]).toEqual({ tick: rulerTick(4), bpm: 90 });
    expect(rulerTick(4)).toBe(CUT + PPQ * 3);
    const tempoTable = screen.getAllByRole("table")[0]!;
    expect(within(within(tempoTable).getAllByRole("row")[2]!).getAllByRole("cell")[0]!.textContent).toBe("4");
  });

  it("andamento fora do início do compasso (ex.: importado) mostra compasso.tempo.tique da régua", async () => {
    useSession.getState().setSession(await api.tempoSet(CUT + PPQ + 10, 100));
    render(<TempoPanel />);
    const tempoTable = screen.getAllByRole("table")[0]!;
    expect(within(within(tempoTable).getAllByRole("row")[2]!).getAllByRole("cell")[0]!.textContent).toBe("003.2.010");
  });

  it("fórmula digitada no compasso 5 cai no início do compasso 5 da régua", async () => {
    render(<TempoPanel />);
    fireEvent.change(screen.getAllByLabelText("Compasso")[1]!, { target: { value: "5" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Definir" })[1]!);
    await waitFor(() => expect(tempo().meters).toHaveLength(3));
    expect(tempo().meters[2]!.tick).toBe(CUT + PPQ * 6);
  });
});
