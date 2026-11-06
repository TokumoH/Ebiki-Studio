// Adaptador real da porta de atualização (Pure Fabrication): plugin updater do Tauri. O download
// é verificado no Rust contra a chave pública do tauri.conf.json antes de instalar.
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { classifyUpdateError, RELEASES_URL, UpdateError, type Updater, type UpdateMode } from "./updater";

const wrap = (e: unknown) => {
  const msg = e instanceof Error ? e.message : String(e);
  return new UpdateError(classifyUpdateError(msg), msg);
};

export function createTauriUpdater(): Updater {
  let pending: Update | null = null;
  return {
    mode: () => invoke<UpdateMode>("app_update_mode"),
    check: async () => {
      try {
        pending = await check({ timeout: 15_000 });
      } catch (e) {
        throw wrap(e);
      }
      return pending ? { version: pending.version, date: pending.date, notes: pending.body ?? "" } : null;
    },
    install: async (onProgress) => {
      if (!pending) throw new UpdateError("other", "nenhuma atualização verificada");
      let total: number | null = null;
      let done = 0;
      try {
        await pending.downloadAndInstall((ev) => {
          if (ev.event === "Started") total = ev.data.contentLength ?? null;
          else if (ev.event === "Progress") done += ev.data.chunkLength;
          onProgress(done, total);
        });
      } catch (e) {
        throw wrap(e);
      }
    },
    relaunch: () => relaunch(),
    openReleasePage: () => openUrl(RELEASES_URL),
  };
}
