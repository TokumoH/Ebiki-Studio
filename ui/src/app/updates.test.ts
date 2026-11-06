import { beforeEach, describe, expect, it, vi } from "vitest";
import { classifyUpdateError, createMockUpdater, setUpdater, UpdateError, type Updater } from "../ipc/updater";
import { APP_VERSION } from "./about";
import { formatProgress, isNewer, loadUpdatePrefs, useUpdates } from "./updates";

const initial = useUpdates.getState();

function fakeUpdater(over: Partial<Updater> = {}): Updater {
  return {
    mode: async () => "inApp",
    check: async () => ({ version: "99.0.0", notes: "Novidades" }),
    install: async (p) => p(10, 10),
    relaunch: vi.fn(async () => {}),
    openReleasePage: vi.fn(async () => {}),
    ...over,
  };
}

beforeEach(() => {
  localStorage.clear();
  useUpdates.setState({ ...initial, prefs: loadUpdatePrefs() }, true);
});

describe("isNewer", () => {
  it("compara versões numericamente, não como texto", () => {
    expect(isNewer("0.10.0", "0.9.0")).toBe(true);
    expect(isNewer("v0.2.1", "0.2.0")).toBe(true);
    expect(isNewer("1.0.0", "1.0")).toBe(false);
    expect(isNewer("0.2.0", "0.2.0")).toBe(false);
    expect(isNewer("0.1.9", "0.2.0")).toBe(false);
    expect(isNewer("0.3.0-beta.1", "0.2.0")).toBe(true);
  });
});

describe("preferências", () => {
  it("verificar ao abrir começa desligado (o app é offline por padrão)", () => {
    expect(loadUpdatePrefs()).toEqual({ checkOnOpen: false, skipped: null });
  });
  it("ignora preferências corrompidas", () => {
    localStorage.setItem("ebiki.updates.v1", "{não é json");
    expect(loadUpdatePrefs().checkOnOpen).toBe(false);
  });
  it("persiste a escolha", () => {
    useUpdates.getState().setCheckOnOpen(true);
    expect(loadUpdatePrefs().checkOnOpen).toBe(true);
  });
});

describe("verificação", () => {
  it("manual com versão nova abre o diálogo", async () => {
    setUpdater(fakeUpdater());
    await useUpdates.getState().check({ manual: true });
    const s = useUpdates.getState();
    expect(s.status).toBe("available");
    expect(s.open).toBe(true);
    expect(s.notice).toBe(false);
  });

  it("manual sem versão nova informa que está atualizado", async () => {
    setUpdater(fakeUpdater({ check: async () => ({ version: APP_VERSION, notes: "" }) }));
    await useUpdates.getState().check({ manual: true });
    expect(useUpdates.getState().status).toBe("upToDate");
  });

  it("automática só acende o aviso na barra de status, sem abrir diálogo", async () => {
    setUpdater(fakeUpdater());
    await useUpdates.getState().check({ manual: false });
    expect(useUpdates.getState()).toMatchObject({ status: "available", open: false, notice: true });
  });

  it("automática respeita a versão pulada; a manual mostra mesmo assim", async () => {
    setUpdater(fakeUpdater());
    await useUpdates.getState().check({ manual: true });
    useUpdates.getState().skip();
    expect(loadUpdatePrefs().skipped).toBe("99.0.0");
    await useUpdates.getState().check({ manual: false });
    expect(useUpdates.getState()).toMatchObject({ status: "idle", notice: false });
    await useUpdates.getState().check({ manual: true });
    expect(useUpdates.getState().status).toBe("available");
  });

  it("falha automática é silenciosa; a manual explica o motivo", async () => {
    setUpdater(fakeUpdater({ check: async () => Promise.reject(new UpdateError("offline", "dns")) }));
    await useUpdates.getState().check({ manual: false });
    expect(useUpdates.getState()).toMatchObject({ status: "idle", open: false });
    await useUpdates.getState().check({ manual: true });
    expect(useUpdates.getState()).toMatchObject({ status: "error", error: "offline", open: true });
  });
});

describe("instalação", () => {
  it("baixa, salva o projeto e reinicia, nessa ordem", async () => {
    const order: string[] = [];
    const u = fakeUpdater({
      install: async (p) => {
        order.push("install");
        p(5, 10);
      },
      relaunch: async () => void order.push("relaunch"),
    });
    setUpdater(u);
    await useUpdates.getState().check({ manual: true });
    await useUpdates.getState().install(async () => void order.push("save"));
    expect(order).toEqual(["install", "save", "relaunch"]);
    expect(useUpdates.getState().progress).toEqual({ done: 5, total: 10 });
  });

  it("assinatura recusada não reinicia", async () => {
    const relaunch = vi.fn(async () => {});
    setUpdater(fakeUpdater({ install: async () => Promise.reject(new UpdateError("signature", "bad")), relaunch }));
    await useUpdates.getState().check({ manual: true });
    const save = vi.fn(async () => {});
    await useUpdates.getState().install(save);
    expect(useUpdates.getState()).toMatchObject({ status: "error", error: "signature" });
    expect(save).not.toHaveBeenCalled();
    expect(relaunch).not.toHaveBeenCalled();
  });

  it("instalação manual (.deb/.rpm) não instala dentro do app", async () => {
    const install = vi.fn(async () => {});
    setUpdater(fakeUpdater({ mode: async () => "manual", install }));
    await useUpdates.getState().check({ manual: true });
    await useUpdates.getState().install(async () => {});
    expect(install).not.toHaveBeenCalled();
    expect(useUpdates.getState().status).toBe("available");
  });
});

describe("porta e mock", () => {
  it("classifica erros do plugin", () => {
    expect(classifyUpdateError("Signature verification failed")).toBe("signature");
    expect(classifyUpdateError("error sending request: dns error")).toBe("offline");
    expect(classifyUpdateError("Could not fetch a valid release JSON from the remote")).toBe("other");
    expect(classifyUpdateError("algo estranho")).toBe("other");
  });

  it("mock segue o cenário configurado", async () => {
    const store: Record<string, string> = {};
    const m = createMockUpdater((k) => store[k] ?? null);
    expect(await m.check()).toBeNull();
    store["ebiki.mock.update"] = JSON.stringify({ version: "1.2.3", notes: "x" });
    expect(await m.check()).toEqual({ version: "1.2.3", notes: "x" });
    store["ebiki.mock.update"] = "offline";
    await expect(m.check()).rejects.toMatchObject({ kind: "offline" });
    store["ebiki.mock.updateMode"] = "manual";
    expect(await m.mode()).toBe("manual");
  });

  it("formata o progresso em MB", () => {
    expect(formatProgress(8_700_000, 14_000_000)).toBe("8,7 de 14 MB");
    expect(formatProgress(1_000_000, null)).toBe("1 MB");
  });
});
