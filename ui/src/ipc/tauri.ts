// Adaptador real do transporte IPC (Tauri): invoke, eventos e diálogos de arquivo.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Transport } from "./transport";

export const tauriTransport: Transport = {
  kind: "tauri",
  invoke: (cmd, args) => invoke(cmd, args),
  invokeBinary: async (cmd, args) => {
    const r = await invoke<ArrayBuffer | number[]>(cmd, args);
    return r instanceof ArrayBuffer ? r : new Uint8Array(r).buffer;
  },
  listen: async (event, cb) => listen(event, (e) => cb(e.payload as never)),
  pickOpen: async (opts) => {
    const r = await open({ directory: opts.directory ?? false, multiple: opts.multiple ?? false, filters: opts.filters, title: opts.title });
    if (r === null) return null;
    return Array.isArray(r) ? r : [r];
  },
  pickSave: (opts) => save({ defaultPath: opts.defaultPath, filters: opts.filters, title: opts.title }),
};
