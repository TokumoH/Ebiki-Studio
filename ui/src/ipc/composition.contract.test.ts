// Contrato IPC de composição (harmonia + bases): fixtures geradas pelo Rust
// (crates/app-core/tests/contract_fixtures.rs → write_composition_ipc_fixtures).
import * as S from "./schema";
import keys from "./__fixtures__/composition-keys.json";
import field from "./__fixtures__/composition-field.json";
import progressions from "./__fixtures__/composition-progressions.json";
import suggestNext from "./__fixtures__/composition-suggest-next.json";
import userProgression from "./__fixtures__/composition-user-progression.json";
import history from "./__fixtures__/composition-library-history.json";
import realization from "./__fixtures__/composition-realization.json";
import insertedClip from "./__fixtures__/composition-inserted-clip.json";
import baseOptions from "./__fixtures__/composition-base-options.json";
import generated from "./__fixtures__/composition-generated.json";
import bases from "./__fixtures__/composition-bases.json";
import mismatch from "./__fixtures__/composition-mismatch.json";
import needsDecision from "./__fixtures__/composition-insert-base-needs-decision.json";
import inserted from "./__fixtures__/composition-insert-base-inserted.json";
import synth from "./__fixtures__/composition-synth-status.json";
import error from "./__fixtures__/composition-error.json";

describe("contrato IPC de composição Rust ↔ UI", () => {
  it("catálogo de tons: 12 tônicas, 21 modos, 15 gêneros", () => {
    const k = S.HarmonyKeysCatalog.parse(keys);
    expect([k.tonics.length, k.modes.length, k.genres.length]).toEqual([12, 21, 15]);
    expect(k.genres.find((g) => g.genre === "bossaNova")?.code).toBe("bossa_nova");
  });

  it("campo harmônico de Dó maior", () => {
    const f = S.HarmonicField.parse(field);
    expect(f.degrees.map((d) => d.triad.symbol)).toEqual(["C", "Dm", "Em", "F", "G", "Am", "Bdim"]);
    expect(f.degrees[4]?.seventh).toEqual({ symbol: "G7", roman: "V7" });
  });

  it("progressões sugeridas e do usuário", () => {
    const list = S.ProgressionSuggestion.array().parse(progressions);
    expect(list).toHaveLength(3);
    expect(list[0]?.cifra).toEqual(["C", "Am", "F", "G"]);
    expect(list[0]?.progression.ref).toEqual({ kind: "builtin", slug: "doo-wop" });
    for (const p of list) expect(p.cifra).toHaveLength(p.progression.roman.length);
    const u = S.ProgressionView.parse(userProgression);
    expect(u.origin).toBe("user");
    expect(u.ref.kind).toBe("user");
  });

  it("próximo acorde após G7 em Dó começa por Dó", () => {
    const s = S.ChordSuggestion.array().parse(suggestNext);
    expect(s[0]?.symbol).toBe("C");
    expect(s).toHaveLength(4);
    expect(s[0]?.reasonLabel).toBe("Cadência autêntica (V → I)");
  });

  it("realização e clipe inserido (com a sessão)", () => {
    const r = S.Realization.parse(realization);
    expect(r.chords.map((c) => c.symbol)).toEqual(["C", "G7"]);
    const c = S.InsertedClip.parse(insertedClip);
    expect(c.view.tracks.some((t) => t.id === c.trackId)).toBe(true);
    expect(c.relPath.startsWith("audio/generated/")).toBe(true);
  });

  it("histórico da biblioteca", () => {
    expect(S.LibraryHistory.parse(history).canUndo).toBe(true);
  });

  it("opções, bases e base gerada", () => {
    const o = S.BaseOptions.parse(baseOptions);
    expect(o.styles.some((x) => x.value === "waltz" && x.meterNumerator === 3)).toBe(true);
    const list = S.LibraryBase.array().parse(bases);
    expect(new Set(list.map((b) => b.kind.kind))).toEqual(new Set(["generated", "userAudio"]));
    const g = S.GeneratedBase.parse(generated);
    expect(g.events).toHaveLength(30);
    expect(g.events[0]).toEqual({ channel: 0, durationTicks: 960, key: 57, program: 25, startTick: 0, velocity: 100 });
  });

  it("divergência e inserção com tag de status", () => {
    expect(S.MismatchReport.parse(mismatch).needsDecision).toBe(true);
    expect(S.InsertBaseOutcome.parse(needsDecision).status).toBe("needsDecision");
    const done = S.InsertBaseOutcome.parse(inserted);
    expect(done.status).toBe("inserted");
    if (done.status === "inserted") expect(done.decision).toBe("adapt");
  });

  it("estado do sintetizador e erro tipado", () => {
    expect(S.SynthStatus.parse(synth).engine).toBe("simple");
    expect(S.ApiError.parse(error).code).toBe("harmony.invalid_chord");
  });
});
