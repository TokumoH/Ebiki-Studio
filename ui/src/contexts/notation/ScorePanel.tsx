import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import "./notation.css";
import { musicApi } from "../../ipc/api";
import { transport } from "../../ipc/transport";
import type { Articulation, Dynamic, InstrumentKind, NoteDuration, NoteValue, Score, ScoreEvent, ScoreExportFormat, ScoreReport, Step, VoiceEdit } from "../../ipc/types";
import { endGesture, errorMessage, newGesture, run, useSession } from "../../core/session";
import { notify } from "../../core/toasts";
import { Button, Dialog, EmptyState, Field, Toggle } from "../../shared/components";
import {
  DURATION_LABELS,
  STEP_NAMES,
  clampCursor,
  defaultOctave,
  eventAt,
  eventLabel,
  keyToAction,
  measureCapacity,
  meterAt,
  midiOf,
  moveCursor,
  noteEvent,
  pitchLabel,
  pitchNear,
  placeEvent,
  previousPitch,
  restEvent,
  spellMidi,
  voiceAt,
  applyVoiceEdits,
  type Cursor,
  type ScoreAction,
} from "./scoreEditing";
import { convertToGp7, isLegacyGuitarPro } from "./alphaTab";
import { ScoreView } from "./ScoreView";

const ENTRY_VALUES: NoteValue[] = ["whole", "half", "quarter", "eighth", "sixteenth", "thirtySecond"];
const VALUE_FRACTION: Record<NoteValue, string> = { whole: "1", half: "1/2", quarter: "1/4", eighth: "1/8", sixteenth: "1/16", thirtySecond: "1/32", sixtyFourth: "1/64", oneTwentyEighth: "1/128" };
const VALUE_SYMBOL: Record<NoteValue, string> = { whole: "𝅝", half: "𝅗𝅥", quarter: "♩", eighth: "♪", sixteenth: "𝅘𝅥𝅯", thirtySecond: "𝅘𝅥𝅰", sixtyFourth: "𝅘𝅥𝅱", oneTwentyEighth: "𝅘𝅥𝅲" };
const ARTICULATIONS: { id: Articulation; label: string; symbol: string }[] = [
  { id: "staccato", label: "Staccato", symbol: "·" },
  { id: "accent", label: "Acento", symbol: ">" },
  { id: "tenuto", label: "Tenuto", symbol: "–" },
  { id: "marcato", label: "Marcato", symbol: "^" },
  { id: "fermata", label: "Fermata", symbol: "𝄐" },
];
const DYNAMICS: Dynamic[] = ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"];
const MAJOR_KEYS = ["Dó♭", "Sol♭", "Ré♭", "Lá♭", "Mi♭", "Si♭", "Fá", "Dó", "Sol", "Ré", "Lá", "Mi", "Si", "Fá♯", "Dó♯"];
const MINOR_KEYS = ["Lá♭", "Mi♭", "Si♭", "Fá", "Dó", "Sol", "Ré", "Lá", "Mi", "Si", "Fá♯", "Dó♯", "Sol♯", "Ré♯", "Lá♯"];
const INSTRUMENTS: { kind: InstrumentKind; label: string }[] = [
  { kind: "lead", label: "Melodia" },
  { kind: "piano", label: "Piano" },
  { kind: "guitar", label: "Violão (tablatura)" },
  { kind: "bass", label: "Baixo (tablatura)" },
  { kind: "drums", label: "Bateria" },
];
const EXPORTS: { id: ScoreExportFormat; label: string; ext: string }[] = [
  { id: "musicXml", label: "MusicXML", ext: "musicxml" },
  { id: "mxl", label: "MusicXML compactado (MXL)", ext: "mxl" },
  { id: "midi", label: "MIDI", ext: "mid" },
];
const IMPORT_ACCEPT = ".musicxml,.xml,.mxl,.mid,.midi,.gp,.gpif,.gp3,.gp4,.gp5,.gpx";
const START: Cursor = { part: 0, measure: 0, voice: 0, index: 0 };

function keyName(fifths: number, mode: "major" | "minor") {
  return `${(mode === "minor" ? MINOR_KEYS : MAJOR_KEYS)[fifths + 7] ?? "?"} ${mode === "minor" ? "menor" : "maior"}`;
}

const fracText = (f: { num: number; den: number }) => `${f.num}/${f.den}`;

/** Partituras: lista, criação/importação/exportação, edição por teclado e render com alphaTab. */
export function ScorePanel() {
  const session = useSession((s) => s.session);
  const [selected, setSelected] = useState<string | null>(null);
  const [score, setScore] = useState<Score | null>(null);
  const [xml, setXml] = useState("");
  const [cursor, setCursor] = useState<Cursor>(START);
  const [duration, setDuration] = useState<NoteDuration>({ value: "quarter", dots: 0, tuplet: null });
  const [report, setReport] = useState<ScoreReport | null>(null);
  const [creating, setCreating] = useState(false);
  const [transposing, setTransposing] = useState(false);
  const [exportFmt, setExportFmt] = useState<ScoreExportFormat>("musicXml");
  const [playing, setPlaying] = useState(false);
  const [tab, setTab] = useState({ string: 1, fret: 0 });
  const audio = useRef<{ ctx: AudioContext; src: AudioBufferSourceNode } | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const editor = useRef<HTMLDivElement>(null);
  const scores = session?.scores ?? [];

  useEffect(() => {
    if (selected && !scores.some((s) => s.id === selected)) setSelected(null);
    if (!selected && scores[0]) setSelected(scores[0].id);
  }, [scores, selected]);

  // estado mais recente para ações em fila (teclas rápidas não podem ler uma partitura velha)
  const scoreRef = useRef<Score | null>(null);
  const cursorRef = useRef<Cursor>(START);
  const durRef = useRef(duration);
  durRef.current = duration;
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const setCur = useCallback((c: Cursor) => {
    cursorRef.current = c;
    setCursor(c);
  }, []);

  const reload = useCallback(async (id: string) => {
    const [sc, x] = await Promise.all([musicApi.scoreGet(id), musicApi.scoreMusicXml(id)]);
    scoreRef.current = sc;
    setScore(sc);
    setXml((old) => (old === x ? old : x));
    setCur(clampCursor(sc, cursorRef.current));
    return sc;
  }, [setCur]);

  // recarrega a partitura a cada mudança da sessão (edição, desfazer, importação…)
  useEffect(() => {
    if (!selected) {
      scoreRef.current = null;
      setScore(null);
      setXml("");
      return;
    }
    queue.current = queue.current.then(() => reload(selected)).catch((e: unknown) => notify.error(errorMessage(e)));
  }, [session, selected, reload]);

  const stopAudio = useCallback(() => {
    const a = audio.current;
    audio.current = null;
    setPlaying(false);
    if (a) {
      try {
        a.src.stop();
      } catch {
        /* já parado */
      }
      void a.ctx.close();
    }
  }, []);
  useEffect(() => stopAudio, [stopAudio]);

  const voiceEdits = async (edits: VoiceEdit[], c: Cursor = cursorRef.current) => {
    if (!selected || edits.length === 0) return false;
    const g = newGesture();
    let ok = true;
    for (const e of edits) {
      ok = await run(() => musicApi.scoreEditVoice(selected, { part: c.part, measure: c.measure, voice: c.voice }, e, g));
      if (!ok) break;
    }
    await endGesture();
    if (ok) await reload(selected);
    return ok;
  };

  /** Coloca um evento no cursor e avança (cria compasso no fim, como editores de partitura). */
  const enter = async (ev: ScoreEvent) => {
    const sc = scoreRef.current;
    const c = cursorRef.current;
    if (!sc || !selected) return;
    const v = voiceAt(sc, c);
    if (!v) return;
    const plan = placeEvent(v, c.index, ev, measureCapacity(sc, c.measure));
    if ("error" in plan) return notify.error(plan.error);
    if (!(await voiceEdits(plan, c))) return;
    const len = applyVoiceEdits(v, plan).events.length;
    if (c.index + 1 < len) return setCur({ ...c, index: c.index + 1 });
    const measures = sc.parts[c.part]?.measures.length ?? 0;
    if (c.measure + 1 >= measures) {
      if (!(await run(() => musicApi.scoreEdit(selected, { kind: "addMeasure", index: measures })))) return;
      await reload(selected);
    }
    setCur({ ...c, measure: c.measure + 1, index: 0 });
  };

  const current = score ? eventAt(score, cursor) : null;
  const measure = score?.parts[cursor.part]?.measures[cursor.measure];
  const part = score?.parts[cursor.part];
  const voice = score ? voiceAt(score, cursor) : null;
  const staffCfg = part?.staves[voice?.staff ?? 0];

  const act = async (a: ScoreAction) => {
    const sc = scoreRef.current;
    const c = cursorRef.current;
    const m = sc?.parts[c.part]?.measures[c.measure];
    if (!sc || !m) return;
    const ev = eventAt(sc, c);
    switch (a.type) {
      case "note": {
        const pitch = pitchNear(a.step, m.key, previousPitch(sc, c), defaultOctave(sc, c));
        return enter(noteEvent(durRef.current, pitch));
      }
      case "rest":
        return enter(restEvent(durRef.current));
      case "duration":
        return setDuration((d) => (durRef.current = { ...d, value: a.value }));
      case "dot":
        return setDuration((d) => (durRef.current = { ...d, dots: d.dots ? 0 : 1 }));
      case "move":
        return setCur(moveCursor(sc, c, a.delta));
      case "semitone": {
        if (ev?.kind.type !== "chord") return;
        const edits: VoiceEdit[] = ev.kind.notes.map((n, note) => ({ kind: "setPitch", index: c.index, note, pitch: spellMidi(Math.min(127, Math.max(0, midiOf(n.pitch) + a.delta)), m.key) }));
        return void (await voiceEdits(edits, c));
      }
      case "delete":
        if (ev?.kind.type !== "chord") return;
        return void (await voiceEdits([{ kind: "replace", index: c.index, event: restEvent(ev.duration) }], c));
      case "tie":
        if (ev?.kind.type !== "chord") return;
        return void (await voiceEdits([{ kind: "setTie", index: c.index, note: 0, tie: !ev.kind.notes[0]?.tie }], c));
    }
  };
  const enqueue = (a: ScoreAction) => {
    queue.current = queue.current.then(() => act(a)).catch((e: unknown) => notify.error(errorMessage(e)));
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const a = keyToAction(e);
    if (!a) return;
    e.preventDefault();
    e.stopPropagation();
    enqueue(a);
  };

  const validate = async () => {
    if (!selected) return;
    try {
      const r = await musicApi.scoreValidate(selected, null);
      setReport(r);
      if (r.measureIssues.length === 0 && r.outOfScale.length === 0) notify.success("Partitura sem problemas de compasso ou de escala.");
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const audition = async () => {
    if (!selected) return;
    if (playing) return stopAudio();
    try {
      const bytes = await musicApi.scoreAudition(selected);
      const ctx = new AudioContext();
      const buf = await ctx.decodeAudioData(bytes.slice(0));
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.connect(ctx.destination);
      src.onended = () => audio.current?.src === src && stopAudio();
      audio.current = { ctx, src };
      src.start();
      setPlaying(true);
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const importFile = async (file: File) => {
    try {
      let bytes: Uint8Array = new Uint8Array(await file.arrayBuffer());
      if (isLegacyGuitarPro(file.name)) bytes = await convertToGp7(bytes);
      const r = await musicApi.scoreImportBytes(file.name, bytes);
      useSession.getState().setSession(r.view);
      setSelected(r.scoreId);
      setCur(START);
      notify.success(`Partitura importada: ${file.name}`);
      for (const w of r.warnings.slice(0, 3)) notify.info(w);
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };

  const exportScore = async () => {
    if (!selected) return;
    const f = EXPORTS.find((x) => x.id === exportFmt)!;
    const title = scores.find((s) => s.id === selected)?.title ?? "partitura";
    const path = await transport().pickSave({ defaultPath: `${title}.${f.ext}`, filters: [{ name: f.label, extensions: [f.ext] }], title: "Exportar partitura" });
    if (path) await run(() => musicApi.scoreExport(selected, path, exportFmt), { success: "Partitura exportada." });
  };

  const print = () => {
    const root = document.documentElement;
    root.classList.add("printing-score");
    const done = () => {
      root.classList.remove("printing-score");
      window.removeEventListener("afterprint", done);
    };
    window.addEventListener("afterprint", done);
    try {
      window.print();
    } finally {
      setTimeout(done, 1000);
    }
  };

  const toggleArticulation = (a: Articulation) => {
    if (!current) return;
    const has = current.articulations.includes(a);
    void voiceEdits([{ kind: has ? "removeArticulation" : "addArticulation", index: cursor.index, articulation: a }]);
  };

  if (!session) return null;
  const voicesInStaff = measure?.voices.filter((v, i) => v.staff === voice?.staff && i < cursor.voice).length ?? 0;
  const meter = score ? meterAt(score, cursor.measure) : { numerator: 4, denominator: 4 };

  return (
    <div className="notation">
      <aside className="notation-side" aria-label="Partituras">
        <ul className="score-list">
          {scores.map((s) => (
            <li key={s.id}>
              <button type="button" aria-pressed={s.id === selected} onClick={() => (setSelected(s.id), setCur(START), setReport(null))} title={s.parts.join(", ")}>
                <span>{s.title}</span>
                <small>
                  {s.parts.length} parte{s.parts.length === 1 ? "" : "s"} · {s.measures} comp.
                </small>
              </button>
            </li>
          ))}
        </ul>
        {scores.length === 0 ? <p className="hint">Nenhuma partitura no projeto.</p> : null}
        <Button onClick={() => setCreating(true)}>+ Nova partitura</Button>
        <Button onClick={() => fileInput.current?.click()}>Importar…</Button>
        <input ref={fileInput} type="file" accept={IMPORT_ACCEPT} hidden aria-label="Arquivo de partitura" onChange={(e) => {
          const f = e.target.files?.[0];
          e.target.value = "";
          if (f) void importFile(f);
        }} />
        {selected ? (
          <>
            <div className="col">
              <select className="select" aria-label="Formato de exportação" value={exportFmt} onChange={(e) => setExportFmt(e.target.value as ScoreExportFormat)}>
                {EXPORTS.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.label}
                  </option>
                ))}
              </select>
              <Button onClick={() => void exportScore()}>Exportar…</Button>
            </div>
            <Button onClick={print} title="Abre a impressão do sistema (salvar como PDF)">
              Imprimir / PDF
            </Button>
            <Button variant="danger" onClick={() => void run(() => musicApi.scoreDelete(selected))}>
              Excluir partitura
            </Button>
          </>
        ) : null}
      </aside>
      <section className="notation-main">
        {!score ? (
          <EmptyState title="Partitura">Crie uma partitura em branco ou importe MusicXML, MIDI ou Guitar Pro.</EmptyState>
        ) : (
          <>
            <div className="notation-toolbar" role="toolbar" aria-label="Entrada de notas">
              <div className="tool-group" role="radiogroup" aria-label="Duração">
                {ENTRY_VALUES.map((v) => (
                  <button key={v} type="button" role="radio" className="btn btn--sm dur" aria-checked={duration.value === v} title={DURATION_LABELS[v]} aria-label={DURATION_LABELS[v]} onClick={() => setDuration((d) => ({ ...d, value: v }))}>
                    {VALUE_SYMBOL[v]}
                  </button>
                ))}
                <Toggle pressed={duration.dots > 0} onChange={(p) => setDuration((d) => ({ ...d, dots: p ? 1 : 0 }))} aria-label="Ponto de aumento" title="Ponto de aumento (.)">
                  ·
                </Toggle>
              </div>
              <div className="tool-group" aria-label="Notas">
                {(Object.keys(STEP_NAMES) as Step[]).map((st) => (
                  <Button key={st} size="sm" onClick={() => enqueue({ type: "note", step: st })} title={`${STEP_NAMES[st]} (${st})`}>
                    {STEP_NAMES[st]}
                  </Button>
                ))}
                <Button size="sm" onClick={() => enqueue({ type: "rest" })} title="Pausa (0)">
                  Pausa
                </Button>
              </div>
              <div className="tool-group" aria-label="Articulações">
                {ARTICULATIONS.map((a) => (
                  <Toggle key={a.id} pressed={!!current?.articulations.includes(a.id)} onChange={() => toggleArticulation(a.id)} aria-label={a.label} title={a.label} disabled={current?.kind.type !== "chord"}>
                    {a.symbol}
                  </Toggle>
                ))}
                <select className="select sel-sm" aria-label="Dinâmica" value={current?.dynamic ?? ""} disabled={!current} onChange={(e) => void voiceEdits([{ kind: "setDynamic", index: cursor.index, dynamic: (e.target.value || null) as Dynamic | null }])}>
                  <option value="">dinâmica</option>
                  {DYNAMICS.map((d) => (
                    <option key={d} value={d}>
                      {d}
                    </option>
                  ))}
                </select>
                <Toggle pressed={current?.kind.type === "chord" && !!current.kind.notes[0]?.tie} onChange={() => enqueue({ type: "tie" })} aria-label="Ligadura de valor" title="Ligadura de valor (+)" disabled={current?.kind.type !== "chord"}>
                  ⁀
                </Toggle>
              </div>
              {staffCfg?.tab ? (
                <div className="tool-group" aria-label="Tablatura">
                  <label className="mini">
                    corda
                    <input className="input num" type="number" min={1} max={staffCfg.tab.tuning.length} value={tab.string} onChange={(e) => setTab((t) => ({ ...t, string: Number(e.target.value) }))} aria-label="Corda" />
                  </label>
                  <label className="mini">
                    traste
                    <input className="input num" type="number" min={0} max={staffCfg.tab.frets} value={tab.fret} onChange={(e) => setTab((t) => ({ ...t, fret: Number(e.target.value) }))} aria-label="Traste" />
                  </label>
                  <Button size="sm" disabled={current?.kind.type !== "chord"} onClick={() => void voiceEdits([{ kind: "setTab", index: cursor.index, note: 0, tab: { string: tab.string, fret: tab.fret } }])}>
                    Definir
                  </Button>
                </div>
              ) : null}
            </div>
            <div className="notation-toolbar" role="toolbar" aria-label="Estrutura da partitura">
              <div className="tool-group">
                <select className="select sel-sm" aria-label="Parte" value={cursor.part} onChange={(e) => setCur(clampCursor(score, { ...cursor, part: Number(e.target.value), voice: 0 }))}>
                  {score.parts.map((p, i) => (
                    <option key={i} value={i}>
                      {p.name}
                    </option>
                  ))}
                </select>
                <select className="select sel-sm" aria-label="Voz" value={cursor.voice} onChange={(e) => setCur(clampCursor(score, { ...cursor, voice: Number(e.target.value) }))}>
                  {(measure?.voices ?? []).map((_, i) => (
                    <option key={i} value={i}>
                      Voz {i + 1}
                    </option>
                  ))}
                </select>
              </div>
              <div className="tool-group">
                <select
                  className="select sel-sm"
                  aria-label="Armadura"
                  value={`${measure?.key.fifths ?? 0}:${measure?.key.mode ?? "major"}`}
                  onChange={(e) => {
                    const [f, m] = e.target.value.split(":");
                    void run(() => musicApi.scoreEdit(selected!, { kind: "setKey", measure: cursor.measure, key: { fifths: Number(f), mode: m as "major" | "minor" } }));
                  }}
                >
                  {(["major", "minor"] as const).flatMap((mode) =>
                    MAJOR_KEYS.map((_, i) => (
                      <option key={`${i - 7}:${mode}`} value={`${i - 7}:${mode}`}>
                        {keyName(i - 7, mode)}
                      </option>
                    )),
                  )}
                </select>
                <select
                  className="select sel-sm"
                  aria-label="Fórmula de compasso"
                  value={`${meter.numerator}/${meter.denominator}`}
                  onChange={(e) => {
                    const [n, d] = e.target.value.split("/").map(Number);
                    void run(() => musicApi.scoreEdit(selected!, { kind: "setMeter", measure: cursor.measure, meter: { numerator: n!, denominator: d! } }));
                  }}
                >
                  {["2/4", "3/4", "4/4", "5/4", "6/8", "7/8", "9/8", "12/8", "2/2", "3/8"].concat(`${meter.numerator}/${meter.denominator}`).filter((v, i, a) => a.indexOf(v) === i).map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
                <Button size="sm" onClick={() => void run(() => musicApi.scoreEdit(selected!, { kind: "addMeasure", index: cursor.measure + 1 }))}>+ Compasso</Button>
                <Button size="sm" onClick={() => void run(() => musicApi.scoreEdit(selected!, { kind: "removeMeasure", index: cursor.measure }))}>− Compasso</Button>
                <Button size="sm" onClick={() => setTransposing(true)}>Transpor…</Button>
                <Button size="sm" onClick={() => void voiceEdits([{ kind: "fillRests" }])} title="Completa a voz atual com pausas">
                  Completar pausas
                </Button>
              </div>
              <div className="tool-group">
                <Button size="sm" onClick={() => void validate()}>Validar</Button>
                <Button size="sm" onClick={() => void audition()} aria-pressed={playing}>
                  {playing ? "■ Parar" : "▶ Ouvir"}
                </Button>
                <Button size="sm" onClick={() => void run(() => musicApi.scoreRenderToTrack(selected!, null, 0), { busy: true, success: "Partitura renderizada numa faixa nova." })}>Para faixa</Button>
              </div>
            </div>
            <div className="notation-status mono" data-testid="score-status" aria-live="polite">
              Comp. {cursor.measure + 1} · voz {cursor.voice + 1} · evento {cursor.index + 1}/{voice?.events.length ?? 0} — {current ? eventLabel(current) : "—"}
              {current?.articulations.length ? ` (${current.articulations.map((a) => ARTICULATIONS.find((x) => x.id === a)?.label).join(", ")})` : ""}
              {current?.dynamic ? ` ${current.dynamic}` : ""} · entrada: {DURATION_LABELS[duration.value].toLowerCase()}
              {duration.dots ? " pontuada" : ""} · {measure ? keyName(measure.key.fifths, measure.key.mode) : ""} {meter.numerator}/{meter.denominator}
            </div>
            <div className="notation-editor" ref={editor} tabIndex={0} role="application" aria-label="Editor de partitura" aria-describedby="score-help" onKeyDown={onKeyDown}>
              <ol className="event-strip" aria-label={`Eventos do compasso ${cursor.measure + 1}`}>
                {(voice?.events ?? []).map((ev, i) => (
                  <li key={ev.id}>
                    <button type="button" tabIndex={-1} aria-current={i === cursor.index} onClick={() => (setCur({ ...cursor, index: i }), editor.current?.focus())}>
                      {ev.kind.type === "rest" ? "pausa" : ev.kind.notes.map((n) => `${n.pitch.step}${n.pitch.alter > 0 ? "♯" : n.pitch.alter < 0 ? "♭" : ""}${n.pitch.octave}`).join("+")} {VALUE_FRACTION[ev.duration.value]}
                      {ev.duration.dots ? "." : ""}
                      {ev.kind.type === "chord" && ev.kind.notes[0]?.tab ? ` [${ev.kind.notes[0].tab.string}/${ev.kind.notes[0].tab.fret}]` : ""}
                    </button>
                  </li>
                ))}
              </ol>
              <ScoreView xml={xml} cursor={cursor} staffOf={voice?.staff ?? 0} voiceInStaff={voicesInStaff} />
            </div>
            <p id="score-help" className="hint">
              Clique no editor e use o teclado: A–G nota · 0 pausa · 1–7 duração (5 = semínima) · . ponto · ←/→ evento · ↑/↓ semitom (Ctrl = oitava) · + ligadura · Delete apaga.
            </p>
            {report ? (
              <div className="score-report" aria-label="Relatório de validação">
                <strong>Validação</strong>
                {report.measureIssues.length === 0 && report.outOfScale.length === 0 ? <span> — nenhum problema.</span> : null}
                <ul>
                  {report.measureIssues.map((m, i) => (
                    <li key={`m${i}`}>
                      <button type="button" onClick={() => setCur(clampCursor(score, { part: m.loc.part, measure: m.loc.measure, voice: m.loc.voice, index: 0 }))}>
                        {score.parts[m.loc.part]?.name}, compasso {m.loc.measure + 1}, voz {m.loc.voice + 1}: {m.issue.kind === "underfilled" ? `faltam ${fracText(m.issue.missing)}` : `sobram ${fracText(m.issue.excess)}`}
                      </button>
                    </li>
                  ))}
                  {report.outOfScale.map((o, i) => (
                    <li key={`o${i}`}>
                      <button type="button" onClick={() => setCur(clampCursor(score, { ...o.event.voice, index: o.event.index }))}>
                        Nota fora da escala: {pitchLabel(o.pitch)} ({score.parts[o.event.voice.part]?.name}, compasso {o.event.voice.measure + 1})
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
          </>
        )}
      </section>
      <CreateScoreDialog open={creating} bpm={session.tempo.tempos[0]?.bpm ?? 120} onClose={() => setCreating(false)} onCreated={(id) => (setSelected(id), setCur(START), setTimeout(() => editor.current?.focus(), 50))} />
      {score && selected ? <TransposeDialog open={transposing} parts={score.parts.map((p) => p.name)} onClose={() => setTransposing(false)} scoreId={selected} /> : null}
    </div>
  );
}

function CreateScoreDialog({ open, bpm, onClose, onCreated }: { open: boolean; bpm: number; onClose: () => void; onCreated: (id: string) => void }) {
  const [title, setTitle] = useState("Nova partitura");
  const [composer, setComposer] = useState("");
  const [tempo, setTempo] = useState(bpm);
  const [meter, setMeter] = useState("4/4");
  const [key, setKey] = useState("0:major");
  const [measures, setMeasures] = useState(8);
  const [instruments, setInstruments] = useState<InstrumentKind[]>(["lead"]);
  const submit = async () => {
    const [n, d] = meter.split("/").map(Number);
    const [f, m] = key.split(":");
    try {
      const r = await musicApi.scoreCreate({ title, composer, bpm: tempo, meterNumerator: n!, meterDenominator: d!, keyFifths: Number(f), keyMode: m as "major" | "minor", measures, instruments: instruments.map((kind) => ({ kind })) });
      useSession.getState().setSession(r.view);
      onCreated(r.scoreId);
      onClose();
    } catch (e) {
      notify.error(errorMessage(e));
    }
  };
  return (
    <Dialog
      title="Nova partitura"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" disabled={!title.trim() || instruments.length === 0} onClick={() => void submit()}>
            Criar partitura
          </Button>
        </>
      }
    >
      <Field label="Título" error={title.trim() ? null : "Informe um título."}>
        <input className="input" value={title} onChange={(e) => setTitle(e.target.value)} aria-label="Título da partitura" maxLength={120} />
      </Field>
      <Field label="Compositor">
        <input className="input" value={composer} onChange={(e) => setComposer(e.target.value)} />
      </Field>
      <div className="row">
        <Field label="BPM">
          <input className="input" type="number" min={20} max={999} value={tempo} onChange={(e) => setTempo(Number(e.target.value))} style={{ width: 80 }} />
        </Field>
        <Field label="Compasso">
          <select className="select" value={meter} onChange={(e) => setMeter(e.target.value)}>
            {["2/4", "3/4", "4/4", "6/8", "12/8", "2/2"].map((m) => (
              <option key={m}>{m}</option>
            ))}
          </select>
        </Field>
        <Field label="Tom">
          <select className="select" value={key} onChange={(e) => setKey(e.target.value)}>
            {(["major", "minor"] as const).flatMap((mode) => MAJOR_KEYS.map((_, i) => <option key={`${i - 7}:${mode}`} value={`${i - 7}:${mode}`}>{keyName(i - 7, mode)}</option>))}
          </select>
        </Field>
        <Field label="Compassos">
          <input className="input" type="number" min={1} max={1000} value={measures} onChange={(e) => setMeasures(Number(e.target.value))} style={{ width: 80 }} />
        </Field>
      </div>
      <fieldset className="instruments">
        <legend>Instrumentos</legend>
        {INSTRUMENTS.map((i) => (
          <label key={i.kind}>
            <input type="checkbox" checked={instruments.includes(i.kind)} onChange={(e) => setInstruments((xs) => (e.target.checked ? [...xs, i.kind] : xs.filter((x) => x !== i.kind)))} /> {i.label}
          </label>
        ))}
      </fieldset>
    </Dialog>
  );
}

function TransposeDialog({ open, parts, scoreId, onClose }: { open: boolean; parts: string[]; scoreId: string; onClose: () => void }) {
  const [semitones, setSemitones] = useState(2);
  const [scope, setScope] = useState(-1);
  const apply = async () => {
    const ok = await run(() => musicApi.scoreEdit(scoreId, { kind: "transpose", scope: scope < 0 ? { scope: "score" } : { scope: "part", target: scope }, semitones }), { success: "Partitura transposta." });
    if (ok) onClose();
  };
  return (
    <Dialog
      title="Transpor"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="primary" disabled={semitones === 0} onClick={() => void apply()}>
            Transpor
          </Button>
        </>
      }
    >
      <Field label="Semitons (−24 a +24)">
        <input className="input" type="number" min={-24} max={24} value={semitones} onChange={(e) => setSemitones(Math.max(-24, Math.min(24, Math.round(Number(e.target.value)))))} aria-label="Semitons" />
      </Field>
      <Field label="Escopo">
        <select className="select" value={scope} onChange={(e) => setScope(Number(e.target.value))} aria-label="Escopo da transposição">
          <option value={-1}>Partitura inteira (armaduras incluídas)</option>
          {parts.map((p, i) => (
            <option key={i} value={i}>
              Só a parte: {p}
            </option>
          ))}
        </select>
      </Field>
    </Dialog>
  );
}
