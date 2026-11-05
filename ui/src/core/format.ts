// Formatação de valores musicais e de áudio para exibição.
export const PPQ = 960;

export function formatDb(db: number): string {
  if (db <= -144) return "−∞";
  const s = db.toFixed(1);
  return db > 0 ? `+${s}` : s.replace("-", "−");
}

export function formatTime(seconds: number): string {
  const s = Math.max(0, seconds);
  const m = Math.floor(s / 60);
  const rest = s - m * 60;
  return `${String(m).padStart(2, "0")}:${rest.toFixed(3).padStart(6, "0")}`;
}

export function formatBarBeat(bb: { bar: number; beat: number; tickInBeat: number }): string {
  return `${String(bb.bar).padStart(3, "0")}.${bb.beat}.${String(bb.tickInBeat).padStart(3, "0")}`;
}

export function formatPan(p: number): string {
  if (Math.abs(p) < 0.005) return "C";
  return `${p < 0 ? "E" : "D"}${Math.round(Math.abs(p) * 100)}`;
}
