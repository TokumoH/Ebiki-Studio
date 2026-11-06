import { useCallback, useEffect, useRef, useState, type PointerEvent } from "react";
import type { Cursor } from "./scoreEditing";
import { snapHalfSpaces } from "./scoreEditing";
import { alphaTabSettings, loadAlphaTab } from "./alphaTab";

type AtApi = InstanceType<Awaited<ReturnType<typeof loadAlphaTab>>["AlphaTabApi"]>;

/** Posição apontada na pauta no modo Inserir (o painel converte em nota pela clave e armadura). */
export interface StaffPick {
  part: number;
  staff: number;
  measure: number;
  /** Batida sob o ponteiro (índice na voz desenhada), se houver. */
  beat: number | null;
  /** Meias-distâncias abaixo da linha de cima (0 = linha de cima). */
  halfSpaces: number;
}

export interface InsertMode {
  /** Texto do balão ("Dó 5 · semínima"); `null` = posição não aceita. */
  describe: (p: StaffPick) => string | null;
  onPick: (p: StaffPick) => void;
}

interface Line {
  x: number;
  y: number;
  w: number;
}

interface Hover {
  pick: StaffPick | null;
  x: number;
  y: number;
  spacing: number;
  bar: { x: number; y: number; w: number; h: number };
  hint: string | null;
}

/** Linhas de pauta desenhadas pelo alphaTab: retângulos finos e largos (coordenadas do host). */
function collectLines(host: HTMLElement): Line[] {
  const hr = host.getBoundingClientRect();
  const out: Line[] = [];
  host.querySelectorAll("svg rect").forEach((el) => {
    const b = el.getBoundingClientRect();
    if (b.height <= 2.5 && b.width > 20) out.push({ x: b.x - hr.x, y: b.y - hr.y + b.height / 2, w: b.width });
  });
  return out;
}

/**
 * Grupo de linhas igualmente espaçadas mais próximo de `y` dentro da faixa vertical [top, bottom]
 * que passa por `x`: 5 linhas = pauta, 6 = tablatura.
 */
export function staffLinesAt(lines: Line[], x: number, y: number, top: number, bottom: number): { top: number; spacing: number; count: number } | null {
  const ys = lines
    .filter((l) => x >= l.x - 2 && x <= l.x + l.w + 2 && l.y >= top - 1 && l.y <= bottom + 1)
    .map((l) => l.y)
    .sort((a, b) => a - b)
    .filter((v, i, a) => i === 0 || v - a[i - 1]! > 0.5);
  const groups: number[][] = [];
  for (const v of ys) {
    const g = groups[groups.length - 1];
    if (g && g.length >= 2) {
      const sp = g[1]! - g[0]!;
      if (Math.abs(v - g[g.length - 1]! - sp) <= sp * 0.2) {
        g.push(v);
        continue;
      }
    } else if (g && g.length === 1 && v - g[0]! < 30) {
      g.push(v);
      continue;
    }
    groups.push([v]);
  }
  let best: { top: number; spacing: number; count: number } | null = null;
  let bestDist = Infinity;
  for (const g of groups) {
    if (g.length < 5) continue;
    const t = g[0]!;
    const b = g[g.length - 1]!;
    const d = y < t ? t - y : y > b ? y - b : 0;
    if (d < bestDist) {
      bestDist = d;
      best = { top: t, spacing: (b - t) / (g.length - 1), count: g.length };
    }
  }
  return best;
}

/** Item cuja faixa vertical está mais perto de `y` (0 = dentro), até `max` px; senão `null`. */
export function nearestBy<T>(items: readonly T[], bounds: (t: T) => { y: number; h: number }, y: number, max: number): T | null {
  let best: T | null = null;
  let bestDist = Infinity;
  for (const it of items) {
    const b = bounds(it);
    const d = y < b.y ? b.y - y : y > b.y + b.h ? y - b.y - b.h : 0;
    if (d < bestDist) {
      best = it;
      bestDist = d;
    }
  }
  return bestDist <= max ? best : null;
}

/**
 * Renderização da partitura (pauta + tablatura) pelo alphaTab a partir do MusicXML do núcleo.
 * O cursor de eventos é desenhado por cima, a partir dos limites de batida do alphaTab.
 * No modo Inserir, mostra a nota fantasma sob o ponteiro e informa o clique.
 */
export function ScoreView({ xml, cursor, staffOf, voiceInStaff, insert = null }: { xml: string; cursor: Cursor | null; staffOf: number; voiceInStaff: number; insert?: InsertMode | null }) {
  const host = useRef<HTMLDivElement>(null);
  const apiRef = useRef<AtApi | null>(null);
  const lines = useRef<Line[]>([]);
  const [ready, setReady] = useState(false);
  const [rendered, setRendered] = useState(0);
  const [failed, setFailed] = useState<string | null>(null);
  const [box, setBox] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const [hover, setHover] = useState<Hover | null>(null);

  useEffect(() => {
    let disposed = false;
    void loadAlphaTab()
      .then((at) => {
        if (disposed || !host.current) return;
        const api = new at.AlphaTabApi(host.current, alphaTabSettings());
        api.postRenderFinished.on(() => setRendered((n) => n + 1));
        api.error.on((e) => setFailed(e.message));
        apiRef.current = api;
        setReady(true);
      })
      .catch((e: unknown) => setFailed(String(e)));
    return () => {
      disposed = true;
      apiRef.current?.destroy();
      apiRef.current = null;
    };
  }, []);

  useEffect(() => {
    const api = apiRef.current;
    if (!api || !xml) return;
    setFailed(null);
    try {
      api.load(new TextEncoder().encode(xml));
    } catch (e) {
      setFailed(e instanceof Error ? e.message : String(e));
    }
  }, [xml, ready]);

  useEffect(() => {
    const api = apiRef.current;
    if (!api?.score || !cursor || !api.boundsLookup) return setBox(null);
    const beat = api.score.tracks[cursor.part]?.staves[staffOf]?.bars[cursor.measure]?.voices[voiceInStaff]?.beats[cursor.index];
    const b = beat ? api.boundsLookup.findBeat(beat) : null;
    setBox(b ? { x: b.visualBounds.x, y: b.realBounds.y, w: Math.max(8, b.visualBounds.w), h: b.realBounds.h } : null);
  }, [cursor, rendered, staffOf, voiceInStaff]);

  // a cada desenho, guarda as linhas de pauta (o SVG só muda quando o alphaTab redesenha)
  useEffect(() => {
    if (host.current) lines.current = collectLines(host.current);
    setHover(null);
  }, [rendered]);

  useEffect(() => {
    if (!insert) setHover(null);
  }, [insert]);

  const locate = useCallback(
    (e: PointerEvent<HTMLDivElement>): Hover | null => {
      const api = apiRef.current;
      const el = host.current;
      if (!api?.boundsLookup || !el || !insert) return null;
      const r = el.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      // limites do alphaTab são justos na pauta: escolhe o sistema e a pauta mais próximos na
      // vertical (as linhas suplementares ficam fora deles)
      const system = nearestBy(api.boundsLookup.staffSystems, (s) => s.realBounds, y, 48);
      const master = system?.findBarAtPos(x);
      const bar = master ? nearestBy(master.bars, (b) => b.realBounds, y, 48) : null;
      if (!master || !bar) return null;
      const rb = bar.realBounds;
      const area = { x: rb.x, y: rb.y, w: rb.w, h: rb.h };
      if (lines.current.length === 0) lines.current = collectLines(el);
      const staff = staffLinesAt(lines.current, x, y, rb.y - 2, rb.y + rb.h + 2);
      if (!staff) return { pick: null, x, y, spacing: 8, bar: area, hint: null };
      if (staff.count !== 5) return { pick: null, x, y, spacing: staff.spacing, bar: area, hint: "Tablatura: use os campos Corda e Traste" };
      const halfSpaces = snapHalfSpaces(y, staff.top, staff.spacing);
      const beat = bar.findBeatAtPos(x)?.beat;
      const pick: StaffPick = { part: bar.bar.staff.track.index, staff: bar.bar.staff.index, measure: master.index, beat: beat ? beat.index : null, halfSpaces };
      return { pick, x, y: staff.top + (halfSpaces * staff.spacing) / 2, spacing: staff.spacing, bar: area, hint: insert.describe(pick) };
    },
    [insert],
  );

  return (
    <div className="score-paper">
      {failed ? <p className="score-error">Não foi possível desenhar a partitura: {failed}</p> : null}
      <div
        className={`score-at${insert ? " score-at--insert" : ""}`}
        ref={host}
        data-testid="score-canvas"
        onPointerMove={insert ? (e) => setHover(locate(e)) : undefined}
        onPointerLeave={insert ? () => setHover(null) : undefined}
        onClick={
          insert
            ? (e) => {
                const h = locate(e as unknown as PointerEvent<HTMLDivElement>);
                if (h?.pick && h.hint) insert.onPick(h.pick);
              }
            : undefined
        }
      />
      {box && !insert ? <div className="score-cursor" style={{ left: box.x, top: box.y, width: box.w, height: box.h }} aria-hidden /> : null}
      {insert && hover ? (
        <>
          <div className="score-bar-hl" style={{ left: hover.bar.x, top: hover.bar.y, width: hover.bar.w, height: hover.bar.h }} aria-hidden />
          {hover.pick ? <div className="score-ghost" style={{ left: hover.x - hover.spacing * 0.65, top: hover.y - hover.spacing * 0.5, width: hover.spacing * 1.3, height: hover.spacing }} aria-hidden /> : null}
          {hover.hint ? (
            <div className="score-tip" role="status" style={{ left: hover.x + 16, top: hover.y - 30 }}>
              {hover.hint}
              {hover.pick ? <small>clique para inserir</small> : null}
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
