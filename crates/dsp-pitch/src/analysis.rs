//! Análise offline: trilha de altura, segmentação em notas e render de correções nota a nota.

use crate::detector::{finite_or, PitchDetector, YinConfig, YinDetector};
use crate::shifter::{PitchShifter, PsolaConfig, PsolaShifter};
use crate::tuner::DEFAULT_A4_HZ;

/// Um quadro da trilha de altura.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchFrame {
    /// Amostra central do quadro.
    pub center: usize,
    pub frequency_hz: f32,
    pub confidence: f32,
    pub voiced: bool,
}

/// Nota detectada (intervalo em amostras, `end_frame` exclusivo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedNote {
    pub start_frame: usize,
    pub end_frame: usize,
    /// Altura média (MIDI fracionário).
    pub midi_float: f32,
    /// Desvio médio em cents em relação à nota inteira mais próxima.
    pub mean_cents: f32,
}

impl DetectedNote {
    /// Nota inteira mais próxima.
    pub fn nearest_note(&self) -> i32 {
        self.midi_float.round() as i32
    }

    pub fn len_frames(&self) -> usize {
        self.end_frame.saturating_sub(self.start_frame)
    }

    pub fn contains(&self, frame: usize) -> bool {
        (self.start_frame..self.end_frame).contains(&frame)
    }
}

/// Parâmetros da segmentação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteAnalysisConfig {
    pub yin: YinConfig,
    /// Salto entre quadros (amostras; 0 → 5 ms).
    pub hop: usize,
    pub min_confidence: f32,
    /// Duração mínima de uma nota (ms).
    pub min_note_ms: f32,
    /// Desvio (semitons) em relação à média da nota que indica mudança de nota.
    pub split_semitones: f32,
    /// Quadros consecutivos que confirmam a mudança (evita quebrar em vibrato/ruído).
    pub confirm_frames: usize,
    /// Quadros não vozeados tolerados dentro de uma nota.
    pub max_gap_frames: usize,
    pub a4_hz: f32,
}

impl Default for NoteAnalysisConfig {
    fn default() -> Self {
        Self {
            yin: YinConfig::default(),
            hop: 0,
            min_confidence: 0.7,
            min_note_ms: 60.0,
            split_semitones: 0.6,
            confirm_frames: 3,
            max_gap_frames: 1,
            a4_hz: DEFAULT_A4_HZ,
        }
    }
}

fn hop_or_default(hop: usize, sample_rate: f32) -> usize {
    if hop == 0 {
        ((sample_rate / 200.0) as usize).max(1)
    } else {
        hop
    }
}

/// Trilha de altura com quadros centrados em `0, hop, 2·hop, …` (bordas com zeros).
pub fn pitch_track(samples: &[f32], sample_rate: f32, hop: usize) -> Vec<PitchFrame> {
    track_with(samples, sample_rate, hop, YinConfig::default())
}

fn track_with(samples: &[f32], sample_rate: f32, hop: usize, yin: YinConfig) -> Vec<PitchFrame> {
    let mut det = YinDetector::new(yin, sample_rate);
    let hop = hop_or_default(hop, det.sample_rate());
    let n = det.frame_len();
    let mut frame = vec![0.0f32; n];
    let mut out = Vec::with_capacity(samples.len() / hop + 1);
    let mut center = 0usize;
    while center < samples.len() {
        let start = center as i64 - (n / 2) as i64;
        for (i, slot) in frame.iter_mut().enumerate() {
            let p = start + i as i64;
            *slot = if p >= 0 && (p as usize) < samples.len() {
                samples[p as usize]
            } else {
                0.0
            };
        }
        let e = det.detect(&frame);
        out.push(PitchFrame {
            center,
            frequency_hz: e.frequency_hz,
            confidence: e.confidence,
            voiced: e.voiced,
        });
        center += hop;
    }
    out
}

fn to_midi(f: &PitchFrame, cfg: &NoteAnalysisConfig) -> Option<f32> {
    (f.voiced && f.confidence >= cfg.min_confidence && f.frequency_hz > 0.0)
        .then(|| 69.0 + 12.0 * (f.frequency_hz / cfg.a4_hz).log2())
}

/// Segmenta notas com a configuração padrão.
pub fn analyze_notes(samples: &[f32], sample_rate: f32) -> Vec<DetectedNote> {
    analyze_notes_with(samples, sample_rate, &NoteAnalysisConfig::default())
}

struct Building {
    start: usize,
    end: usize,
    sum: f64,
    count: usize,
}

impl Building {
    fn mean(&self) -> f32 {
        (self.sum / self.count as f64) as f32
    }
}

/// Segmenta a trilha de altura em notas (mudança confirmada por `confirm_frames` quadros,
/// lacunas não vozeadas curtas toleradas, notas curtas descartadas).
pub fn analyze_notes_with(
    samples: &[f32],
    sample_rate: f32,
    cfg: &NoteAnalysisConfig,
) -> Vec<DetectedNote> {
    let sr = finite_or(sample_rate, 48_000.0).clamp(8_000.0, 384_000.0);
    let hop = hop_or_default(cfg.hop, sr);
    let track = track_with(samples, sr, hop, cfg.yin);
    let raw: Vec<Option<f32>> = track.iter().map(|f| to_midi(f, cfg)).collect();
    // Mediana de 5 quadros sobre os vozeados (remove saltos isolados de oitava).
    let midi: Vec<Option<f32>> = (0..raw.len())
        .map(|i| {
            raw[i]?;
            let mut w: Vec<f32> = raw[i.saturating_sub(2)..(i + 3).min(raw.len())]
                .iter()
                .flatten()
                .copied()
                .collect();
            w.sort_by(f32::total_cmp);
            Some(w[w.len() / 2])
        })
        .collect();

    let split = finite_or(cfg.split_semitones, 0.6).clamp(0.1, 6.0);
    let confirm = cfg.confirm_frames.max(1);
    let mut segments: Vec<Building> = Vec::new();
    let mut cur: Option<Building> = None;
    let mut gap = 0usize;
    for i in 0..midi.len() {
        let Some(m) = midi[i] else {
            if cur.is_some() {
                gap += 1;
                if gap > cfg.max_gap_frames {
                    segments.extend(cur.take());
                }
            }
            continue;
        };
        gap = 0;
        match cur.as_mut() {
            None => {
                cur = Some(Building {
                    start: i,
                    end: i,
                    sum: m as f64,
                    count: 1,
                })
            }
            Some(b) => {
                let reference = b.mean();
                if (m - reference).abs() < split {
                    b.end = i;
                    b.sum += m as f64;
                    b.count += 1;
                } else {
                    let confirmed = (i..(i + confirm).min(midi.len()))
                        .all(|j| midi[j].is_some_and(|v| (v - reference).abs() >= split))
                        && i + confirm <= midi.len();
                    if confirmed {
                        segments.extend(cur.take());
                        cur = Some(Building {
                            start: i,
                            end: i,
                            sum: m as f64,
                            count: 1,
                        });
                    } else {
                        b.end = i; // desvio passageiro: estende sem afetar a média
                    }
                }
            }
        }
    }
    segments.extend(cur);

    let min_len = (finite_or(cfg.min_note_ms, 60.0).max(0.0) * sr / 1000.0) as usize;
    segments
        .into_iter()
        .filter_map(|b| {
            let start_frame = track[b.start].center.saturating_sub(hop / 2);
            let end_frame = (track[b.end].center + hop / 2 + hop % 2).min(samples.len());
            let midi_float = b.mean();
            let note = DetectedNote {
                start_frame,
                end_frame,
                midi_float,
                mean_cents: (midi_float - midi_float.round()) * 100.0,
            };
            (note.len_frames() >= min_len.max(1)).then_some(note)
        })
        .collect()
}

/// Correção manual de uma nota detectada.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteCorrection {
    /// Índice em `notes`.
    pub note_index: usize,
    /// Deslocamento de altura em cents (ex.: −mean_cents centraliza a nota).
    pub offset_cents: f32,
    /// 0–1: quanto a deriva (vibrato/oscilação) é achatada em direção à média da nota.
    pub flatten: f32,
}

/// Renderiza as correções nota a nota. Retorna buffer do mesmo tamanho, alinhado no tempo
/// (a latência do shifter é compensada). Sem correções efetivas, devolve uma cópia.
pub fn render_note_corrections(
    samples: &[f32],
    sample_rate: f32,
    notes: &[DetectedNote],
    corrections: &[NoteCorrection],
) -> Vec<f32> {
    let effective: Vec<(DetectedNote, f32, f32)> = corrections
        .iter()
        .filter_map(|c| {
            let note = notes.get(c.note_index)?;
            let offset = finite_or(c.offset_cents, 0.0).clamp(-2400.0, 2400.0);
            let flatten = finite_or(c.flatten, 0.0).clamp(0.0, 1.0);
            (offset != 0.0 || flatten > 0.0).then_some((*note, offset, flatten))
        })
        .collect();
    if effective.is_empty() || samples.is_empty() {
        return samples.to_vec();
    }
    let sr = finite_or(sample_rate, 48_000.0).clamp(8_000.0, 384_000.0);
    let cfg = NoteAnalysisConfig::default();
    let hop = hop_or_default(0, sr);
    let track = pitch_track(samples, sr, hop);

    // Cents de correção por quadro (fora das notas: 0).
    let frame_cents: Vec<f32> = track
        .iter()
        .map(|f| {
            effective
                .iter()
                .find(|(n, _, _)| n.contains(f.center))
                .map_or(0.0, |(n, offset, flatten)| {
                    let drift = to_midi(f, &cfg)
                        .map_or(0.0, |inst| (n.midi_float - inst) * 100.0 * flatten);
                    offset + drift
                })
        })
        .collect();
    // Interpolação linear por amostra entre centros de quadro.
    let ratios: Vec<f32> = (0..samples.len())
        .map(|i| {
            let pos = i as f32 / hop as f32;
            let k = pos.floor() as usize;
            let t = pos - k as f32;
            let a = frame_cents.get(k).copied().unwrap_or(0.0);
            let b = frame_cents.get(k + 1).copied().unwrap_or(a);
            2f32.powf((a + (b - a) * t) / 1200.0)
        })
        .collect();

    let mut shifter = PsolaShifter::new(PsolaConfig::default(), sr);
    let latency = shifter.latency_samples();
    let last = ratios.len() - 1;
    let ratio_at = |pos: f64| ratios[(pos.max(0.0).round() as usize).min(last)];
    let mut out = Vec::with_capacity(samples.len());
    for i in 0..samples.len() + latency {
        let x = samples.get(i).copied().unwrap_or(0.0);
        let y = shifter.process_sample_with(x, ratio_at);
        if i >= latency {
            out.push(y);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::cents_between;
    use crate::signal;

    const SR: f32 = 48_000.0;

    fn hz(midi: f32) -> f32 {
        440.0 * 2f32.powf((midi - 69.0) / 12.0)
    }

    #[test]
    fn pitch_track_has_one_frame_per_hop() {
        let sig = signal::sine(220.0, SR, 4800, 0.5);
        let t = pitch_track(&sig, SR, 480);
        assert_eq!(t.len(), 10);
        assert_eq!(t[3].center, 1440);
        assert!(cents_between(220.0, t[5].frequency_hz).abs() < 5.0);
    }

    #[test]
    fn segments_a_synthetic_melody() {
        let d = (0.3 * SR) as usize;
        let sig = signal::melody(
            &[
                (hz(57.0), d),
                (hz(60.0), d),
                (0.0, d / 3),
                (hz(64.0 + 0.2), d), // E4 20 cents agudo
                (hz(67.0), d),
            ],
            SR,
            0.5,
        );
        let notes = analyze_notes(&sig, SR);
        let found: Vec<i32> = notes.iter().map(|n| n.nearest_note()).collect();
        assert_eq!(found, vec![57, 60, 64, 67], "{notes:?}");
        let starts = [0, d, 2 * d + d / 3, 3 * d + d / 3];
        for (n, s) in notes.iter().zip(starts) {
            assert!(
                (n.start_frame as i64 - s as i64).abs() < (0.03 * SR) as i64,
                "início {} vs {s}",
                n.start_frame
            );
            assert!(n.len_frames() as f32 > 0.2 * SR);
        }
        assert!(
            (notes[2].mean_cents - 20.0).abs() < 3.0,
            "{}",
            notes[2].mean_cents
        );
        assert!(notes[0].mean_cents.abs() < 3.0);
    }

    #[test]
    fn noise_and_silence_yield_no_notes() {
        assert!(analyze_notes(&signal::noise(24_000, 0.3, 9), SR).is_empty());
        assert!(analyze_notes(&vec![0.0; 24_000], SR).is_empty());
        assert!(analyze_notes(&[], SR).is_empty());
    }

    #[test]
    fn renders_per_note_offsets() {
        let d = (0.5 * SR) as usize;
        let sig = signal::melody(&[(hz(60.3), d), (hz(64.0), d)], SR, 0.5);
        let notes = analyze_notes(&sig, SR);
        assert_eq!(notes.len(), 2);
        // Centraliza a 1ª (−30 cents) e sobe a 2ª um semitom.
        let fixes = [
            NoteCorrection {
                note_index: 0,
                offset_cents: -notes[0].mean_cents,
                flatten: 0.0,
            },
            NoteCorrection {
                note_index: 1,
                offset_cents: 100.0,
                flatten: 0.0,
            },
        ];
        let out = render_note_corrections(&sig, SR, &notes, &fixes);
        assert_eq!(out.len(), sig.len());
        assert!(out.iter().all(|v| v.is_finite()));
        let track = pitch_track(&out, SR, 0);
        let at = |sample: usize| track[sample / 240].frequency_hz;
        assert!(
            cents_between(hz(60.0), at(d / 2)).abs() < 5.0,
            "{}",
            at(d / 2)
        );
        assert!(cents_between(hz(65.0), at(d + d / 2)).abs() < 5.0);
    }

    #[test]
    fn flatten_removes_vibrato() {
        // C4 com vibrato de ±40 cents a 5 Hz.
        let n = 48_000;
        let mut phase = 0.0f64;
        let sig: Vec<f32> = (0..n)
            .map(|i| {
                let cents = 40.0 * (std::f64::consts::TAU * 5.0 * i as f64 / SR as f64).sin();
                let f = hz(60.0) as f64 * 2f64.powf(cents / 1200.0);
                phase += std::f64::consts::TAU * f / SR as f64;
                0.5 * phase.sin() as f32
            })
            .collect();
        let notes = analyze_notes(&sig, SR);
        assert_eq!(notes.len(), 1, "vibrato não quebra a nota");
        let fix = [NoteCorrection {
            note_index: 0,
            offset_cents: 0.0,
            flatten: 1.0,
        }];
        let out = render_note_corrections(&sig, SR, &notes, &fix);
        let spread = |s: &[f32]| {
            let t = pitch_track(s, SR, 0);
            let c: Vec<f32> = t[40..160]
                .iter()
                .map(|f| cents_between(hz(60.0), f.frequency_hz))
                .collect();
            c.iter().cloned().fold(f32::MIN, f32::max) - c.iter().cloned().fold(f32::MAX, f32::min)
        };
        let before = spread(&sig);
        let after = spread(&out);
        assert!(before > 60.0, "vibrato original {before}");
        assert!(after < before * 0.4, "achatado: {before} → {after}");
    }

    #[test]
    fn no_effective_correction_returns_copy() {
        let sig = signal::sine(220.0, SR, 1000, 0.5);
        let notes = [DetectedNote {
            start_frame: 0,
            end_frame: 1000,
            midi_float: 57.0,
            mean_cents: 0.0,
        }];
        let out = render_note_corrections(
            &sig,
            SR,
            &notes,
            &[NoteCorrection {
                note_index: 5,
                offset_cents: 100.0,
                flatten: 0.0,
            }],
        );
        assert_eq!(out, sig);
    }
}
