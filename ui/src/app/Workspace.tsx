import { useEffect, useRef, useState } from "react";
import { api } from "../ipc/api";
import { transport } from "../ipc/transport";
import type { AudioStatus } from "../ipc/types";
import { run, useSession } from "../core/session";
import { useLive } from "../core/live";
import { useTheme } from "../core/theme";
import { loopRangeFromCursor } from "../core/bars";
import { Button, InlineRename } from "../shared/components";
import { TransportBar } from "../contexts/transport/TransportBar";
import { DockLayout } from "../panels/DockLayout";
import { PanelsMenu } from "../panels/PanelsMenu";
import { splitAtCursor } from "../contexts/project/clipActions";
import { AudioSettingsDialog } from "./AudioSettingsDialog";
import { ExportDialog } from "./ExportDialog";
import { ShortcutsDialog } from "./ShortcutsDialog";
import { CloseProjectDialog, closeProject } from "./CloseProjectDialog";
import { eventKeys, isActivationKey, isTypingTarget, loadShortcuts } from "./shortcuts";

const AUDIO_EXT = ["wav", "wave", "aif", "aiff", "flac", "mp3", "ogg", "oga", "m4a", "aac", "mp4"];

export async function importAudio(trackId: string | null = null) {
  const files = await transport().pickOpen({ multiple: true, title: "Importar áudio", filters: [{ name: "Áudio", extensions: AUDIO_EXT }] });
  if (!files?.length) return;
  const tick = useLive.getState().transport?.tick ?? 0;
  await run(() => api.audioImport(files, trackId, Math.max(0, tick)), { busy: true, success: `${files.length} arquivo(s) importado(s).` });
}

export function Workspace() {
  const session = useSession((s) => s.session)!;
  const busy = useSession((s) => s.busy);
  const { theme, toggle } = useTheme();
  const meters = useLive((s) => s.meters);
  const [audio, setAudio] = useState<AudioStatus | null>(null);
  const [dialog, setDialog] = useState<"audio" | "export" | "shortcuts" | "close" | null>(null);
  const shortcuts = useRef(loadShortcuts());

  useEffect(() => {
    void api.audioStatus().then(setAudio).catch(() => undefined);
  }, []);

  useEffect(() => {
    const actions: Record<string, () => void> = {
      playPause: () => {
        const st = useLive.getState().transport?.state;
        void run(() => (st === "playing" || st === "recording" ? api.pause() : api.play()));
      },
      stopReturn: () => void run(() => api.stop(true)),
      record: () => {
        const t = useLive.getState().transport;
        void run(() => (t?.state === "recording" || t?.state === "countin" ? api.stop(false) : api.record(t?.tick ?? 0, (useSession.getState().session?.recording.metronome.countInBars ?? 0) > 0)));
      },
      undo: () => void run(() => api.undo()),
      redo: () => void run(() => api.redo()),
      redoAlt: () => void run(() => api.redo()),
      save: () => void run(() => api.projectSave(), { success: "Projeto salvo." }),
      newTrack: () => void run(() => api.trackCreate(`Faixa ${(useSession.getState().session?.tracks.length ?? 0) + 1}`)),
      import: () => void importAudio(useSession.getState().selectedTrackId),
      export: () => setDialog("export"),
      metronome: () => {
        const m = useSession.getState().session?.recording.metronome;
        if (m) void run(() => api.recordingMetronome({ ...m, enabled: !m.enabled }));
      },
      loop: () => {
        const s = useSession.getState().session;
        if (!s) return;
        // liga: 4 compassos a partir do compasso do cursor; desliga se já houver loop
        void run(() => api.recordingLoop(s.recording.loopRange ? null : loopRangeFromCursor(s.tempo, useLive.getState().transport?.tick ?? 0)));
      },
      home: () => void run(() => api.seek(0)),
      split: () => void splitAtCursor(),
    };
    const onKey = (e: KeyboardEvent) => {
      if (isTypingTarget(e.target) || document.querySelector(".dialog-backdrop")) return;
      const keys = eventKeys(e);
      if (isActivationKey(keys, e.target)) return;
      const sc = shortcuts.current.find((s) => s.keys === keys);
      if (!sc) return;
      e.preventDefault();
      actions[sc.id]?.();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // arrastar arquivos de áudio do sistema para a janela importa na faixa selecionada (ou em faixas novas)
  useEffect(() => {
    let off: (() => void) | undefined;
    void transport()
      .listen<{ paths?: string[] }>("tauri://drag-drop", (p) => {
        const files = (p.paths ?? []).filter((f) => AUDIO_EXT.includes(f.split(".").pop()?.toLowerCase() ?? ""));
        if (!files.length) return;
        const tick = useLive.getState().transport?.tick ?? 0;
        void run(() => api.audioImport(files, null, Math.max(0, tick)), { busy: true, success: `${files.length} arquivo(s) importado(s).` });
      })
      .then((f) => (off = f));
    return () => off?.();
  }, []);

  useEffect(() => {
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      if (useSession.getState().session?.history.dirty) e.preventDefault();
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, []);

  const h = session.history;
  return (
    <div className="app" aria-busy={busy}>
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" role="img" aria-label="Ebiki Studio" />
        </div>
        <span className="project-name" title={session.project.root}>
          <InlineRename value={session.project.name} label={`Renomear projeto ${session.project.name}`} inputLabel="Novo nome do projeto" onCommit={(name) => void run(() => api.projectRename(name))} />
          {h.dirty ? <span className="dirty" title="Alterações não salvas"> ●</span> : null}
        </span>
        <span className="sep" />
        <Button icon variant="ghost" disabled={!h.canUndo} title={h.undoLabel ? `Desfazer: ${h.undoLabel} (Ctrl+Z)` : "Desfazer"} aria-label="Desfazer" onClick={() => void run(() => api.undo())}>
          ↶
        </Button>
        <Button icon variant="ghost" disabled={!h.canRedo} title={h.redoLabel ? `Refazer: ${h.redoLabel}` : "Refazer"} aria-label="Refazer" onClick={() => void run(() => api.redo())}>
          ↷
        </Button>
        <span className="sep" />
        <TransportBar />
        <span style={{ flex: 1 }} />
        <PanelsMenu />
        <Button size="sm" onClick={() => void importAudio(useSession.getState().selectedTrackId)} title="Importar áudio (Ctrl+I)">
          Importar
        </Button>
        <Button size="sm" onClick={() => setDialog("export")} title="Exportar (Ctrl+E)">
          Exportar
        </Button>
        <Button size="sm" variant="primary" onClick={() => void run(() => api.projectSave(), { success: "Projeto salvo." })} title="Salvar (Ctrl+S)">
          Salvar
        </Button>
        <Button size="sm" variant="ghost" icon onClick={() => setDialog("audio")} title="Configurações de áudio" aria-label="Configurações de áudio">
          ⚙
        </Button>
        <Button size="sm" variant="ghost" icon onClick={() => setDialog("shortcuts")} title="Atalhos de teclado" aria-label="Atalhos de teclado">
          ⌨
        </Button>
        <Button size="sm" variant="ghost" icon onClick={toggle} title="Alternar tema" aria-label="Alternar tema">
          ◐
        </Button>
        <Button size="sm" variant="ghost" icon onClick={() => (h.dirty ? setDialog("close") : void closeProject())} title="Fechar projeto" aria-label="Fechar projeto">
          ✕
        </Button>
      </header>
      <DockLayout theme={theme} />
      <footer className="statusbar" aria-label="Status">
        <span>
          {audio ? (audio.mode === "device" ? `Saída: ${audio.outputName ?? "padrão"}` : <span className="warn">Sem dispositivo de áudio (modo silencioso)</span>) : "…"}
        </span>
        <span className="mono">{session.project.sampleRate / 1000} kHz · {session.project.bitDepth} bits</span>
        {audio?.bufferSize ? <span className="mono">buffer {audio.bufferSize} · ~{audio.estimatedLatencyMs?.toFixed(1) ?? "?"} ms</span> : null}
        {meters && meters.xruns > 0 ? <span className="warn">{meters.xruns} falha(s) de buffer — aumente o buffer</span> : null}
        <span style={{ flex: 1 }} />
        <span>{h.dirty ? "Alterações não salvas (autosave a cada 2 min)" : "Salvo"}</span>
      </footer>
      <AudioSettingsDialog open={dialog === "audio"} onClose={() => setDialog(null)} onChanged={setAudio} />
      <ExportDialog open={dialog === "export"} onClose={() => setDialog(null)} />
      <ShortcutsDialog open={dialog === "shortcuts"} onClose={() => setDialog(null)} onChanged={(s) => (shortcuts.current = s)} />
      <CloseProjectDialog open={dialog === "close"} onClose={() => setDialog(null)} />
    </div>
  );
}
