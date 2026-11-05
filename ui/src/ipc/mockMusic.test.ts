import { setTransport } from "./transport";
import { createMockTransport } from "./mock";
import { api, musicApi } from "./api";
import { mockSyllables, scoreToMusicXml } from "./mockMusic";

describe("núcleo simulado — música (contrato da API)", () => {
  beforeEach(async () => {
    setTransport(createMockTransport());
    await api.projectCreate({ dir: "/tmp", name: "Música", sampleRate: 48000, bitDepth: 24 });
  });

  it("partitura: criar, editar voz (um desfazer por gesto), validar, MusicXML e audição", async () => {
    const { scoreId, view } = await musicApi.scoreCreate({ title: "Estudo", bpm: 100, meterNumerator: 4, meterDenominator: 4, keyFifths: 1, measures: 2, instruments: [{ kind: "lead" }, { kind: "guitar" }] });
    expect(view.scores[0]).toMatchObject({ title: "Estudo", parts: ["Melodia", "Violão"], measures: 2 });
    const loc = { part: 1, measure: 0, voice: 0 };
    const note = { id: 0, duration: { value: "quarter" as const, dots: 0, tuplet: null }, kind: { type: "chord" as const, notes: [{ pitch: { step: "E" as const, alter: 0, octave: 3 }, tie: false, tab: null, techniques: [] }] }, articulations: [], dynamic: null, grace: null, lyric: null };
    await musicApi.scoreEditVoice(scoreId, loc, { kind: "replace", index: 0, event: note }, 42);
    let v = await musicApi.scoreEditVoice(scoreId, loc, { kind: "fillRests" }, 42);
    expect(v.history.undoLabel).toBe("Editar partitura");
    const sc = await musicApi.scoreGet(scoreId);
    const ev = sc.parts[1]!.measures[0]!.voices[0]!.events[0]!;
    expect(ev.kind.type === "chord" && ev.kind.notes[0]!.tab).toEqual({ string: 4, fret: 2 });
    expect(sc.parts[1]!.measures[0]!.voices[0]!.events).toHaveLength(3);
    const xml = await musicApi.scoreMusicXml(scoreId);
    expect(xml).toContain("<score-partwise");
    expect(xml).toContain("<fret>2</fret>");
    const report = await musicApi.scoreValidate(scoreId, null);
    expect(report.measureIssues).toEqual([]);
    const wav = await musicApi.scoreAudition(scoreId);
    expect(new TextDecoder().decode(new Uint8Array(wav, 0, 4))).toBe("RIFF");
    v = await api.undo();
    const back = await musicApi.scoreGet(scoreId);
    expect(back.parts[1]!.measures[0]!.voices[0]!.events[0]!.kind.type).toBe("rest");
    expect(v.history.canUndo).toBe(true);
    await expect(musicApi.scoreEditVoice(scoreId, { part: 9, measure: 0, voice: 0 }, { kind: "fillRests" })).rejects.toMatchObject({ code: "notation.part_not_found" });
  });

  it("partitura vazia não toca; transposição e import por bytes", async () => {
    const { scoreId } = await musicApi.scoreCreate({ title: "Vazia", bpm: 120, meterNumerator: 3, meterDenominator: 4, measures: 1, instruments: [{ kind: "piano" }] });
    await expect(musicApi.scoreAudition(scoreId)).rejects.toMatchObject({ code: "app.empty_score" });
    const r = await musicApi.scoreImportBytes("riff.musicxml", new TextEncoder().encode("<score-partwise/>"));
    expect(r.view.scores).toHaveLength(2);
    await expect(musicApi.scoreImportBytes("x.txt", new TextEncoder().encode("nada"))).rejects.toMatchObject({ code: "notation_io.unsupported" });
  });

  it("letra: texto → sílabas, tap-sync atômico, deslocamento e exportação", async () => {
    let v = await musicApi.lyricsSetText("Can-to a-qui\noutra linha");
    expect(v.lyrics.lines[0]!.words[0]!.syllables.map((s) => s.text)).toEqual(["Can-", "to"]);
    await expect(musicApi.lyricsExport("/tmp/a.lrc", { format: "srt" })).rejects.toMatchObject({ code: "app.nothing_to_export" });
    await expect(musicApi.lyricsApplyTapSync("word", 0, [960, 10], 2000)).rejects.toMatchObject({ code: "sync.tap_out_of_order" });
    v = await musicApi.lyricsApplyTapSync("word", 0, [0, 960], 1920);
    const spans = v.lyrics.lines[0]!.words.flatMap((w) => w.syllables.map((s) => s.span));
    expect(spans[0]!.start).toBe(0);
    expect(spans[3]!.end).toBe(1920);
    v = await musicApi.lyricsShift({ line: 0, word: 0, syllable: 0 }, { line: 1, word: 1, syllable: 2 }, 480);
    expect(v.lyrics.lines[0]!.words[0]!.syllables[0]!.span!.start).toBe(480);
    await musicApi.lyricsExport("/tmp/a.lrc", { format: "lrc", granularity: "syllable" });
  });

  it("afinação: analisar, editar nota, aplicar (fonte nova) e desfazer devolve o clipe", async () => {
    let v = await api.audioImport(["/x/voz.wav"], null, 0);
    const clip = v.tracks[0]!.clips[0]!;
    v = await musicApi.tuningAnalyze(clip.sourceId);
    expect(v.tuning.maps[0]).toMatchObject({ sourceId: clip.sourceId, noteCount: 6 });
    const map = await musicApi.tuningGet(clip.sourceId);
    await expect(musicApi.tuningApply(clip.sourceId, clip.id)).rejects.toMatchObject({ code: "tuning.nothing_to_apply" });
    await musicApi.tuningEditNote(clip.sourceId, map.notes[0]!.id, { target: 60, centsOffset: 0, flatten: 0, formantSemitones: 0 });
    v = await musicApi.tuningApply(clip.sourceId, clip.id);
    expect(v.tracks[0]!.clips[0]!.sourceId).not.toBe(clip.sourceId);
    v = await api.undo();
    expect(v.tracks[0]!.clips[0]!.sourceId).toBe(clip.sourceId);
    await expect(musicApi.tuningSetReference(470)).rejects.toMatchObject({ code: "tuning.invalid_reference" });
    v = await musicApi.clipTimeStretch(clip.id, 100, 120);
    expect(v.tracks[0]!.clips[0]!.lengthFrames).toBe(Math.round(clip.lengthFrames * (100 / 120)));
  });

  it("afinador: desligado dá erro; ligado devolve leitura em movimento", async () => {
    await expect(musicApi.tunerRead()).rejects.toMatchObject({ code: "app.tuner_off" });
    await musicApi.tunerStart(0);
    const r = await musicApi.tunerRead();
    expect(r).toMatchObject({ name: "A", octave: 4 });
    await musicApi.tunerStop();
  });
});

describe("auxiliares do núcleo simulado", () => {
  it("sílabas aproximadas preservam a palavra", () => {
    for (const w of ["primeira", "linha", "canto", "Can-to", "mar", "a"]) expect(mockSyllables(w).join("")).toBe(w);
    expect(mockSyllables("canto")).toEqual(["can", "to"]);
  });
  it("MusicXML mínimo tem partes e compassos", () => {
    const xml = scoreToMusicXml({ title: "T & U", composer: "", tempo: { tempos: [{ tick: 0, bpm: 120 }], meters: [{ tick: 0, numerator: 4, denominator: 4 }] }, nextEventId: 1, parts: [] });
    expect(xml).toContain("T &amp; U");
  });
});
