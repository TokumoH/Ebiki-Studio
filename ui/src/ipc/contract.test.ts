// Contrato IPC: as fixtures são geradas pelo Rust (crates/app-core/tests/contract_fixtures.rs).
import * as S from "./schema";
import session from "./__fixtures__/session.json";
import transport from "./__fixtures__/transport.json";
import meters from "./__fixtures__/meters.json";
import edits from "./__fixtures__/strip-edits.json";
import targets from "./__fixtures__/strip-targets.json";
import error from "./__fixtures__/error.json";
import musicSession from "./__fixtures__/music-session.json";
import score from "./__fixtures__/score.json";
import report from "./__fixtures__/score-report.json";
import voiceEdits from "./__fixtures__/voice-edits.json";
import scoreEdits from "./__fixtures__/score-edits.json";
import lyricsReq from "./__fixtures__/lyrics-requests.json";
import tuningMap from "./__fixtures__/tuning-map.json";
import tuner from "./__fixtures__/tuner-reading.json";

describe("contrato IPC Rust ↔ UI", () => {
  it("SessionView", () => {
    const v = S.SessionView.parse(session);
    expect(v.tracks[0]?.strip.inserts.map((i) => i.effect.type)).toEqual(["compressor", "eq"]);
    expect(v.tracks[0]?.recording.input).toEqual({ left: 0, right: 1 });
    expect(v.buses[0]?.name).toBe("Reverb");
  });
  it("TransportView e MetersView", () => {
    expect(S.TransportView.parse(transport).state).toBe("playing");
    expect(S.MetersView.parse(meters).tracks.length).toBeGreaterThan(0);
  });
  it("StripEdit e StripTarget que a UI envia são os que o Rust produz", () => {
    expect(S.StripEdit.array().parse(edits)).toHaveLength(6);
    expect(S.StripTarget.array().parse(targets)).toHaveLength(3);
  });
  it("ApiError", () => {
    expect(S.ApiError.parse(error).code).toBe("project.invalid_name");
  });
});

describe("contrato IPC — música (Fases 6–8)", () => {
  it("SessionView com letra sincronizada, partituras, afinação e PitchCorrector", () => {
    const v = S.SessionView.parse(musicSession);
    const syl = v.lyrics.lines.flatMap((l) => l.words.flatMap((w) => w.syllables));
    expect(syl.length).toBeGreaterThan(0);
    expect(syl.some((s) => s.span !== null)).toBe(true);
    expect(syl.some((s) => s.span === null)).toBe(true);
    expect(v.scores[0]).toMatchObject({ title: "Estudo", parts: ["Voz", "Violão"], measures: 2 });
    expect(v.tuning.maps[0]).toMatchObject({ noteCount: 1, editCount: 1 });
    const pc = v.tracks[0]!.strip.inserts.find((i) => i.effect.type === "pitchCorrector");
    expect(S.PitchCorrectorParams.parse(pc?.effect.params)).toMatchObject({ keyRoot: 7, scaleMask: 0b1010_1101_0101 });
  });
  it("Score completo (score_get) e relatório de validação", () => {
    const sc = S.Score.parse(score);
    expect(sc.parts[1]!.staves[0]!.tab?.tuning).toHaveLength(6);
    const ev = sc.parts[0]!.measures[0]!.voices[0]!.events[0]!;
    expect(ev).toMatchObject({ dynamic: "mf", articulations: ["staccato"], kind: { type: "chord" } });
    const r = S.ScoreReport.parse(report);
    expect(r.measureIssues[0]!.issue.kind).toBe("underfilled");
    expect(r.outOfScale[0]!.pitch.step).toBe("F");
  });
  it("edições de voz/partitura que a UI envia são as que o Rust produz", () => {
    expect(S.VoiceEdit.array().parse(voiceEdits).map((e) => e.kind)).toContain("setTab");
    expect(S.ScoreEdit.array().parse(scoreEdits).map((e) => e.kind)).toContain("transpose");
    const lr = lyricsReq as { units: unknown; formats: unknown; noteEdit: unknown; scale: unknown };
    expect(S.LyricUnit.array().parse(lr.units)).toHaveLength(3);
    expect(S.LyricsFileFormat.array().parse(lr.formats)).toHaveLength(3);
    expect(S.NoteEdit.parse(lr.noteEdit).flatten).toBe(0.5);
    expect(S.ScaleCorrection.parse(lr.scale).key).toBe(9);
  });
  it("mapa de afinação e leitura do afinador", () => {
    const m = S.PitchCorrectionMap.parse(tuningMap);
    expect(m.notes).toHaveLength(1);
    expect(m.edits[m.notes[0]!.id]?.target).toBe(69);
    expect(S.TunerReading.parse(tuner)).toMatchObject({ name: "A", octave: 4 });
  });
});
