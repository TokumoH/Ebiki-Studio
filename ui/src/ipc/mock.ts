// Transporte simulado: um núcleo em memória para rodar a UI no navegador (desenvolvimento e E2E).
// Simula as regras visíveis (histórico, mixer, transporte, medidores); não processa áudio.
import type { Transport } from "./transport";
import type { ChannelStrip, SessionView, StripEdit, StripTarget, Track, TransportView, MetersView } from "./types";
import { emptyMusicView, musicHandlers } from "./mockMusic";

const PPQ = 960;
const SR_DEFAULT = 48_000;
const uuid = () => crypto.randomUUID();

const strip = (): ChannelStrip => ({ volume: 0, pan: 0, mute: false, solo: false, inserts: [], sends: [], output: { kind: "master" } });

function fail(code: string, message: string): never {
  throw { code, message };
}

function validName(name: string) {
  const n = name.trim();
  if (!n || [...n].length > 64) fail("project.invalid_name", "nome inválido: deve ter de 1 a 64 caracteres visíveis");
  return n;
}

const PRESETS: Record<string, string> = {
  leadVocal: "Voz principal",
  backingVocal: "Backing vocal",
  rap: "Voz falada / rap",
  acousticGuitar: "Violão",
  electricGuitarClean: "Guitarra limpa",
  electricGuitarDistorted: "Guitarra distorcida",
  bass: "Baixo",
  drums: "Bateria",
  piano: "Piano / teclado",
  masterBus: "Master",
};

interface State {
  session: SessionView | null;
  undo: { label: string; snapshot: SessionView }[];
  redo: { label: string; snapshot: SessionView }[];
  savedAt: number;
  revision: number;
  playing: "stopped" | "playing" | "paused" | "recording";
  frame: number;
  lastGesture: number | null;
  recent: { path: string; name: string; openedAt: number }[];
}

export function createMockTransport(): Transport {
  const st: State = { session: null, undo: [], redo: [], savedAt: 0, revision: 0, playing: "stopped", frame: 0, lastGesture: null, recent: [] };
  const listeners = new Map<string, Set<(p: unknown) => void>>();
  const emit = (ev: string, p: unknown) => listeners.get(ev)?.forEach((cb) => cb(p));

  const s = () => st.session ?? fail("app.no_project", "Nenhum projeto aberto.");
  const sr = () => st.session?.project.sampleRate ?? SR_DEFAULT;
  const bpm = () => s().tempo.tempos[0]?.bpm ?? 120;
  const tickToFrame = (t: number) => Math.round((t / PPQ) * (60 / bpm()) * sr());
  const frameToTick = (f: number) => Math.round((f / sr()) * (bpm() / 60) * PPQ);

  function view(): SessionView {
    const v = s();
    v.history = {
      canUndo: st.undo.length > 0,
      canRedo: st.redo.length > 0,
      undoLabel: st.undo.at(-1)?.label ?? null,
      redoLabel: st.redo.at(-1)?.label ?? null,
      dirty: st.revision !== st.savedAt,
    };
    const anySolo = v.tracks.some((t) => t.strip.solo);
    v.tracks.forEach((t, i) => {
      t.audible = !t.strip.mute && (!anySolo || t.strip.solo);
      t.meterSlot = i + 1;
      t.clips.forEach((c) => {
        c.startFrame = tickToFrame(c.start);
        c.endTick = frameToTick(c.startFrame + c.lengthFrames);
      });
    });
    v.buses.forEach((b, i) => (b.meterSlot = v.tracks.length + 1 + i));
    v.contentEndFrame = Math.max(0, ...v.tracks.flatMap((t) => t.clips.map((c) => c.startFrame + c.lengthFrames)));
    return structuredClone(v);
  }

  function mutate(label: string, gesture: number | null, fn: (v: SessionView) => void): SessionView {
    const before = structuredClone(s());
    fn(st.session!);
    if (!(gesture !== null && gesture === st.lastGesture)) {
      st.undo.push({ label, snapshot: before });
      st.revision++;
    }
    st.lastGesture = gesture;
    st.redo = [];
    return view();
  }

  function stripOf(v: SessionView, t: StripTarget): ChannelStrip {
    if (t.kind === "master") return v.master;
    if (t.kind === "bus") return v.buses.find((b) => b.id === t.id)?.strip ?? fail("mixing.bus_not_found", "bus não encontrado");
    return v.tracks.find((x) => x.id === t.id)?.strip ?? fail("mixing.strip_not_found", "canal não encontrado");
  }

  function applyEdit(sp: ChannelStrip, e: StripEdit) {
    const clampDb = (x: number) => Math.min(12, Math.max(-144, x));
    switch (e.op) {
      case "volume": sp.volume = clampDb(e.value); break;
      case "pan": sp.pan = Math.min(1, Math.max(-1, e.value)); break;
      case "mute": sp.mute = e.value; break;
      case "solo": sp.solo = e.value; break;
      case "addEffect": sp.inserts.splice(e.index ?? sp.inserts.length, 0, e.slot); break;
      case "removeEffect": sp.inserts = sp.inserts.filter((x) => x.id !== e.id); break;
      case "moveEffect": {
        const i = sp.inserts.findIndex((x) => x.id === e.id);
        const [x] = sp.inserts.splice(i, 1);
        if (x) sp.inserts.splice(Math.min(e.to, sp.inserts.length), 0, x);
        break;
      }
      case "setEffect": sp.inserts = sp.inserts.map((x) => (x.id === e.id ? { ...x, effect: e.effect } : x)); break;
      case "bypass": sp.inserts = sp.inserts.map((x) => (x.id === e.id ? { ...x, bypass: e.value } : x)); break;
      case "output": sp.output = e.value; break;
      case "addSend": sp.sends.push(e.send); break;
      case "removeSend": sp.sends = sp.sends.filter((x) => x.id !== e.id); break;
      case "sendLevel": sp.sends = sp.sends.map((x) => (x.id === e.id ? { ...x, level: clampDb(e.value) } : x)); break;
    }
  }

  function newSession(name: string, root: string, sampleRate: number, bitDepth: number): SessionView {
    return {
      project: { id: uuid(), name, root, sampleRate, bitDepth },
      tempo: { tempos: [{ tick: 0, bpm: 120 }], meters: [{ tick: 0, numerator: 4, denominator: 4 }] },
      tracks: [],
      buses: [],
      master: strip(),
      sources: [],
      recording: {
        metronome: { enabled: false, volume: 0.6, subdivision: 1, accentFirstBeat: true, countInBars: 1 },
        punch: null,
        loopRange: null,
        latencyCompensationFrames: 0,
      },
      history: { canUndo: false, canRedo: false, undoLabel: null, redoLabel: null, dirty: false },
      contentEndFrame: 0,
      ...emptyMusicView(uuid),
    };
  }

  function addTrack(v: SessionView, name: string, index?: number | null): Track {
    const t: Track = {
      id: uuid(),
      name: validName(name),
      clips: [],
      strip: strip(),
      recording: { armed: false, input: null, monitor: "off", takes: [], activeTake: null },
      audible: true,
      meterSlot: 0,
    };
    v.tracks.splice(index ?? v.tracks.length, 0, t);
    return t;
  }

  function transportView(): TransportView {
    const tick = st.session ? frameToTick(st.frame) : 0;
    const meter = st.session?.tempo.meters[0] ?? { numerator: 4, denominator: 4 };
    const beatTicks = (PPQ * 4) / meter.denominator;
    const barTicks = beatTicks * meter.numerator;
    return {
      state: st.playing,
      frame: st.frame,
      tick,
      seconds: st.frame / sr(),
      barBeat: { bar: Math.floor(tick / barTicks) + 1, beat: Math.floor((tick % barTicks) / beatTicks) + 1, tickInBeat: tick % beatTicks },
      bpm: st.session ? bpm() : 120,
      recordStart: null,
    };
  }

  // relógio simulado (~30 Hz)
  setInterval(() => {
    if (!st.session) return;
    if (st.playing === "playing" || st.playing === "recording") {
      st.frame += Math.round(sr() / 30);
      const loop = st.session.recording.loopRange;
      if (loop && st.playing === "playing" && st.frame >= tickToFrame(loop[1])) st.frame = tickToFrame(loop[0]);
    }
    emit("transport", transportView());
    const rolling = st.playing === "playing" || st.playing === "recording";
    const level = (t: Track) => {
      if (!rolling || !t.audible) return 0;
      const f = st.frame;
      const has = t.clips.some((c) => f >= c.startFrame && f < c.startFrame + c.lengthFrames);
      return has ? Math.min(1, 0.35 * Math.pow(10, t.strip.volume / 20) * (0.8 + 0.2 * Math.random())) : 0;
    };
    const tracks = st.session.tracks.map((t) => ({ id: t.id, peak: [level(t), level(t)] as [number, number] }));
    const m = tracks.reduce((a, t) => Math.max(a, t.peak[0]), 0);
    const meters: MetersView = { master: [m, m], tracks, buses: st.session.buses.map((b) => ({ id: b.id, peak: [0, 0] })), xruns: 0, recordOverflow: 0 };
    emit("meters", meters);
  }, 33);

  const handlers: Record<string, (a: Record<string, any>) => unknown> = {
    audio_devices: () => ({ host: "Simulado", outputs: [{ id: "sim-out", name: "Saída simulada", isDefault: true }], inputs: [{ id: "sim-in", name: "Entrada simulada", isDefault: true }] }),
    audio_status: () => ({ mode: "null", outputName: "Saída simulada", inputName: null, sampleRate: sr(), bufferSize: 256, inputChannels: 2, estimatedLatencyMs: 10.7, error: null }),
    audio_configure: (a) => ({ mode: "null", outputName: "Saída simulada", inputName: a.req.inputId ? "Entrada simulada" : null, sampleRate: sr(), bufferSize: a.req.bufferSize ?? 256, inputChannels: 2, estimatedLatencyMs: ((a.req.bufferSize ?? 256) * 2000) / sr(), error: null }),
    project_create: (a) => {
      const name = validName(a.req.name);
      const root = `${a.req.dir}/${name}.amproj`;
      if (st.recent.some((r) => r.path === root)) fail("app.exists", "Já existe um projeto com este nome nesta pasta.");
      st.session = newSession(name, root, a.req.sampleRate, a.req.bitDepth);
      st.undo = [];
      st.redo = [];
      st.revision = st.savedAt = 0;
      st.frame = 0;
      st.recent.unshift({ path: root, name, openedAt: Date.now() / 1000 });
      return view();
    },
    project_open: (a) => {
      const r = st.recent.find((x) => x.path === a.path) ?? fail("persistence.corrupt", "dados corrompidos no projeto: arquivo do projeto não encontrado");
      st.session = newSession(r.name, r.path, SR_DEFAULT, 24);
      st.undo = [];
      st.redo = [];
      st.revision = st.savedAt = 0;
      return view();
    },
    project_view: () => view(),
    project_save: () => {
      st.savedAt = st.revision;
      return view();
    },
    project_close: () => {
      st.session = null;
      st.playing = "stopped";
      return null;
    },
    project_recent: () => st.recent,
    project_forget_recent: (a) => {
      st.recent = st.recent.filter((r) => r.path !== a.path);
      return null;
    },
    project_recovery_check: () => false,
    project_recover: () => view(),
    project_discard_recovery: () => null,
    project_rename: (a) => mutate("Renomear projeto", null, (v) => (v.project.name = validName(a.name))),
    history_undo: () => {
      const e = st.undo.pop();
      if (e) {
        st.redo.push({ label: e.label, snapshot: structuredClone(s()) });
        st.session = e.snapshot;
        st.revision--;
      }
      st.lastGesture = null;
      return view();
    },
    history_redo: () => {
      const e = st.redo.pop();
      if (e) {
        st.undo.push({ label: e.label, snapshot: structuredClone(s()) });
        st.session = e.snapshot;
        st.revision++;
      }
      st.lastGesture = null;
      return view();
    },
    history_end_gesture: () => {
      st.lastGesture = null;
      return null;
    },
    track_create: (a) => mutate("Criar faixa", null, (v) => addTrack(v, a.name, a.index)),
    track_delete: (a) => mutate("Remover faixa", null, (v) => {
      if (!v.tracks.some((t) => t.id === a.trackId)) fail("project.track_not_found", "faixa não encontrada");
      v.tracks = v.tracks.filter((t) => t.id !== a.trackId);
    }),
    track_rename: (a) => mutate("Renomear faixa", null, (v) => {
      const t = v.tracks.find((x) => x.id === a.trackId) ?? fail("project.track_not_found", "faixa não encontrada");
      t.name = validName(a.name);
    }),
    track_move: (a) => mutate("Mover faixa", null, (v) => {
      const i = v.tracks.findIndex((x) => x.id === a.trackId);
      const [t] = v.tracks.splice(i, 1);
      if (t) v.tracks.splice(Math.min(a.to, v.tracks.length), 0, t);
    }),
    audio_import: (a) => mutate("Importar áudio", null, (v) => {
      for (const p of a.paths as string[]) {
        const name = p.split(/[\\/]/).pop() ?? "audio.wav";
        const srcId = uuid();
        const len = sr() * 8;
        v.sources.push({ id: srcId, contentHash: "a".repeat(64), relPath: `audio/imported/${srcId}.wav`, originalName: name, channels: 2, lengthFrames: len });
        const track = a.trackId ? v.tracks.find((t) => t.id === a.trackId) : addTrack(v, name.replace(/\.[^.]+$/, ""));
        if (!track) fail("project.track_not_found", "faixa não encontrada");
        track.clips.push({ id: uuid(), sourceId: srcId, sourceName: name, start: a.atTick, startFrame: 0, offsetFrames: 0, lengthFrames: len, endTick: 0, gain: 0 });
      }
    }),
    clip_move: (a) => mutate("Mover clipe", a.gesture, (v) => {
      const from = v.tracks.find((t) => t.clips.some((c) => c.id === a.clipId)) ?? fail("project.clip_not_found", "clipe não encontrado");
      const to = v.tracks.find((t) => t.id === a.trackId) ?? fail("project.track_not_found", "faixa não encontrada");
      const c = from.clips.find((x) => x.id === a.clipId)!;
      from.clips = from.clips.filter((x) => x.id !== a.clipId);
      to.clips.push({ ...c, start: Math.max(0, a.start) });
    }),
    clip_delete: (a) => mutate("Remover clipe", null, (v) => v.tracks.forEach((t) => (t.clips = t.clips.filter((c) => c.id !== a.clipId)))),
    clip_set_gain: (a) => mutate("Ganho do clipe", a.gesture, (v) => v.tracks.forEach((t) => t.clips.forEach((c) => c.id === a.clipId && (c.gain = a.gainDb)))),
    tempo_set: (a) => mutate("Alterar andamento", null, (v) => {
      if (!(a.bpm >= 20 && a.bpm <= 999)) fail("project.tempo", `BPM fora do intervalo 20–999: ${a.bpm}`);
      const e = v.tempo.tempos.find((x) => x.tick === a.tick);
      if (e) e.bpm = a.bpm;
      else v.tempo.tempos.push({ tick: a.tick, bpm: a.bpm });
      v.tempo.tempos.sort((x, y) => x.tick - y.tick);
    }),
    tempo_remove: (a) => mutate("Remover mudança de andamento", null, (v) => {
      if (a.tick === 0) fail("project.tempo", "o evento no início (tick 0) não pode ser removido");
      v.tempo.tempos = v.tempo.tempos.filter((x) => x.tick !== a.tick);
    }),
    meter_set: (a) => mutate("Alterar fórmula de compasso", null, (v) => {
      const e = v.tempo.meters.find((x) => x.tick === a.tick);
      if (e) Object.assign(e, { numerator: a.numerator, denominator: a.denominator });
      else v.tempo.meters.push({ tick: a.tick, numerator: a.numerator, denominator: a.denominator });
      v.tempo.meters.sort((x, y) => x.tick - y.tick);
    }),
    meter_remove: (a) => mutate("Remover mudança de compasso", null, (v) => (v.tempo.meters = v.tempo.meters.filter((x) => x.tick !== a.tick || a.tick === 0))),
    mixer_edit: (a) => mutate(labelOf(a.edit), a.gesture, (v) => applyEdit(stripOf(v, a.target), a.edit)),
    bus_create: (a) => mutate("Criar bus", null, (v) => v.buses.push({ id: uuid(), name: validName(a.name), strip: strip(), audible: true, meterSlot: 0 })),
    bus_delete: (a) => mutate("Remover bus", null, (v) => {
      v.buses = v.buses.filter((b) => b.id !== a.busId);
      v.tracks.forEach((t) => {
        if (t.strip.output.kind === "bus" && t.strip.output.id === a.busId) t.strip.output = { kind: "master" };
        t.strip.sends = t.strip.sends.filter((x) => x.bus !== a.busId);
      });
    }),
    bus_rename: (a) => mutate("Renomear bus", null, (v) => v.buses.forEach((b) => b.id === a.busId && (b.name = validName(a.name)))),
    mixer_presets: () => Object.entries(PRESETS).map(([id, label]) => ({ id, label })),
    mixer_apply_preset: (a) => mutate(`Preset: ${PRESETS[a.preset] ?? a.preset}`, null, (v) => {
      const sp = stripOf(v, a.target);
      sp.inserts = [
        { id: uuid(), bypass: false, effect: { type: "eq", params: { bands: [{ kind: "highPass", enabled: true, freqHz: 90, gainDb: 0, q: 0.707 }] } } },
        { id: uuid(), bypass: false, effect: { type: "compressor", params: { thresholdDb: -20, ratio: 3.5, attackMs: 5, releaseMs: 80, kneeDb: 6, makeupDb: 4 } } },
      ];
    }),
    mix_analyze: () => ({
      analysis: { master: { peakDb: -0.05, truePeakDb: 0.3, rmsDb: -11, integratedLufs: -9.5, correlation: 0.7, clippedSamples: 12 }, channels: s().tracks.map((t) => ({ name: t.name, peakDb: -6, rmsDb: -20, clippedSamples: 0 })), targetLufs: -14 },
      advice: [
        { severity: "critical", code: "master.clipping", channel: null, message: "O master está clipando (12 amostras). Reduza o volume das faixas ou use um limitador." },
        { severity: "warning", code: "master.too_loud", channel: null, message: "Loudness de -9.5 LUFS está 4.5 dB acima da meta (-14 LUFS); as plataformas vão abaixar o volume." },
      ],
    }),
    recording_track: (a) => mutate("Configurar gravação", null, (v) => {
      const t = v.tracks.find((x) => x.id === a.req.trackId) ?? fail("project.track_not_found", "faixa não encontrada");
      if (a.req.armed !== undefined) t.recording.armed = a.req.armed;
      if (a.req.input !== undefined) t.recording.input = a.req.input;
      if (a.req.monitor !== undefined) t.recording.monitor = a.req.monitor;
    }),
    recording_metronome: (a) => mutate("Metrônomo", null, (v) => (v.recording.metronome = a.config)),
    recording_punch: (a) => mutate("Punch in/out", null, (v) => (v.recording.punch = a.range)),
    recording_loop: (a) => mutate("Região de loop", null, (v) => (v.recording.loopRange = a.range)),
    recording_latency: (a) => mutate("Compensação de latência", null, (v) => (v.recording.latencyCompensationFrames = a.frames)),
    take_select: () => view(),
    transport_play: () => {
      if (st.playing !== "recording") st.playing = "playing";
      return null;
    },
    transport_pause: () => {
      st.playing = "paused";
      return null;
    },
    transport_stop: (a) => {
      const wasRec = st.playing === "recording";
      st.playing = "stopped";
      if (a.returnToStart) st.frame = 0;
      return wasRec ? view() : null;
    },
    transport_seek: (a) => {
      st.frame = tickToFrame(Math.max(0, a.tick));
      return null;
    },
    transport_record: () => {
      if (!s().tracks.some((t) => t.recording.armed && t.recording.input)) fail("app.nothing_armed", "Arme ao menos uma faixa com entrada definida para gravar.");
      st.playing = "recording";
      return null;
    },
    transport_state: () => transportView(),
    export_formats: () => ["wav", "aiff", "flac", "mp3", "ogg"],
    export_mix: () => null,
    export_stems: (a) => s().tracks.map((t, i) => `${a.dir}/${String(i + 1).padStart(2, "0")} - ${t.name}.wav`),
  };

  // Fases 6–8 (partituras, letra, afinação): ver mockMusic.ts
  const music = musicHandlers({ s, mutate, view, fail, sr, uuid });
  Object.assign(handlers, music.handlers);

  function labelOf(e: StripEdit): string {
    const labels: Record<string, string> = { volume: "Volume", pan: "Pan", mute: "Mute", solo: "Solo", addEffect: "Adicionar efeito", removeEffect: "Remover efeito", moveEffect: "Reordenar efeito", setEffect: "Ajustar efeito", bypass: "Bypass de efeito", output: "Saída do canal", addSend: "Adicionar envio", removeSend: "Remover envio", sendLevel: "Nível do envio" };
    return labels[e.op] ?? e.op;
  }

  return {
    kind: "mock",
    async invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
      const h = handlers[cmd];
      if (!h) throw { code: "ui.unknown_command", message: `comando não simulado: ${cmd}` };
      await new Promise((r) => setTimeout(r, 0));
      return h(args) as T;
    },
    async invokeBinary(cmd, args = {}) {
      const mb = music.binary(cmd, args);
      if (mb) return mb;
      if (cmd !== "waveform_peaks") throw { code: "ui.unknown_command", message: cmd };
      const src = s().sources.find((x) => x.id === args.sourceId) ?? fail("app.source_unavailable", "Áudio indisponível.");
      const fpp = Number(args.framesPerPeak) || 256;
      const n = Math.ceil(src.lengthFrames / fpp);
      const out = new Float32Array(n * 2);
      for (let i = 0; i < n; i++) {
        const a = 0.15 + 0.6 * Math.abs(Math.sin(i / 37)) * (0.6 + 0.4 * Math.sin(i / 5.3));
        out[i * 2] = -a;
        out[i * 2 + 1] = a;
      }
      return out.buffer;
    },
    async listen(event, cb) {
      const set = listeners.get(event) ?? new Set();
      set.add(cb as (p: unknown) => void);
      listeners.set(event, set);
      return () => set.delete(cb as (p: unknown) => void);
    },
    async pickOpen(opts) {
      if (opts.directory) return ["/home/usuario/Musica"];
      return opts.multiple ? ["/home/usuario/Musica/voz.wav", "/home/usuario/Musica/violao.flac"] : ["/home/usuario/Musica/voz.wav"];
    },
    async pickSave(opts) {
      return opts.defaultPath ? `/home/usuario/Musica/${opts.defaultPath}` : "/home/usuario/Musica/mix.wav";
    },
  };
}
