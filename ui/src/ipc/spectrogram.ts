// Decodificação do espectrograma binário `AMSG` (gerado por crates/infra-codecs/src/spectrogram.rs).
// Cabeçalho little-endian de 40 bytes; depois `columns × bins` bytes (por coluna, grave → agudo).

export interface Spectrogram {
  bins: number;
  fftSize: number;
  hop: number;
  sampleRate: number;
  columns: number;
  totalFrames: number;
  /** 0 (≤ −100 dBFS) a 255 (0 dBFS). */
  data: Uint8Array;
}

export const SPECTROGRAM_HEADER = 40;

export class SpectrogramFormatError extends Error {}

export function parseSpectrogram(buf: ArrayBuffer): Spectrogram {
  if (buf.byteLength < SPECTROGRAM_HEADER) throw new SpectrogramFormatError("espectrograma truncado");
  const dv = new DataView(buf);
  const magic = String.fromCharCode(dv.getUint8(0), dv.getUint8(1), dv.getUint8(2), dv.getUint8(3));
  if (magic !== "AMSG" || dv.getUint16(4, true) !== 1) throw new SpectrogramFormatError("formato de espectrograma desconhecido");
  const bins = dv.getUint16(6, true);
  const columns = dv.getUint32(20, true);
  const data = new Uint8Array(buf, SPECTROGRAM_HEADER);
  if (bins === 0 || data.length !== columns * bins) throw new SpectrogramFormatError("tamanho do espectrograma inconsistente");
  return {
    bins,
    fftSize: dv.getUint32(8, true),
    hop: dv.getUint32(12, true),
    sampleRate: dv.getUint32(16, true),
    columns,
    totalFrames: Number(dv.getBigUint64(24, true)),
    data,
  };
}

/** Monta um buffer `AMSG` (usado pelo transporte simulado e por testes). Checksum não é verificado pela UI. */
export function encodeSpectrogram(s: Omit<Spectrogram, "columns">): ArrayBuffer {
  const columns = Math.floor(s.data.length / s.bins);
  const out = new ArrayBuffer(SPECTROGRAM_HEADER + s.data.length);
  const dv = new DataView(out);
  "AMSG".split("").forEach((c, i) => dv.setUint8(i, c.charCodeAt(0)));
  dv.setUint16(4, 1, true);
  dv.setUint16(6, s.bins, true);
  dv.setUint32(8, s.fftSize, true);
  dv.setUint32(12, s.hop, true);
  dv.setUint32(16, s.sampleRate, true);
  dv.setUint32(20, columns, true);
  dv.setBigUint64(24, BigInt(s.totalFrames), true);
  new Uint8Array(out, SPECTROGRAM_HEADER).set(s.data);
  return out;
}
