import { render, screen, fireEvent } from "@testing-library/react";
import { dbToFader, faderToDb, Fader, Segmented, Dialog } from "./components";

describe("curva do fader", () => {
  it("é monotônica e inversível", () => {
    let prev = -1;
    for (let db = -144; db <= 12; db += 0.5) {
      const p = dbToFader(db);
      expect(p).toBeGreaterThanOrEqual(prev);
      prev = p;
      expect(Math.abs(faderToDb(p) - db)).toBeLessThan(0.05);
    }
    expect(dbToFader(0)).toBeCloseTo(0.8, 5);
  });
});

describe("Fader", () => {
  it("teclado ajusta em passos de 1 dB e duplo clique volta a 0 dB", () => {
    const calls: number[] = [];
    render(<Fader db={-6} label="Volume" onChange={(v) => calls.push(v)} />);
    const s = screen.getByRole("slider", { name: "Volume" });
    fireEvent.keyDown(s, { key: "ArrowUp" });
    fireEvent.keyDown(s, { key: "ArrowDown", shiftKey: true });
    fireEvent.doubleClick(s);
    expect(calls).toEqual([-5, -5.1, 0]);
    expect(s).toHaveAttribute("aria-valuetext", "0.0 dB");
  });
});

describe("Fader otimista", () => {
  it("teclas rápidas acumulam antes da confirmação do núcleo", () => {
    const calls: number[] = [];
    render(<Fader db={0} label="Rápido" onChange={(v) => calls.push(v)} />);
    const s = screen.getByRole("slider", { name: "Rápido" });
    for (let i = 0; i < 3; i++) fireEvent.keyDown(s, { key: "ArrowDown" });
    expect(calls).toEqual([-1, -2, -3]);
  });
});

describe("Segmented e Dialog", () => {
  it("segmentado marca a opção e o diálogo fecha com Esc", () => {
    const onChange = vi.fn();
    const onClose = vi.fn();
    render(
      <>
        <Segmented label="Taxa" value={48000} options={[{ value: 44100, label: "44,1" }, { value: 48000, label: "48" }]} onChange={onChange} />
        <Dialog title="Teste" open onClose={onClose}>
          <input aria-label="x" />
        </Dialog>
      </>,
    );
    expect(screen.getByRole("radio", { name: "48" })).toHaveAttribute("aria-checked", "true");
    fireEvent.click(screen.getByRole("radio", { name: "44,1" }));
    expect(onChange).toHaveBeenCalledWith(44100);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });
});
