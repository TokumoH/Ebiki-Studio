//! Sintetizador offline para audição de progressões, bases geradas e partituras.
//!
//! [`SimpleSynth`] não depende de arquivos externos (funciona sempre); timbres aproximados por família
//! General MIDI e bateria sintetizada no canal 10 (índice 9).

use shared_kernel::{TempoMap, Tick};

/// Nota a renderizar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteEvent {
    pub start_tick: i64,
    pub duration_ticks: i64,
    pub channel: u8,
    pub program: u8,
    pub key: u8,
    pub velocity: u8,
}

/// Limite de duração renderizada (proteção).
pub const MAX_SECONDS: f64 = 30.0 * 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Timbre {
    Piano,
    Organ,
    Guitar,
    Bass,
    Strings,
    Lead,
}

fn timbre(program: u8) -> Timbre {
    match program {
        0..=15 => Timbre::Piano,
        16..=23 => Timbre::Organ,
        24..=31 => Timbre::Guitar,
        32..=39 => Timbre::Bass,
        40..=55 | 88..=103 => Timbre::Strings,
        _ => Timbre::Lead,
    }
}

/// Gerador de ruído determinístico (xorshift).
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleSynth;

impl SimpleSynth {
    /// Renderiza em estéreo (L, R) na taxa `sr`. Retorna erro se exceder [`MAX_SECONDS`].
    pub fn render(
        &self,
        events: &[NoteEvent],
        tempo: &TempoMap,
        sr: u32,
        tail_seconds: f64,
    ) -> Result<[Vec<f32>; 2], String> {
        let srf = sr as f64;
        let end_tick = events
            .iter()
            .map(|e| e.start_tick.max(0) + e.duration_ticks.max(0))
            .max()
            .unwrap_or(0);
        let seconds = tempo.tick_to_seconds(Tick(end_tick)) + tail_seconds.max(0.0);
        if seconds > MAX_SECONDS {
            return Err("a renderização excede 30 minutos".into());
        }
        let total = (seconds * srf).ceil() as usize;
        let mut l = vec![0.0f32; total];
        let mut r = vec![0.0f32; total];
        let mut noise = Noise(0x9E37_79B9);
        for e in events {
            if e.duration_ticks <= 0 || e.start_tick < 0 {
                continue;
            }
            let start = tempo.tick_to_frame(Tick(e.start_tick), sr) as usize;
            let end = tempo.tick_to_frame(Tick(e.start_tick + e.duration_ticks), sr) as usize;
            let vel = (e.velocity.min(127) as f32 / 127.0).powf(1.5) * 0.25;
            if e.channel == 9 {
                drum(e.key, start, vel, sr, &mut l, &mut r, &mut noise);
                continue;
            }
            let t = timbre(e.program);
            let freq = 440.0 * 2f64.powf((e.key as f64 - 69.0) / 12.0);
            let release = (0.15 * srf) as usize;
            let stop = (end + release).min(total);
            // pan leve por canal para separar instrumentos
            let pan = ((e.channel as f32 * 0.37).sin() * 0.3).clamp(-0.5, 0.5);
            let (gl, gr) = (1.0 - pan.max(0.0), 1.0 + pan.min(0.0));
            for (i, idx) in (start..stop).enumerate() {
                let ts = i as f64 / srf;
                let phase = std::f64::consts::TAU * freq * ts;
                let env = envelope(t, i, end.saturating_sub(start), release, srf);
                let s = match t {
                    Timbre::Piano | Timbre::Guitar => {
                        (phase.sin() + 0.5 * (2.0 * phase).sin() + 0.25 * (3.0 * phase).sin())
                            / 1.75
                    }
                    Timbre::Organ => {
                        (phase.sin() + 0.6 * (2.0 * phase).sin() + 0.4 * (4.0 * phase).sin()) / 2.0
                    }
                    Timbre::Bass => (phase.sin() + 0.3 * (2.0 * phase).sin()) / 1.3,
                    Timbre::Strings => {
                        let vib = (std::f64::consts::TAU * 5.0 * ts).sin() * 0.003;
                        let p = phase * (1.0 + vib);
                        (p.sin()
                            + 0.5 * (2.0 * p).sin()
                            + 0.33 * (3.0 * p).sin()
                            + 0.25 * (4.0 * p).sin())
                            / 2.08
                    }
                    Timbre::Lead => {
                        (phase.sin() + 0.33 * (3.0 * phase).sin() + 0.2 * (5.0 * phase).sin())
                            / 1.53
                    }
                } as f32;
                let v = s * env * vel;
                l[idx] += v * gl;
                r[idx] += v * gr;
            }
        }
        Ok([l, r])
    }
}

fn envelope(t: Timbre, i: usize, held: usize, release: usize, sr: f64) -> f32 {
    let ts = i as f64 / sr;
    let attack = match t {
        Timbre::Strings => 0.08,
        Timbre::Organ => 0.01,
        _ => 0.004,
    };
    let a = (ts / attack).min(1.0);
    let sustain = match t {
        Timbre::Piano | Timbre::Guitar => (-ts * 2.5).exp(),
        Timbre::Bass => (-ts * 1.2).exp().max(0.4),
        _ => 1.0,
    };
    let rel = if i >= held {
        1.0 - ((i - held) as f64 / release.max(1) as f64).min(1.0)
    } else {
        1.0
    };
    (a * sustain * rel) as f32
}

fn drum(key: u8, start: usize, vel: f32, sr: u32, l: &mut [f32], r: &mut [f32], noise: &mut Noise) {
    let srf = sr as f32;
    let (len_s, kind) = match key {
        35 | 36 => (0.35, 0),
        37..=40 => (0.2, 1),
        42 | 44 | 46 | 51 | 59 => (if key == 46 { 0.3 } else { 0.06 }, 2),
        49 | 57 | 52 | 55 => (1.2, 2),
        _ => (0.15, 1),
    };
    let n = (len_s * srf) as usize;
    let mut phase = 0.0f32;
    for i in 0..n {
        let idx = start + i;
        if idx >= l.len() {
            break;
        }
        let t = i as f32 / srf;
        let env = (-t / (len_s * 0.3)).exp();
        let s = match kind {
            0 => {
                let f = 50.0 + 100.0 * (-t * 30.0).exp();
                phase += std::f32::consts::TAU * f / srf;
                phase.sin() * 1.6
            }
            1 => noise.next() * 0.7 + (std::f32::consts::TAU * 190.0 * t).sin() * 0.4,
            _ => {
                let a = noise.next();
                let b = noise.next();
                (a - b) * 0.35
            }
        };
        let v = s * env * vel;
        l[idx] += v;
        r[idx] += v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(start: i64, dur: i64, ch: u8, prog: u8, key: u8) -> NoteEvent {
        NoteEvent {
            start_tick: start,
            duration_ticks: dur,
            channel: ch,
            program: prog,
            key,
            velocity: 100,
        }
    }

    #[test]
    fn renders_note_at_right_time_and_length() {
        let tempo = TempoMap::default(); // 120 BPM: 1 semínima = 0,5 s
        let [l, _] = SimpleSynth
            .render(&[ev(960, 960, 0, 0, 69)], &tempo, 48_000, 0.5)
            .unwrap();
        assert_eq!(l.len(), 48_000 + 24_000, "fim da nota (1 s) + cauda 0,5 s");
        assert!(
            l[..24_000].iter().all(|x| *x == 0.0),
            "silêncio antes do início"
        );
        assert!(l[24_000..30_000].iter().any(|x| x.abs() > 0.01));
        assert!(l.iter().all(|x| x.is_finite() && x.abs() < 1.0));
    }

    #[test]
    fn every_program_and_drum_is_bounded() {
        let tempo = TempoMap::default();
        let mut evs: Vec<_> = (0..128u8).map(|p| ev(0, 480, 0, p, 60)).collect();
        evs.extend(
            [35u8, 38, 42, 46, 49, 60]
                .iter()
                .map(|k| ev(0, 240, 9, 0, *k)),
        );
        let [l, r] = SimpleSynth.render(&evs[..8], &tempo, 44_100, 0.2).unwrap();
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        for e in &evs {
            let [a, _] = SimpleSynth
                .render(std::slice::from_ref(e), &tempo, 44_100, 0.2)
                .unwrap();
            assert!(
                a.iter().any(|x| x.abs() > 1e-4),
                "programa {} tecla {} produz som",
                e.program,
                e.key
            );
        }
    }

    #[test]
    fn rejects_absurd_length() {
        let tempo = TempoMap::default();
        assert!(SimpleSynth
            .render(&[ev(0, 960 * 2 * 60 * 60, 0, 0, 60)], &tempo, 48_000, 0.0)
            .is_err());
    }
}
