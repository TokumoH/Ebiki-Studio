import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../ipc/api";
import { transport } from "../ipc/transport";
import { refreshAudio, useAudio } from "../core/audio";
import { run, useSession } from "../core/session";
import { useLive } from "../core/live";
import { useTheme } from "../core/theme";
import { loopRangeFromCursor } from "../core/bars";
import { Button, InlineRename } from "../shared/components";
import { TransportBar } from "../contexts/transport/TransportBar";
import { DockLayout } from "../panels/DockLayout";
import { openPanel, resetLayout, togglePanel, useDockState } from "../panels/dockControl";
import { findClip, normalizeClip, setPolarity, setReversed, splitAtCursor } from "../contexts/project/clipActions";
import { useTimelineView } from "../contexts/project/timelineView";
import { useClipViewMode } from "../contexts/project/spectrogramCache";
import { MenuBar } from "../shared/MenuBar";
import { buildMenus } from "./menuModel";
import { HelpCenter } from "./HelpCenter";
import { AboutDialog } from "./AboutDialog";
import { UpdateDialog } from "./UpdateDialog";
import { useTooltipPrefs } from "./tooltips";
import { useUpdates } from "./updates";
import { Tutorial, shouldAutoStartTutorial, useTour } from "./Tutorial";
import { AudioSettingsDialog } from "./AudioSettingsDialog";
import { ExportDialog } from "./ExportDialog";
import { ShortcutsDialog } from "./ShortcutsDialog";
import { CloseProjectDialog, closeProject } from "./CloseProjectDialog";
import { eventKeys, isActivationKey, isTypingTarget, loadShortcuts, type Shortcut } from "./shortcuts";

const AUDIO_EXT = ["wav", "wave", "aif", "aiff", "flac", "mp3", "ogg", "oga", "m4a", "aac", "mp4"];

export async function importAudio(trackId: string | null = null) {
  const files = await transport().pickOpen({ multiple: true, title: "Importar áudio", filters: [{ name: "Áudio", extensions: AUDIO_EXT }] });
  if (!files?.length) return;
  const tick = useLive.getState().transport?.tick ?? 0;
  await run(() => api.audioImport(files, trackId, Math.max(0, tick)), { busy: true, success: `${files.length} arquivo(s) importado(s).` });
}

/** Abre outro projeto (pergunta antes se houver alterações não salvas). */
async function openOtherProject(askToClose: () => void) {
  if (useSession.getState().session?.history.dirty) return askToClose();
  const r = await transport().pickOpen({ directory: true, title: "Abrir projeto (.ebiki)" });
  const path = r?.[0];
  if (path) await run(() => api.projectOpen(path), { busy: true });
}

async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await document.documentElement.requestFullscreen();
  } catch {
    /* sem suporte a tela cheia neste ambiente */
  }
}

export function Workspace() {
  const session = useSession((s) => s.session)!;
  const busy = useSession((s) => s.busy);
  const { theme } = useTheme();
  const meters = useLive((s) => s.meters);
  const selectedTrackId = useSession((s) => s.selectedTrackId);
  const selectedClipId = useSession((s) => s.selectedClipId);
  const openPanels = useDockState((s) => s.open);
  const spectrum = useClipViewMode((s) => s.spectrum);
  const audio = useAudio((a) => a.status);
  const [dialog, setDialog] = useState<"audio" | "export" | "shortcuts" | "close" | "help" | "about" | null>(null);
  const [statusBar, setStatusBar] = useState(true);
  const tooltipsOn = useTooltipPrefs((s) => s.enabled);
  const [shortcutList, setShortcutList] = useState<Shortcut[]>(loadShortcuts);
  const shortcuts = useRef(shortcutList);
  shortcuts.current = shortcutList;
  const startTour = useTour((s) => s.start);

  useEffect(() => {
    void refreshAudio().catch(() => undefined);
  }, []);

  // primeira vez: o tutorial abre sozinho (pulável; "não mostrar ao abrir" fica lembrado)
  useEffect(() => {
    if (shouldAutoStartTutorial()) startTour(1);
  }, [startTour]);

  // atualização: aviso discreto na barra de status; antes de instalar, para o transporte (uma
  // gravação em andamento vira take) e salva — no Windows a instalação encerra o app na hora
  const updateNotice = useUpdates((u) => (u.notice && u.info ? u.info.version : null));
  const showUpdate = useUpdates((u) => u.show);
  const prepareForUpdate = useCallback(async () => {
    // via run(): aplica a sessão devolvida e mostra o erro; falhou → a instalação é abortada
    const st = useLive.getState().transport?.state;
    if (st && st !== "stopped" && !(await run(() => api.stop(false)))) throw new Error("não foi possível parar o transporte");
    if (useSession.getState().session?.history.dirty && !(await run(() => api.projectSave()))) throw new Error("não foi possível salvar o projeto");
  }, []);

  // registro único de comandos: usado pelos atalhos de teclado e pelo menu principal
  const actions = useRef<Record<string, () => void>>({});
  actions.current = {
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
      newProject: () => (useSession.getState().session?.history.dirty ? setDialog("close") : void closeProject()),
      openProject: () => void openOtherProject(() => setDialog("close")),
      closeProject: () => (useSession.getState().session?.history.dirty ? setDialog("close") : void closeProject()),
      audioSettings: () => setDialog("audio"),
      shortcutsDialog: () => setDialog("shortcuts"),
      help: () => setDialog("help"),
      about: () => setDialog("about"),
      checkUpdates: () => void useUpdates.getState().check({ manual: true }),
      tutorial: () => startTour(1),
      themeDark: () => useTheme.getState().theme !== "dark" && useTheme.getState().toggle(),
      themeLight: () => useTheme.getState().theme !== "light" && useTheme.getState().toggle(),
      zoomIn: () => useTimelineView.getState().zoomIn(),
      zoomOut: () => useTimelineView.getState().zoomOut(),
      zoomReset: () => useTimelineView.getState().resetZoom(),
      toggleSpectrum: () => {
        const id = useSession.getState().selectedTrackId;
        if (id) useClipViewMode.getState().toggle(id);
      },
      toggleStatusBar: () => setStatusBar((v) => !v),
      toggleTooltips: () => useTooltipPrefs.getState().toggle(),
      fullscreen: () => void toggleFullscreen(),
      resetLayout: () => void resetLayout(),
      deleteClip: () => {
        const id = useSession.getState().selectedClipId;
        if (id) void run(() => api.clipDelete(id));
      },
      normalizeClip: () => {
        const id = useSession.getState().selectedClipId;
        if (id) void normalizeClip(id);
      },
      polarityClip: () => {
        const f = findClip(useSession.getState().session, useSession.getState().selectedClipId);
        if (f) void setPolarity(f.clip.id, !f.clip.polarityInverted);
      },
      reverseClip: () => {
        const f = findClip(useSession.getState().session, useSession.getState().selectedClipId);
        if (f) void setReversed(f.clip.id, !f.clip.reversed);
      },
      armTrack: () => {
        const st = useSession.getState();
        const t = st.session?.tracks.find((x) => x.id === st.selectedTrackId);
        if (t) void run(() => api.recordingTrack({ trackId: t.id, armed: !t.recording.armed, input: t.recording.input ?? { left: 0, right: null } }));
      },
      deleteTrack: () => {
        const id = useSession.getState().selectedTrackId;
        if (id) void run(() => api.trackDelete(id));
      },
  };

  const runCommand = (id: string) => {
    if (id.startsWith("panel:")) openPanel(id.slice(6));
    else if (id.startsWith("togglePanel:")) togglePanel(id.slice(12));
    else actions.current[id]?.();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTypingTarget(e.target) || document.querySelector(".dialog-backdrop")) return;
      const keys = eventKeys(e);
      if (isActivationKey(keys, e.target)) return;
      const sc = shortcuts.current.find((s) => s.keys === keys);
      if (!sc) return;
      e.preventDefault();
      actions.current[sc.id]?.();
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
  const selectedTrack = session.tracks.find((t) => t.id === selectedTrackId);
  const selectedClip = findClip(session, selectedClipId)?.clip ?? null;
  const menus = buildMenus({
    shortcuts: shortcutList,
    run: runCommand,
    canUndo: h.canUndo,
    canRedo: h.canRedo,
    undoLabel: h.undoLabel,
    redoLabel: h.redoLabel,
    theme,
    openPanels,
    statusBar,
    tooltips: tooltipsOn,
    metronome: session.recording.metronome.enabled,
    loop: session.recording.loopRange !== null,
    hasSelectedTrack: Boolean(selectedTrack),
    selectedTrackArmed: selectedTrack?.recording.armed ?? false,
    selectedTrackSpectrum: Boolean(selectedTrack && spectrum[selectedTrack.id]),
    clip: selectedClip ? { polarityInverted: selectedClip.polarityInverted, reversed: selectedClip.reversed } : null,
  });
  return (
    <div className="app" aria-busy={busy}>
      <header className="appbar">
        <div className="menubar-row" data-tour="menu">
          <span className="brand-mark" role="img" aria-label="Ebiki Studio" />
          <MenuBar menus={menus} />
          <span style={{ flex: 1 }} />
          <span className="project-name" title={session.project.root}>
            <InlineRename value={session.project.name} label={`Renomear projeto ${session.project.name}`} inputLabel="Novo nome do projeto" onCommit={(name) => void run(() => api.projectRename(name))} />
            {h.dirty ? <span className="dirty" title="Alterações não salvas"> ●</span> : null}
          </span>
        </div>
        <div className="topbar" data-tour="transporte">
          <Button icon variant="ghost" disabled={!h.canUndo} title={h.undoLabel ? `Desfazer: ${h.undoLabel} (Ctrl+Z)` : "Desfazer"} aria-label="Desfazer" onClick={() => void run(() => api.undo())}>
            ↶
          </Button>
          <Button icon variant="ghost" disabled={!h.canRedo} title={h.redoLabel ? `Refazer: ${h.redoLabel}` : "Refazer"} aria-label="Refazer" onClick={() => void run(() => api.redo())}>
            ↷
          </Button>
          <span className="sep" />
          <TransportBar />
          <span style={{ flex: 1 }} />
          <Button size="sm" onClick={() => void importAudio(useSession.getState().selectedTrackId)} title="Importar áudio (Ctrl+I)">
            Importar
          </Button>
          <Button size="sm" onClick={() => setDialog("export")} title="Exportar (Ctrl+E)">
            Exportar
          </Button>
          <Button size="sm" variant="primary" onClick={() => void run(() => api.projectSave(), { success: "Projeto salvo." })} title="Salvar (Ctrl+S)">
            Salvar
          </Button>
        </div>
      </header>
      <DockLayout theme={theme} />
      {statusBar ? (
      <footer className="statusbar" aria-label="Status">
        <span data-tip="status.audio">
          {audio ? (audio.mode === "device" ? `Saída: ${audio.outputName ?? "padrão"}` : <span className="warn">Sem dispositivo de áudio (modo silencioso)</span>) : "…"}
        </span>
        <span className="mono" data-tip="status.format">{session.project.sampleRate / 1000} kHz · {session.project.bitDepth} bits</span>
        {audio?.bufferSize ? <span className="mono" data-tip="status.buffer">buffer {audio.bufferSize} · ~{audio.estimatedLatencyMs?.toFixed(1) ?? "?"} ms</span> : null}
        {meters && meters.xruns > 0 ? <span className="warn">{meters.xruns} falha(s) de buffer — aumente o buffer</span> : null}
        <span style={{ flex: 1 }} />
        {updateNotice ? (
          <button type="button" className="statusbar-update" onClick={showUpdate}>
            ● Versão {updateNotice} disponível
          </button>
        ) : null}
        <span>{h.dirty ? "Alterações não salvas (autosave a cada 2 min)" : "Salvo"}</span>
      </footer>
      ) : <span />}
      <AudioSettingsDialog open={dialog === "audio"} onClose={() => setDialog(null)} onChanged={useAudio.getState().setStatus} />
      <ExportDialog open={dialog === "export"} onClose={() => setDialog(null)} />
      <ShortcutsDialog open={dialog === "shortcuts"} onClose={() => setDialog(null)} onChanged={setShortcutList} />
      <HelpCenter
        open={dialog === "help"}
        onClose={() => setDialog(null)}
        onStartTour={(step) => {
          setDialog(null);
          startTour(step);
        }}
        onEditShortcuts={() => setDialog("shortcuts")}
      />
      <AboutDialog open={dialog === "about"} onClose={() => setDialog(null)} />
      <UpdateDialog beforeInstall={prepareForUpdate} />
      <Tutorial />
      <CloseProjectDialog open={dialog === "close"} onClose={() => setDialog(null)} />
    </div>
  );
}
