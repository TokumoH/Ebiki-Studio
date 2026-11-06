//! Núcleo de tempo real. Regras: sem alocação, sem bloqueio, sem liberar memória, sem pânico.

use crate::graph::*;
use crate::input_tap::InputTapRt;
use crate::metronome::Click;
use crate::recorder::RecorderRt;
use crate::telemetry::Telemetry;
use crate::transport::TransportState;
use crate::MAX_BLOCK;
use std::sync::Arc;

/// Mensagens da thread de controle para o núcleo.
pub enum Message {
    SwapGraph(Box<RenderGraph>),
    Play,
    Pause,
    /// Para e (opcionalmente) volta para a posição dada.
    Stop {
        return_to: Option<u64>,
    },
    Seek(u64),
    /// Inicia gravação em `start` (com contagem se configurada no metrônomo).
    Record {
        start: u64,
        count_in: bool,
    },
    AttachRecorder(Box<RecorderRt>),
    DetachRecorder,
    /// Liga a derivação de entrada (afinador). Substitui a anterior.
    AttachInputTap(Box<InputTapRt>),
    DetachInputTap,
}

/// Objetos devolvidos para liberação fora do tempo real.
pub enum Garbage {
    Graph(Box<RenderGraph>),
    Recorder(Box<RecorderRt>),
    InputTap(Box<InputTapRt>),
}

/// Itens de lixo que o núcleo segura se a fila encher (controle sem coletar).
const PENDING_GARBAGE_CAP: usize = 64;
pub struct EngineCore {
    graph: Box<RenderGraph>,
    messages: rtrb::Consumer<Message>,
    garbage: rtrb::Producer<Garbage>,
    telemetry: Arc<Telemetry>,
    state: TransportState,
    position: u64,
    click: Click,
    count_in_remaining: u64,
    count_in_interval: u64,
    count_in_elapsed: u64,
    count_in_beats_per_bar: u64,
    record_start: u64,
    recorder: Option<Box<RecorderRt>>,
    recording_started: bool,
    input_tap: Option<Box<InputTapRt>>,
    master_l: Vec<f32>,
    master_r: Vec<f32>,
    click_l: Vec<f32>,
    click_r: Vec<f32>,
    /// Garbage que não coube na fila (fica até haver espaço; nunca é liberado aqui).
    /// Transbordo da fila de lixo, pré-alocado (push sem alocar enquanto houver capacidade).
    pending_garbage: Vec<Garbage>,
}

impl EngineCore {
    pub(crate) fn new(
        graph: Box<RenderGraph>,
        messages: rtrb::Consumer<Message>,
        garbage: rtrb::Producer<Garbage>,
        telemetry: Arc<Telemetry>,
    ) -> Self {
        let sr = graph.sample_rate;
        Self {
            graph,
            messages,
            garbage,
            telemetry,
            state: TransportState::Stopped,
            position: 0,
            click: Click::new(sr),
            count_in_remaining: 0,
            count_in_interval: 1,
            count_in_elapsed: 0,
            count_in_beats_per_bar: 4,
            record_start: 0,
            recorder: None,
            recording_started: false,
            input_tap: None,
            master_l: vec![0.0; MAX_BLOCK],
            master_r: vec![0.0; MAX_BLOCK],
            click_l: vec![0.0; MAX_BLOCK],
            click_r: vec![0.0; MAX_BLOCK],
            pending_garbage: Vec::with_capacity(PENDING_GARBAGE_CAP),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.graph.sample_rate
    }

    pub fn telemetry(&self) -> Arc<Telemetry> {
        self.telemetry.clone()
    }

    pub fn state(&self) -> TransportState {
        self.state
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    fn discard(&mut self, g: Garbage) {
        self.flush_garbage();
        if let Err(rtrb::PushError::Full(g)) = self.garbage.push(g) {
            if self.pending_garbage.len() < self.pending_garbage.capacity() {
                self.pending_garbage.push(g); // capacidade reservada: não aloca
            } else {
                // último recurso (controle travado por muito tempo): vazar em vez de liberar no RT
                std::mem::forget(g);
            }
        }
    }

    /// Reenvia o transbordo para a fila, na ordem, enquanto houver espaço.
    fn flush_garbage(&mut self) {
        while !self.pending_garbage.is_empty() {
            if self.garbage.is_full() {
                return;
            }
            let g = self.pending_garbage.remove(0);
            if let Err(rtrb::PushError::Full(g)) = self.garbage.push(g) {
                self.pending_garbage.insert(0, g);
                return;
            }
        }
    }

    fn set_state(&mut self, s: TransportState) {
        self.state = s;
        self.telemetry.set_state(s);
    }

    fn handle_messages(&mut self) {
        while let Ok(msg) = self.messages.pop() {
            match msg {
                Message::SwapGraph(mut new) => {
                    inherit(&mut new, &mut self.graph);
                    let old = std::mem::replace(&mut self.graph, new);
                    self.discard(Garbage::Graph(old));
                }
                Message::Play => {
                    if matches!(self.state, TransportState::Stopped | TransportState::Paused) {
                        self.set_state(TransportState::Playing);
                    }
                }
                Message::Pause => {
                    if self.state.is_rolling() || self.state == TransportState::CountIn {
                        self.end_recording();
                        self.set_state(TransportState::Paused);
                    }
                }
                Message::Stop { return_to } => {
                    self.end_recording();
                    self.set_state(TransportState::Stopped);
                    if let Some(p) = return_to {
                        self.position = p;
                    }
                    self.reset_processors();
                }
                Message::Seek(p) => {
                    if self.state != TransportState::Recording
                        && self.state != TransportState::CountIn
                    {
                        self.position = p;
                    }
                }
                Message::Record { start, count_in } => {
                    self.record_start = start;
                    self.position = start;
                    self.recording_started = false;
                    let m = &self.graph.metronome;
                    if count_in && m.count_in_bars > 0 {
                        let meter = self.graph.tempo.meter_at(
                            self.graph
                                .tempo
                                .frame_to_tick(start, self.graph.sample_rate),
                        );
                        let bpm = self.graph.tempo.bpm_at(
                            self.graph
                                .tempo
                                .frame_to_tick(start, self.graph.sample_rate),
                        );
                        let beat_seconds = 60.0 / bpm * 4.0 / meter.denominator as f64;
                        self.count_in_interval =
                            ((beat_seconds * self.graph.sample_rate as f64) as u64).max(1);
                        self.count_in_beats_per_bar = meter.numerator as u64;
                        self.count_in_remaining = self.count_in_interval
                            * meter.numerator as u64
                            * m.count_in_bars as u64;
                        self.count_in_elapsed = 0;
                        self.set_state(TransportState::CountIn);
                    } else {
                        self.set_state(TransportState::Recording);
                    }
                }
                Message::AttachRecorder(r) => {
                    if let Some(old) = self.recorder.replace(r) {
                        self.discard(Garbage::Recorder(old));
                    }
                }
                Message::DetachRecorder => {
                    if let Some(old) = self.recorder.take() {
                        self.discard(Garbage::Recorder(old));
                    }
                }
                Message::AttachInputTap(t) => {
                    if let Some(old) = self.input_tap.replace(t) {
                        self.discard(Garbage::InputTap(old));
                    }
                }
                Message::DetachInputTap => {
                    if let Some(old) = self.input_tap.take() {
                        self.discard(Garbage::InputTap(old));
                    }
                }
            }
        }
    }

    fn end_recording(&mut self) {
        if self.state == TransportState::Recording || self.state == TransportState::CountIn {
            self.recording_started = false;
        }
    }

    fn reset_processors(&mut self) {
        let g = &mut self.graph;
        for s in g
            .tracks
            .iter_mut()
            .map(|t| &mut t.strip)
            .chain(g.buses.iter_mut())
            .chain(std::iter::once(&mut g.master))
        {
            for i in &mut s.inserts {
                if let Some(p) = i.processor.as_mut() {
                    p.reset();
                }
            }
        }
    }

    /// Processa um callback. `output`/`input` intercalados.
    pub fn process(
        &mut self,
        output: &mut [f32],
        out_channels: usize,
        input: &[f32],
        in_channels: usize,
    ) {
        self.handle_messages();
        self.flush_garbage();
        if let Some(tap) = self.input_tap.as_mut() {
            if in_channels > 0 {
                tap.capture(input, in_channels, input.len() / in_channels);
            }
        }
        if out_channels == 0 {
            return;
        }
        let total = output.len() / out_channels;
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_BLOCK);
            let in_slice = if in_channels > 0 {
                let a = (done * in_channels).min(input.len());
                let b = ((done + n) * in_channels).min(input.len());
                &input[a..b]
            } else {
                &input[0..0]
            };
            self.process_block(n, in_slice, in_channels);
            for f in 0..n {
                let (l, r) = (self.master_l[f], self.master_r[f]);
                let o = &mut output[(done + f) * out_channels..(done + f + 1) * out_channels];
                if out_channels == 1 {
                    o[0] = 0.5 * (l + r);
                } else {
                    o[0] = l;
                    o[1] = r;
                    for x in &mut o[2..] {
                        *x = 0.0;
                    }
                }
            }
            done += n;
        }
        self.telemetry.set_position(self.position);
    }

    /// Processa até MAX_BLOCK quadros para `master_l/r` (inclui o clique).
    fn process_block(&mut self, n: usize, input: &[f32], in_channels: usize) {
        for x in &mut self.click_l[..n] {
            *x = 0.0;
        }
        for x in &mut self.click_r[..n] {
            *x = 0.0;
        }
        if self.state == TransportState::CountIn {
            self.count_in_block(n, input, in_channels);
        } else if self.state.is_rolling() {
            // divisão no fim do loop (não em gravação)
            let mut offset = 0;
            while offset < n {
                let remaining = n - offset;
                let part = match self.graph.loop_range {
                    Some((_, le))
                        if self.state == TransportState::Playing
                            && self.position < le
                            && self.position + remaining as u64 > le =>
                    {
                        let p = (le - self.position) as usize;
                        if p == 0 {
                            remaining
                        } else {
                            p
                        }
                    }
                    _ => remaining,
                };
                self.render_part(
                    offset,
                    part,
                    &input[(offset * in_channels).min(input.len())..],
                    in_channels,
                    true,
                );
                self.position += part as u64;
                if let Some((ls, le)) = self.graph.loop_range {
                    if self.state == TransportState::Playing && self.position >= le {
                        self.position = ls;
                    }
                }
                offset += part;
            }
        } else {
            self.render_part(0, n, input, in_channels, false);
        }
        for f in 0..n {
            self.master_l[f] += self.click_l[f];
            self.master_r[f] += self.click_r[f];
        }
    }

    fn count_in_block(&mut self, n: usize, input: &[f32], in_channels: usize) {
        let vol = self.graph.metronome.volume;
        let take = (self.count_in_remaining as usize).min(n);
        let mut cursor = 0;
        for f in 0..take {
            if self.count_in_elapsed.is_multiple_of(self.count_in_interval) {
                self.click.render(
                    &mut self.click_l[cursor..f],
                    &mut self.click_r[cursor..f],
                    0,
                );
                cursor = f;
                let beat = self.count_in_elapsed / self.count_in_interval;
                self.click
                    .trigger(beat.is_multiple_of(self.count_in_beats_per_bar), vol);
            }
            self.count_in_elapsed += 1;
        }
        self.click.render(
            &mut self.click_l[cursor..take],
            &mut self.click_r[cursor..take],
            0,
        );
        self.count_in_remaining -= take as u64;
        // a timeline fica parada durante a contagem, mas a monitoração de entrada continua
        self.render_part(0, take, input, in_channels, false);
        if self.count_in_remaining == 0 {
            self.set_state(TransportState::Recording);
            let rest = n - take;
            if rest > 0 {
                let inp = &input[(take * in_channels).min(input.len())..];
                self.render_part(take, rest, inp, in_channels, true);
                self.position += rest as u64;
            }
        }
    }

    /// Renderiza `len` quadros a partir de `self.position` em master_l/r[offset..].
    fn render_part(
        &mut self,
        offset: usize,
        len: usize,
        input: &[f32],
        in_channels: usize,
        rolling: bool,
    ) {
        if len == 0 {
            return;
        }
        let pos = self.position;
        let g = &mut *self.graph;
        for x in &mut self.master_l[offset..offset + len] {
            *x = 0.0;
        }
        for x in &mut self.master_r[offset..offset + len] {
            *x = 0.0;
        }
        for b in &mut g.buses {
            b.buf_l[..len].fill(0.0);
            b.buf_r[..len].fill(0.0);
        }
        // faixas
        for t in &mut g.tracks {
            let s = &mut t.strip;
            s.buf_l[..len].fill(0.0);
            s.buf_r[..len].fill(0.0);
            if rolling {
                mix_clips(&t.clips, pos, len, &mut s.buf_l, &mut s.buf_r);
            }
            if let Some(m) = t.monitor {
                if in_channels > 0 {
                    for f in 0..len {
                        let base = f * in_channels;
                        let l = input.get(base + m.left).copied().unwrap_or(0.0);
                        let r = m
                            .right
                            .and_then(|r| input.get(base + r).copied())
                            .unwrap_or(l);
                        s.buf_l[f] += l;
                        s.buf_r[f] += r;
                    }
                }
            }
            run_inserts(s, len);
            // envios pré-fader
            for snd in &s.sends {
                if snd.pre_fader && s.audible {
                    let bus = &mut g.buses[snd.bus];
                    for f in 0..len {
                        bus.buf_l[f] += s.buf_l[f] * snd.gain;
                        bus.buf_r[f] += s.buf_r[f] * snd.gain;
                    }
                }
            }
            apply_fader(s, len);
            for snd in &s.sends {
                if !snd.pre_fader {
                    let bus = &mut g.buses[snd.bus];
                    for f in 0..len {
                        bus.buf_l[f] += s.buf_l[f] * snd.gain;
                        bus.buf_r[f] += s.buf_r[f] * snd.gain;
                    }
                }
            }
            meter(s, len, &self.telemetry);
            match s.dest {
                Destination::Master => {
                    for f in 0..len {
                        self.master_l[offset + f] += s.buf_l[f];
                        self.master_r[offset + f] += s.buf_r[f];
                    }
                }
                Destination::Bus(b) => {
                    let bus = &mut g.buses[b];
                    for f in 0..len {
                        bus.buf_l[f] += s.buf_l[f];
                        bus.buf_r[f] += s.buf_r[f];
                    }
                }
            }
        }
        // buses
        for b in &mut g.buses {
            run_inserts(b, len);
            apply_fader(b, len);
            meter(b, len, &self.telemetry);
            for f in 0..len {
                self.master_l[offset + f] += b.buf_l[f];
                self.master_r[offset + f] += b.buf_r[f];
            }
        }
        // master
        let m = &mut g.master;
        m.buf_l[..len].copy_from_slice(&self.master_l[offset..offset + len]);
        m.buf_r[..len].copy_from_slice(&self.master_r[offset..offset + len]);
        run_inserts(m, len);
        apply_fader(m, len);
        meter(m, len, &self.telemetry);
        self.master_l[offset..offset + len].copy_from_slice(&m.buf_l[..len]);
        self.master_r[offset..offset + len].copy_from_slice(&m.buf_r[..len]);

        // metrônomo da timeline
        if rolling && g.metronome.enabled {
            render_grid_clicks(
                g,
                &mut self.click,
                pos,
                offset,
                len,
                &mut self.click_l,
                &mut self.click_r,
            );
        } else if self.click.is_active() && self.state != TransportState::CountIn {
            self.click.render(
                &mut self.click_l[offset..offset + len],
                &mut self.click_r[offset..offset + len],
                0,
            );
        }

        // gravação
        if rolling && self.state == TransportState::Recording {
            let (in_a, in_b) = g.punch.unwrap_or((0, u64::MAX));
            let from = pos.max(in_a);
            let to = (pos + len as u64).min(in_b);
            if from < to {
                if !self.recording_started {
                    self.recording_started = true;
                    self.telemetry.set_record_start(Some(from));
                }
                if let Some(rec) = self.recorder.as_mut() {
                    let skip = (from - pos) as usize;
                    let frames = (to - from) as usize;
                    let a = (skip * in_channels).min(input.len());
                    let b = ((skip + frames) * in_channels).min(input.len());
                    let dropped =
                        rec.capture(&input[a..b], in_channels, (b - a) / in_channels.max(1));
                    if dropped > 0 {
                        self.telemetry.add_record_overflow(dropped);
                    }
                }
            }
        }
    }
}

fn mix_clips(clips: &[ClipRender], pos: u64, len: usize, out_l: &mut [f32], out_r: &mut [f32]) {
    let end = pos + len as u64;
    for c in clips {
        if c.start >= end {
            break; // ordenados por início
        }
        if c.end() <= pos {
            continue;
        }
        let from = c.start.max(pos);
        let to = c.end().min(end);
        let src = &c.source.channels;
        let Some(l) = src.first() else { continue };
        let r = src.get(1).unwrap_or(l);
        let src_start = (c.offset + (from - c.start)) as usize;
        let out_start = (from - pos) as usize;
        let n = (to - from) as usize;
        let (sl, sr) = (&l[src_start..src_start + n], &r[src_start..src_start + n]);
        for i in 0..n {
            out_l[out_start + i] += sl[i] * c.gain;
            out_r[out_start + i] += sr[i] * c.gain;
        }
    }
}

fn run_inserts(s: &mut StripRender, len: usize) {
    let (l, r) = (&mut s.buf_l[..len], &mut s.buf_r[..len]);
    for ins in &mut s.inserts {
        if ins.bypass {
            continue;
        }
        if let Some(p) = ins.processor.as_mut() {
            p.process(l, r);
        }
    }
    // proteção contra NaN/inf vindos de efeitos
    for x in l.iter_mut().chain(r.iter_mut()) {
        if !x.is_finite() {
            *x = 0.0;
        }
    }
}

fn apply_fader(s: &mut StripRender, len: usize) {
    let target = s.target_gains();
    if !s.state.initialized {
        s.state.current_gain = target;
        s.state.initialized = true;
    }
    let [cl, cr] = s.state.current_gain;
    let step_l = (target[0] - cl) / len as f32;
    let step_r = (target[1] - cr) / len as f32;
    for f in 0..len {
        let gl = cl + step_l * (f + 1) as f32;
        let gr = cr + step_r * (f + 1) as f32;
        s.buf_l[f] *= gl;
        s.buf_r[f] *= gr;
    }
    s.state.current_gain = target;
}

fn meter(s: &StripRender, len: usize, t: &Telemetry) {
    let mut pl = 0f32;
    let mut pr = 0f32;
    for f in 0..len {
        pl = pl.max(s.buf_l[f].abs());
        pr = pr.max(s.buf_r[f].abs());
    }
    t.push_peak(s.meter_slot, pl, pr);
}

fn render_grid_clicks(
    g: &RenderGraph,
    click: &mut Click,
    pos: u64,
    offset: usize,
    len: usize,
    out_l: &mut [f32],
    out_r: &mut [f32],
) {
    let sr = g.sample_rate;
    let t0 = g.tempo.seconds_to_ticks_f64(pos as f64 / sr as f64);
    let meter = g.tempo.meter_at(shared_kernel::Tick(t0.floor() as i64));
    let sub = g.metronome.subdivision.max(1) as i64;
    let step = (meter.beat_ticks() / sub).max(1);
    let rel = t0 - meter.tick.0 as f64;
    // começa um passo antes: um clique cujo tick cai no fim do bloco anterior mas cujo
    // quadro arredonda para este bloco não pode ser perdido. A pertença é decidida pelo quadro.
    let mut k = ((rel / step as f64).ceil() as i64 - 1).max(0);
    let mut cursor = offset;
    loop {
        let tick = meter.tick.0 + k * step;
        let frame = g.tempo.tick_to_frame(shared_kernel::Tick(tick), sr);
        if frame >= pos + len as u64 {
            break;
        }
        if frame >= pos {
            let at = offset + (frame - pos) as usize;
            // renderiza o clique anterior até este ponto
            click.render(&mut out_l[cursor..at], &mut out_r[cursor..at], 0);
            cursor = at;
            let in_bar = (tick - meter.tick.0) % meter.bar_ticks() == 0;
            let on_beat = (tick - meter.tick.0) % meter.beat_ticks() == 0;
            let accent = in_bar && g.metronome.accent_first_beat;
            let vol = if on_beat {
                g.metronome.volume
            } else {
                g.metronome.volume * 0.6
            };
            click.trigger(accent, vol);
        }
        k += 1;
    }
    click.render(
        &mut out_l[cursor..offset + len],
        &mut out_r[cursor..offset + len],
        0,
    );
}

/// Transfere estado e processadores do grafo antigo para o novo (por chave/id) e aplica parâmetros.
fn inherit(new: &mut RenderGraph, old: &mut RenderGraph) {
    let new_strips = new
        .tracks
        .iter_mut()
        .map(|t| &mut t.strip)
        .chain(new.buses.iter_mut())
        .chain(std::iter::once(&mut new.master));
    for ns in new_strips {
        let old_strip = old
            .tracks
            .iter_mut()
            .map(|t| &mut t.strip)
            .chain(old.buses.iter_mut())
            .chain(std::iter::once(&mut old.master))
            .find(|s| s.key == ns.key);
        if let Some(os) = old_strip {
            ns.state = os.state;
            for ins in &mut ns.inserts {
                if ins.processor.is_none() {
                    if let Some(oi) = os
                        .inserts
                        .iter_mut()
                        .find(|o| o.id == ins.id && o.type_tag == ins.type_tag)
                    {
                        ins.processor = oi.processor.take();
                    }
                }
            }
        }
        for ins in &mut ns.inserts {
            if let Some(p) = ins.processor.as_mut() {
                p.set_params(&ins.params);
            }
        }
    }
}
