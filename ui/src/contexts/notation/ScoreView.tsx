import { useEffect, useRef, useState } from "react";
import type { Cursor } from "./scoreEditing";
import { alphaTabSettings, loadAlphaTab } from "./alphaTab";

type AtApi = InstanceType<Awaited<ReturnType<typeof loadAlphaTab>>["AlphaTabApi"]>;

/**
 * Renderização da partitura (pauta + tablatura) pelo alphaTab a partir do MusicXML do núcleo.
 * O cursor de eventos é desenhado por cima, a partir dos limites de batida do alphaTab.
 */
export function ScoreView({ xml, cursor, staffOf, voiceInStaff }: { xml: string; cursor: Cursor | null; staffOf: number; voiceInStaff: number }) {
  const host = useRef<HTMLDivElement>(null);
  const apiRef = useRef<AtApi | null>(null);
  const [ready, setReady] = useState(false);
  const [rendered, setRendered] = useState(0);
  const [failed, setFailed] = useState<string | null>(null);
  const [box, setBox] = useState<{ x: number; y: number; w: number; h: number } | null>(null);

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

  return (
    <div className="score-paper">
      {failed ? <p className="score-error">Não foi possível desenhar a partitura: {failed}</p> : null}
      <div className="score-at" ref={host} />
      {box ? <div className="score-cursor" style={{ left: box.x, top: box.y, width: box.w, height: box.h }} aria-hidden /> : null}
    </div>
  );
}
