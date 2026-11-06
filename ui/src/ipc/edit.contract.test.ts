// Contrato IPC da Fase 3 (edição): fixtures geradas por crates/app-core/tests/contract_fixtures.rs.
import * as S from "./schema";
import editSession from "./__fixtures__/edit-session.json";
import editRequests from "./__fixtures__/edit-requests.json";
import { encodeSpectrogram, parseSpectrogram, SpectrogramFormatError } from "./spectrogram";

describe("contrato IPC — edição (Fase 3)", () => {
  it("SessionView com clipes divididos, fades, polaridade e reverso", () => {
    const v = S.SessionView.parse(editSession);
    const [a, b] = v.tracks[0]!.clips;
    expect(a).toMatchObject({ fadeInFrames: 480, fadeInShape: "equalPower", fadeOutFrames: 0, polarityInverted: true, reversed: false, lengthFrames: 24000, sourceFrames: 48000 });
    expect(b).toMatchObject({ fadeInFrames: 0, fadeOutFrames: 960, reversed: true, offsetFrames: 24000, startFrame: 24000 });
    expect(a!.gain).toBeGreaterThan(9);
    expect(v.history.undoLabel).toBe("Ganho do clipe");
  });
  it("pedidos que a UI envia são os que o Rust produz", () => {
    expect(S.FadeRequest.parse(editRequests.fades).fadeOutFrames).toBe(960);
    expect(S.FadeShape.array().parse(editRequests.shapes)).toEqual(["linear", "equalPower"]);
    expect(S.SpectrogramRequest.parse(editRequests.spectrogram)).toEqual({ fftSize: 2048, hop: 512, bins: 128 });
  });
});

describe("espectrograma binário (AMSG)", () => {
  it("ida e volta do cabeçalho e dos dados", () => {
    const data = Uint8Array.from({ length: 3 * 16 }, (_, i) => i);
    const s = parseSpectrogram(encodeSpectrogram({ bins: 16, fftSize: 1024, hop: 256, sampleRate: 48000, totalFrames: 700, data }));
    expect(s).toMatchObject({ bins: 16, fftSize: 1024, hop: 256, sampleRate: 48000, columns: 3, totalFrames: 700 });
    expect(Array.from(s.data)).toEqual(Array.from(data));
  });
  it("rejeita buffers truncados ou de outro formato", () => {
    expect(() => parseSpectrogram(new ArrayBuffer(10))).toThrow(SpectrogramFormatError);
    const ok = encodeSpectrogram({ bins: 16, fftSize: 1024, hop: 256, sampleRate: 48000, totalFrames: 10, data: new Uint8Array(16) });
    new DataView(ok).setUint8(0, 88);
    expect(() => parseSpectrogram(ok)).toThrow(SpectrogramFormatError);
    const short = encodeSpectrogram({ bins: 16, fftSize: 1024, hop: 256, sampleRate: 48000, totalFrames: 10, data: new Uint8Array(16) });
    expect(() => parseSpectrogram(short.slice(0, short.byteLength - 1))).toThrow(SpectrogramFormatError);
  });
});
