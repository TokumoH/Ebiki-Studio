import { HELP_TOPICS, normalize, searchTopics } from "./helpTopics";

describe("busca da central de ajuda", () => {
  it("ignora acentos e maiúsculas", () => {
    expect(normalize("Edição AFINAÇÃO")).toBe("edicao afinacao");
    expect(searchTopics(HELP_TOPICS, "edicao").map((t) => t.id)).toContain("edicao");
    expect(searchTopics(HELP_TOPICS, "AFINAÇÃO")[0]!.id).toBe("afinacao");
  });

  it("exige todos os termos e prioriza o título", () => {
    const r = searchTopics(HELP_TOPICS, "fade");
    expect(r[0]!.id).toBe("edicao");
    expect(searchTopics(HELP_TOPICS, "fade lrc")).toEqual([]);
    expect(searchTopics(HELP_TOPICS, "exportar")[0]!.id).toBe("exportar");
  });

  it("sem busca devolve todos na ordem original", () => {
    expect(searchTopics(HELP_TOPICS, "  ")).toEqual(HELP_TOPICS);
  });

  it("tópicos têm ids únicos, conteúdo e passos do tour válidos", () => {
    const ids = HELP_TOPICS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const t of HELP_TOPICS) {
      expect(t.steps.length).toBeGreaterThan(0);
      if (t.tourStep !== undefined) expect(t.tourStep).toBeGreaterThanOrEqual(1);
    }
  });
});
