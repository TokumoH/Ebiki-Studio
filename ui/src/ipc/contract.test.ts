// Contrato IPC: as fixtures são geradas pelo Rust (crates/app-core/tests/contract_fixtures.rs).
import * as S from "./schema";
import session from "./__fixtures__/session.json";
import transport from "./__fixtures__/transport.json";
import meters from "./__fixtures__/meters.json";
import edits from "./__fixtures__/strip-edits.json";
import targets from "./__fixtures__/strip-targets.json";
import error from "./__fixtures__/error.json";

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
