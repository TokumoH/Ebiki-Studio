//! Gerador de clique do metrônomo com disparo sample-accurate.
//!
//! O transporte (engine) decide **quando** há batida e chama [`Metronome::trigger`] com o
//! deslocamento dentro do próximo bloco; o `process` **soma** o clique aos buffers (o nó do
//! metrônomo normalmente recebe buffers zerados). Clique = seno com envelope exponencial
//! (ataque de 0,5 ms para evitar estalo), tom agudo para acento (1,6 kHz) e normal (1 kHz).
//! Disparos ficam numa fila fixa (sem alocação); excedentes são descartados.

use crate::smoothing::GainSmoother;
use crate::util::{clamp_param, db_to_lin, safe_sr};
use crate::Processor;
use std::f32::consts::TAU;

pub const ACCENT_FREQ_HZ: f32 = 1_600.0;
pub const NORMAL_FREQ_HZ: f32 = 1_000.0;
/// Duração do clique.
pub const CLICK_MS: f32 = 40.0;
const ATTACK_MS: f32 = 0.5;
/// Acento é 4 dB mais forte que a batida normal.
const ACCENT_GAIN: f32 = 1.0;
const NORMAL_GAIN: f32 = 0.63;
const MAX_PENDING: usize = 8;

#[derive(Debug, Clone, Copy, Default)]
struct Pending {
    offset: usize,
    accent: bool,
}

#[derive(Debug, Clone)]
pub struct Metronome {
    sample_rate: f32,
    volume: GainSmoother,
    pending: [Pending; MAX_PENDING],
    pending_len: usize,
    /// Clique em andamento.
    active: bool,
    pos: usize,
    len: usize,
    attack: usize,
    phase: f32,
    phase_inc: f32,
    amp: f32,
    decay: f32,
    env: f32,
}

impl Metronome {
    pub fn new() -> Self {
        let mut m = Self {
            sample_rate: 48_000.0,
            volume: GainSmoother::new(db_to_lin(-6.0)),
            pending: [Pending::default(); MAX_PENDING],
            pending_len: 0,
            active: false,
            pos: 0,
            len: 0,
            attack: 1,
            phase: 0.0,
            phase_inc: 0.0,
            amp: 0.0,
            decay: 0.0,
            env: 0.0,
        };
        m.prepare(48_000.0, 0);
        m
    }

    /// Volume do clique em dB (−60..+6), suavizado.
    pub fn set_volume_db(&mut self, db: f32) {
        self.volume
            .set_target(db_to_lin(clamp_param(db, -60.0, 6.0, -6.0)));
    }

    /// Agenda um clique `offset` amostras após o início do **próximo** `process`. Se o offset
    /// passar do tamanho do bloco, o clique é adiado para os blocos seguintes. Retorna `false`
    /// se a fila (8) estiver cheia.
    pub fn trigger(&mut self, offset: usize, accent: bool) -> bool {
        if self.pending_len >= MAX_PENDING {
            return false;
        }
        self.pending[self.pending_len] = Pending { offset, accent };
        self.pending_len += 1;
        true
    }

    /// Quantidade de disparos aguardando.
    pub fn pending(&self) -> usize {
        self.pending_len
    }

    fn start_click(&mut self, accent: bool) {
        self.active = true;
        self.pos = 0;
        self.phase = 0.0;
        let f = if accent {
            ACCENT_FREQ_HZ
        } else {
            NORMAL_FREQ_HZ
        };
        self.phase_inc = TAU * f / self.sample_rate;
        self.amp = if accent { ACCENT_GAIN } else { NORMAL_GAIN };
        self.env = 1.0;
    }

    #[inline]
    fn next_click_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let a = if self.pos < self.attack {
            self.pos as f32 / self.attack as f32
        } else {
            1.0
        };
        let s = self.amp * a * self.env * self.phase.sin();
        self.env *= self.decay;
        self.phase += self.phase_inc;
        if self.phase >= TAU {
            self.phase -= TAU;
        }
        self.pos += 1;
        if self.pos >= self.len {
            self.active = false;
        }
        s
    }
}

impl Default for Metronome {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Metronome {
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        self.sample_rate = safe_sr(sample_rate);
        self.len = ((CLICK_MS * 0.001 * self.sample_rate) as usize).max(1);
        self.attack = ((ATTACK_MS * 0.001 * self.sample_rate) as usize).max(1);
        // Decai ~60 dB ao longo do clique.
        self.decay = (0.001f32.ln() / self.len as f32).exp();
        self.volume.prepare(self.sample_rate, max_block);
        self.reset();
    }

    fn reset(&mut self) {
        self.pending_len = 0;
        self.active = false;
        self.volume.snap();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        for i in 0..n {
            // Dispara os cliques agendados para esta amostra (o mais recente prevalece).
            let mut k = 0;
            while k < self.pending_len {
                if self.pending[k].offset == i {
                    let accent = self.pending[k].accent;
                    self.start_click(accent);
                    self.pending_len -= 1;
                    self.pending[k] = self.pending[self.pending_len];
                } else {
                    k += 1;
                }
            }
            let s = self.next_click_sample() * self.volume.next_value();
            left[i] += s;
            right[i] += s;
        }
        // Adia o que não coube neste bloco.
        for p in &mut self.pending[..self.pending_len] {
            p.offset = p.offset.saturating_sub(n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::peak;

    const SR: f32 = 48_000.0;

    fn metro() -> Metronome {
        let mut m = Metronome::new();
        m.prepare(SR, 128);
        m.set_volume_db(0.0);
        m.reset();
        m
    }

    #[test]
    fn click_starts_at_exact_offset() {
        let mut m = metro();
        let mut l = vec![0.0f32; 128];
        let mut r = vec![0.0f32; 128];
        assert!(m.trigger(37, false));
        m.process(&mut l, &mut r);
        assert!(l[..37].iter().all(|v| *v == 0.0));
        // Primeira amostra do clique: sen(0)·0 = 0; a segunda já é não nula.
        assert!(l[38] != 0.0);
        assert_eq!(l, r);
    }

    #[test]
    fn offset_beyond_block_is_carried_over() {
        let mut m = metro();
        m.trigger(200, true);
        let mut l = vec![0.0f32; 128];
        let mut r = vec![0.0f32; 128];
        m.process(&mut l, &mut r);
        assert_eq!(peak(&l), 0.0);
        assert_eq!(m.pending(), 1);
        let mut l = vec![0.0f32; 128];
        let mut r = vec![0.0f32; 128];
        m.process(&mut l, &mut r);
        assert!(l[..72].iter().all(|v| *v == 0.0));
        assert!(l[73] != 0.0, "200 − 128 = 72");
        assert_eq!(m.pending(), 0);
    }

    #[test]
    fn accent_is_louder_and_higher_and_click_ends() {
        let render = |accent: bool| {
            let mut m = metro();
            m.trigger(0, accent);
            let mut l = vec![0.0f32; 4_800];
            let mut r = vec![0.0f32; 4_800];
            m.process(&mut l, &mut r);
            l
        };
        let a = render(true);
        let n = render(false);
        assert!(peak(&a) > peak(&n) * 1.4);
        let zero_crossings = |x: &[f32]| {
            x[..1_000]
                .windows(2)
                .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
                .count()
        };
        assert!(zero_crossings(&a) > zero_crossings(&n));
        // Clique dura 40 ms (1920 amostras) e depois silêncio.
        assert!(a[1_920..].iter().all(|v| *v == 0.0));
        assert!(peak(&a) <= 1.0);
    }

    #[test]
    fn click_is_added_to_existing_signal_and_queue_is_bounded() {
        let mut m = metro();
        for _ in 0..MAX_PENDING {
            assert!(m.trigger(10_000, false));
        }
        assert!(!m.trigger(1, false), "fila cheia");
        m.reset();
        m.trigger(0, false);
        let mut l = vec![0.5f32; 64];
        let mut r = vec![0.5f32; 64];
        m.process(&mut l, &mut r);
        assert_eq!(l[0], 0.5);
        assert!(l[10] != 0.5);
    }
}
