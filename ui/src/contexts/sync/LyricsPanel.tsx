import { useEffect, useMemo, useRef, useState } from "react";
import "./lyrics.css";
import { api, musicApi } from "../../ipc/api";
import { transport } from "../../ipc/transport";
import type { Granularity, LyricsFileFormat } from "../../ipc/types";
import { run, useSession } from "../../core/session";
import { useLive } from "../../core/live";
import { notify } from "../../core/toasts";
import { Button, Field, Segmented } from "../../shared/components";
import { activeUnit, beatsToTicks, finishTap, formatPosition, lastSyllable, lyricUnits, lyricsText, startTap, tap, tapsRemaining, ticksToBeats, undoTap, type TapState } from "./lyricsModel";

const GRANULARITY: { value: Granularity; label: string }[] = [
  { value: "syllable", label: "Sílaba" },
  { value: "word", label: "Palavra" },
  { value: "line", label: "Linha" },
];
const EXPORTS: { id: string; label: string; format: LyricsFileFormat; ext: string }[] = [
  { id: "lrc-line", label: "LRC por linha", format: { format: "lrc", granularity: "line" }, ext: "lrc" },
  { id: "lrc-word", label: "LRC por palavra", format: { format: "lrc", granularity: "word" }, ext: "lrc" },
  { id: "lrc-syllable", label: "LRC por sílaba", format: { format: "lrc", granularity: "syllable" }, ext: "lrc" },
  { id: "srt", label: "Legenda SRT", format: { format: "srt" }, ext: "srt" },
];

/** Letra sincronizada: texto → sílabas, tempos manuais, sincronização por toque e LRC/SRT. */
export function LyricsPanel() {
  const session = useSession((s) => s.session);
  const tick = useLive((s) => s.transport?.tick ?? 0);
  const [text, setText] = useState("");
  const [g, setG] = useState<Granularity>("word");
  const [sel, setSel] = useState(0);
  const [start, setStart] = useState(0);
  const [end, setEnd] = useState(1);
  const [shift, setShift] = useState(0.5);
  const [exportId, setExportId] = useState("lrc-syllable");
  const [tapState, setTapState] = useState<TapState | null>(null);
  const tapArea = useRef<HTMLDivElement>(null);
  const lines = session?.lyrics.lines;
  const units = useMemo(() => lyricUnits(lines ?? [], g), [lines, g]);
  const serverText = useMemo(() => lyricsText(lines ?? []), [lines]);
  const meter = session?.tempo.meters[0];

  // o texto acompanha a letra do núcleo (desfazer, importação) enquanto não houver edição pendente
  const lastServer = useRef("");
  useEffect(() => {
    setText((t) => (t === lastServer.current ? serverText : t));
    lastServer.current = serverText;
  }, [serverText]);

  const selected = units[Math.min(sel, units.length - 1)];
  // o formulário acompanha a unidade selecionada **no mesmo render** (estado derivado): com um
  // useEffect havia um intervalo após a seleção em que o que o usuário digitava era sobrescrito
  const formKey = selected?.span ? `${sel}:${selected.span.start}:${selected.span.end}` : null;
  const [syncedKey, setSyncedKey] = useState<string | null>(null);
  if (formKey !== syncedKey) {
    setSyncedKey(formKey);
    if (selected?.span) {
      setStart(ticksToBeats(selected.span.start));
      setEnd(ticksToBeats(selected.span.end));
    }
  }

  // tap-sync: Espaço marca, Backspace desfaz, Enter conclui, Esc cancela (captura antes dos atalhos globais)
  const tapRef = useRef(tapState);
  tapRef.current = tapState;
  useEffect(() => {
    if (!tapState) return;
    const onKey = (e: KeyboardEvent) => {
      const st = tapRef.current;
      if (!st) return;
      if (![" ", "Backspace", "Enter", "Escape"].includes(e.key)) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.repeat) return;
      if (e.key === " ") {
        const now = useLive.getState().transport?.tick ?? 0;
        if (tapsRemaining(st) <= 0) return void finish(st, now);
        const r = tap(st, now);
        if (r.ok) setTapState(r.state);
        else notify.error(r.error);
      } else if (e.key === "Backspace") setTapState(undoTap(st));
      else if (e.key === "Enter") void finish(st, useLive.getState().transport?.tick ?? 0);
      else cancelTap();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [tapState !== null]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!session || !lines) return null;

  const beginTap = async () => {
    if (units.length === 0) return notify.error("Aplique um texto antes de sincronizar.");
    setTapState(startTap(units.length, sel));
    if (useLive.getState().transport?.state !== "playing") await run(() => api.play());
    setTimeout(() => tapArea.current?.focus(), 0);
  };
  const cancelTap = () => {
    setTapState(null);
    void run(() => api.pause());
  };
  const finish = async (st: TapState, now: number) => {
    const r = finishTap(st, now);
    if (!r.ok) return notify.error(r.error);
    setTapState(null);
    await run(() => api.pause());
    await run(() => musicApi.lyricsApplyTapSync(g, r.from, r.taps, r.end), { success: `Sincronização aplicada (${r.taps.length} ${GRANULARITY.find((x) => x.value === g)!.label.toLowerCase()}s).` });
  };

  const exportLyrics = async () => {
    const f = EXPORTS.find((x) => x.id === exportId)!;
    const path = await transport().pickSave({ defaultPath: `${session.project.name}.${f.ext}`, filters: [{ name: f.label, extensions: [f.ext] }], title: "Exportar letra" });
    if (path) await run(() => musicApi.lyricsExport(path, f.format), { success: `Letra exportada (${f.label}).` });
  };
  const importLyrics = async () => {
    const r = await transport().pickOpen({ title: "Importar letra", filters: [{ name: "Letra (LRC/SRT)", extensions: ["lrc", "srt"] }] });
    if (r?.[0]) await run(() => musicApi.lyricsImport(r[0]!), { success: "Letra importada." });
  };

  const active = activeUnit(units, tick);
  const nextTapUnit = tapState ? units[tapState.from + tapState.taps.length] : undefined;
  const allSyllables = lastSyllable(lines);
  const byLine: [number, { u: (typeof units)[number]; i: number }[]][] = [];
  units.forEach((u, i) => {
    const last = byLine[byLine.length - 1];
    if (last && last[0] === u.first.line) last[1].push({ u, i });
    else byLine.push([u.first.line, [{ u, i }]]);
  });

  return (
    <div className="lyrics">
      <section className="lyrics-text">
        <Field label="Texto da letra (uma linha por verso; hífens separam sílabas)">
          <textarea className="input lyrics-area" value={text} onChange={(e) => setText(e.target.value)} aria-label="Texto da letra" rows={6} spellCheck={false} />
        </Field>
        <div className="row">
          <Button variant="primary" disabled={text === serverText} onClick={() => void run(() => musicApi.lyricsSetText(text), { success: "Letra atualizada." })}>
            Aplicar texto
          </Button>
        </div>
        <div className="row">
          <select className="select" aria-label="Formato de exportação da letra" value={exportId} onChange={(e) => setExportId(e.target.value)}>
            {EXPORTS.map((x) => (
              <option key={x.id} value={x.id}>
                {x.label}
              </option>
            ))}
          </select>
          <Button onClick={() => void exportLyrics()}>Exportar…</Button>
          <Button onClick={() => void importLyrics()}>Importar LRC/SRT…</Button>
        </div>
      </section>
      <section className="lyrics-sync">
        <div className="row">
          <Segmented label="Granularidade" value={g} options={GRANULARITY} onChange={(v) => (setG(v), setSel(0))} />
          <span className="hint">{units.length} unidades · {units.filter((u) => u.span).length} com tempo</span>
        </div>
        <ol className="lyric-lines" aria-label="Unidades da letra">
          {units.length === 0 ? <li className="hint">Cole o texto ao lado e clique em “Aplicar texto”.</li> : null}
          {byLine.map(([line, items]) => (
            <li key={line}>
              <span className="line-no mono">{line + 1}</span>
              {items.map(({ u, i }) => (
                <button
                  key={i}
                  type="button"
                  className="lyric-unit"
                  aria-pressed={i === sel}
                  data-active={i === active || undefined}
                  data-next={nextTapUnit === u || undefined}
                  onClick={() => setSel(i)}
                  title={u.span ? `${formatPosition(u.span.start, meter)} → ${formatPosition(u.span.end, meter)}` : "sem tempo"}
                >
                  <span>{u.text}</span>
                  <small className="mono">{u.span ? formatPosition(u.span.start, meter) : "—"}</small>
                </button>
              ))}
            </li>
          ))}
        </ol>
        {selected ? (
          <div className="lyrics-forms">
            <div className="row">
              <strong className="sel-text" title={selected.text}>
                “{selected.text}”
              </strong>
              <Field label="Início (tempos)">
                <input className="input num" type="number" step={0.25} min={0} value={start} onChange={(e) => setStart(Number(e.target.value))} aria-label="Início em tempos" />
              </Field>
              <Button size="sm" onClick={() => setStart(ticksToBeats(tick))} title="Usar a posição atual do transporte">
                ← cursor
              </Button>
              <Field label="Fim (tempos)">
                <input className="input num" type="number" step={0.25} min={0} value={end} onChange={(e) => setEnd(Number(e.target.value))} aria-label="Fim em tempos" />
              </Field>
              <Button size="sm" onClick={() => setEnd(ticksToBeats(tick))} title="Usar a posição atual do transporte">
                ← cursor
              </Button>
              <Button size="sm" variant="primary" onClick={() => void run(() => musicApi.lyricsSetSpan(selected.unit, beatsToTicks(start), beatsToTicks(end)))}>
                Definir
              </Button>
              <Button size="sm" onClick={() => void run(() => musicApi.lyricsClearSpan(selected.unit))}>
                Limpar
              </Button>
            </div>
            <div className="row">
              <Field label="Deslocar (tempos)">
                <input className="input num" type="number" step={0.25} value={shift} onChange={(e) => setShift(Number(e.target.value))} aria-label="Deslocamento em tempos" />
              </Field>
              <Button size="sm" disabled={!allSyllables || shift === 0} onClick={() => allSyllables && void run(() => musicApi.lyricsShift(selected.first, allSyllables, Math.round(shift * 960)))}>
                Deslocar da seleção ao fim
              </Button>
            </div>
          </div>
        ) : null}
        <div className={`tap-box${tapState ? " tap-box--on" : ""}`} ref={tapArea} tabIndex={-1} aria-label="Sincronização por toque">
          {tapState ? (
            <>
              <strong>Toque Espaço no início de cada {GRANULARITY.find((x) => x.value === g)!.label.toLowerCase()}</strong>
              <span data-testid="tap-progress" className="mono">
                {tapState.taps.length}/{tapState.total - tapState.from} toques
              </span>
              <span>próxima: “{nextTapUnit?.text ?? "fim — Espaço ou Enter conclui"}”</span>
              <Button size="sm" onClick={() => setTapState(undoTap(tapState))}>Desfazer toque</Button>
              <Button size="sm" variant="primary" disabled={tapState.taps.length === 0} onClick={() => void finish(tapState, tick)}>
                Concluir
              </Button>
              <Button size="sm" variant="ghost" onClick={cancelTap}>Cancelar</Button>
            </>
          ) : (
            <>
              <Button onClick={() => void beginTap()} disabled={units.length === 0}>
                Sincronizar por toque
              </Button>
              <span className="hint">Começa na unidade selecionada e toca o projeto · Espaço marca · ⌫ desfaz · Enter conclui · Esc cancela</span>
            </>
          )}
        </div>
      </section>
    </div>
  );
}
