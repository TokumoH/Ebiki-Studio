//! Renderização com SoundFont 2 (`.sf2`) via `rustysynth` (Rust puro, licença MIT — ADR-0025).
//!
//! - [`SoundFontSynth::load`] lê e valida o arquivo uma vez; o banco fica compartilhado (`Arc`) e
//!   cada renderização cria um sintetizador novo (renderizações independentes e determinísticas).
//! - Arquivo ausente, corrompido ou que faria a biblioteca entrar em pânico vira [`SynthError`]
//!   tipado; nada aqui entra em pânico.

use crate::{NoteEvent, MAX_SECONDS};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use shared_kernel::{TempoMap, Tick};
use std::io::BufReader;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Tamanho máximo aceito para um SoundFont (proteção contra arquivos absurdos).
pub const MAX_SOUNDFONT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const PERCUSSION_CHANNEL: u8 = 9;

/// Erros do sintetizador (código estável + mensagem PT-BR).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SynthError {
    #[error("não foi possível ler o arquivo SoundFont: {0}")]
    Io(String),
    #[error("arquivo SoundFont inválido: {0}")]
    InvalidSoundFont(String),
    #[error("taxa de amostragem não suportada pelo SoundFont: {0} Hz")]
    UnsupportedSampleRate(u32),
    #[error("a renderização excede 30 minutos")]
    TooLong,
}

impl SynthError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "synth.io",
            Self::InvalidSoundFont(_) => "synth.invalid_soundfont",
            Self::UnsupportedSampleRate(_) => "synth.unsupported_rate",
            Self::TooLong => "synth.too_long",
        }
    }
}

/// Sintetizador General MIDI baseado em um arquivo SoundFont 2.
#[derive(Debug, Clone)]
pub struct SoundFontSynth {
    path: PathBuf,
    font: Arc<SoundFont>,
}

/// Mensagem MIDI ordenável por quadro: nota desligada (0) antes de troca de programa (1) antes de
/// nota ligada (2) no mesmo quadro.
#[derive(Debug, Clone, Copy)]
struct Msg {
    frame: usize,
    order: u8,
    channel: i32,
    a: i32,
    b: i32,
}

impl SoundFontSynth {
    /// Carrega e valida o arquivo. Nunca entra em pânico.
    pub fn load(path: &Path) -> Result<Self, SynthError> {
        let meta = std::fs::metadata(path).map_err(|e| SynthError::Io(e.to_string()))?;
        if !meta.is_file() {
            return Err(SynthError::Io("o caminho não é um arquivo".into()));
        }
        if meta.len() > MAX_SOUNDFONT_BYTES {
            return Err(SynthError::InvalidSoundFont(
                "arquivo grande demais (máx. 2 GiB)".into(),
            ));
        }
        let file = std::fs::File::open(path).map_err(|e| SynthError::Io(e.to_string()))?;
        let mut reader = BufReader::new(file);
        let parsed = catch_unwind(AssertUnwindSafe(|| SoundFont::new(&mut reader)))
            .map_err(|_| SynthError::InvalidSoundFont("estrutura interna inconsistente".into()))?;
        let font = parsed.map_err(|e| SynthError::InvalidSoundFont(e.to_string()))?;
        if font.get_presets().is_empty() {
            return Err(SynthError::InvalidSoundFont("nenhum preset".into()));
        }
        Ok(Self {
            path: path.to_path_buf(),
            font: Arc::new(font),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nome do banco declarado no arquivo (pode ser vazio).
    pub fn bank_name(&self) -> &str {
        self.font.get_info().get_bank_name()
    }

    pub fn preset_count(&self) -> usize {
        self.font.get_presets().len()
    }

    /// Renderiza em estéreo (L, R) na taxa `sr` (mesmo contrato de [`crate::SimpleSynth::render`]).
    pub fn render(
        &self,
        events: &[NoteEvent],
        tempo: &TempoMap,
        sr: u32,
        tail_seconds: f64,
    ) -> Result<[Vec<f32>; 2], SynthError> {
        let end_tick = events
            .iter()
            .map(|e| e.start_tick.max(0) + e.duration_ticks.max(0))
            .max()
            .unwrap_or(0);
        let tail = if tail_seconds.is_finite() {
            tail_seconds.max(0.0)
        } else {
            0.0
        };
        let seconds = tempo.tick_to_seconds(Tick(end_tick)) + tail;
        if seconds > MAX_SECONDS {
            return Err(SynthError::TooLong);
        }
        let rate = i32::try_from(sr).map_err(|_| SynthError::UnsupportedSampleRate(sr))?;
        let mut settings = SynthesizerSettings::new(rate);
        settings.maximum_polyphony = 128;
        let mut synth = Synthesizer::new(&self.font, &settings)
            .map_err(|_| SynthError::UnsupportedSampleRate(sr))?;
        let total = (seconds * sr as f64).ceil() as usize;
        let msgs = schedule(events, tempo, sr, total);
        let mut l = vec![0.0f32; total];
        let mut r = vec![0.0f32; total];
        let rendered = catch_unwind(AssertUnwindSafe(|| {
            let mut cursor = 0usize;
            for m in &msgs {
                if m.frame > cursor {
                    synth.render(&mut l[cursor..m.frame], &mut r[cursor..m.frame]);
                    cursor = m.frame;
                }
                match m.order {
                    0 => synth.note_off(m.channel, m.a),
                    1 => synth.process_midi_message(m.channel, 0xC0, m.a, 0),
                    _ => synth.note_on(m.channel, m.a, m.b),
                }
            }
            if cursor < total {
                synth.render(&mut l[cursor..], &mut r[cursor..]);
            }
        }));
        if rendered.is_err() {
            return Err(SynthError::InvalidSoundFont(
                "falha interna ao sintetizar".into(),
            ));
        }
        for x in l.iter_mut().chain(r.iter_mut()) {
            if !x.is_finite() {
                *x = 0.0;
            }
        }
        Ok([l, r])
    }
}

/// Converte notas em mensagens MIDI ordenadas por quadro (trocas de programa por canal incluídas).
fn schedule(events: &[NoteEvent], tempo: &TempoMap, sr: u32, total: usize) -> Vec<Msg> {
    let mut sorted: Vec<&NoteEvent> = events
        .iter()
        .filter(|e| e.duration_ticks > 0 && e.start_tick >= 0)
        .collect();
    sorted.sort_by_key(|e| (e.start_tick, e.channel, e.key));
    let mut program: [Option<u8>; 16] = [None; 16];
    let mut msgs = Vec::with_capacity(sorted.len() * 2 + 16);
    for e in sorted {
        let ch = e.channel & 0x0F;
        let key = e.key.min(127) as i32;
        let start = (tempo.tick_to_frame(Tick(e.start_tick), sr) as usize).min(total);
        let end = (tempo.tick_to_frame(Tick(e.start_tick + e.duration_ticks), sr) as usize)
            .min(total)
            .max(start);
        let prog = e.program.min(127);
        if ch != PERCUSSION_CHANNEL && program[ch as usize] != Some(prog) {
            program[ch as usize] = Some(prog);
            msgs.push(Msg {
                frame: start,
                order: 1,
                channel: ch as i32,
                a: prog as i32,
                b: 0,
            });
        }
        msgs.push(Msg {
            frame: start,
            order: 2,
            channel: ch as i32,
            a: key,
            b: e.velocity.clamp(1, 127) as i32,
        });
        msgs.push(Msg {
            frame: end,
            order: 0,
            channel: ch as i32,
            a: key,
            b: 0,
        });
    }
    // estável: preserva a ordem de inserção (troca de programa antes da nota que a pediu)
    msgs.sort_by_key(|m| (m.frame, m.order));
    msgs
}

/// Apoio a testes de outros crates (não faz parte da API estável).
#[doc(hidden)]
pub mod test_support {
    /// Gera um SoundFont 2 mínimo e válido (um preset GM 0, um instrumento, uma senoide de 440 Hz
    /// em laço). Criado pelo próprio teste: sem arquivos de terceiros.
    pub fn tiny_sf2() -> Vec<u8> {
        fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut v = id.to_vec();
            v.extend((data.len() as u32).to_le_bytes());
            v.extend(data);
            if data.len() % 2 == 1 {
                v.push(0);
            }
            v
        }
        fn list(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
            let mut body = kind.to_vec();
            for c in chunks {
                body.extend(c);
            }
            chunk(b"LIST", &body)
        }
        fn name20(s: &str) -> Vec<u8> {
            let mut v = s.as_bytes().to_vec();
            v.resize(20, 0);
            v
        }
        let u16le = |x: u16| x.to_le_bytes().to_vec();
        let u32le = |x: u32| x.to_le_bytes().to_vec();

        // amostra: 100 períodos de 440 Hz a 44 100 Hz (≈ 100,2 amostras/período) + 46 zeros
        let n = 10_022usize;
        let mut smpl = Vec::new();
        for i in 0..n {
            let x = (std::f64::consts::TAU * 440.0 * i as f64 / 44_100.0).sin() * 16_000.0;
            smpl.extend((x as i16).to_le_bytes());
        }
        smpl.extend(vec![0u8; 46 * 2]);

        let info = list(
            b"INFO",
            &[
                chunk(b"ifil", &[u16le(2), u16le(1)].concat()),
                chunk(b"isng", b"EMU8000\0"),
                chunk(b"INAM", b"Teste\0"),
            ],
        );
        let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);
        let phdr = [
            name20("Seno"),
            u16le(0), // preset
            u16le(0), // banco
            u16le(0), // índice do pbag
            u32le(0),
            u32le(0),
            u32le(0),
            name20("EOP"),
            u16le(0),
            u16le(0),
            u16le(1),
            u32le(0),
            u32le(0),
            u32le(0),
        ]
        .concat();
        let pbag = [u16le(0), u16le(0), u16le(1), u16le(0)].concat();
        let pmod = vec![0u8; 10];
        // gerador 41 = instrumento 0; depois o terminador
        let pgen = [u16le(41), u16le(0), u16le(0), u16le(0)].concat();
        let inst = [name20("Seno"), u16le(0), name20("EOI"), u16le(1)].concat();
        let ibag = [u16le(0), u16le(0), u16le(2), u16le(0)].concat();
        let imod = vec![0u8; 10];
        // 54 = sampleModes (1 = laço contínuo); 53 = sampleID (sempre o último)
        let igen = [u16le(54), u16le(1), u16le(53), u16le(0), u16le(0), u16le(0)].concat();
        let shdr = [
            name20("Seno"),
            u32le(0),
            u32le(n as u32),
            u32le(100),
            u32le(n as u32 - 1_000),
            u32le(44_100),
            vec![69u8, 0],
            u16le(0),
            u16le(1),
            name20("EOS"),
            u32le(0),
            u32le(0),
            u32le(0),
            u32le(0),
            u32le(0),
            vec![0u8, 0],
            u16le(0),
            u16le(0),
        ]
        .concat();
        let pdta = list(
            b"pdta",
            &[
                chunk(b"phdr", &phdr),
                chunk(b"pbag", &pbag),
                chunk(b"pmod", &pmod),
                chunk(b"pgen", &pgen),
                chunk(b"inst", &inst),
                chunk(b"ibag", &ibag),
                chunk(b"imod", &imod),
                chunk(b"igen", &igen),
                chunk(b"shdr", &shdr),
            ],
        );
        let mut body = b"sfbk".to_vec();
        body.extend(info);
        body.extend(sdta);
        body.extend(pdta);
        chunk(b"RIFF", &body)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::tiny_sf2;
    use super::*;

    fn ev(start: i64, dur: i64, ch: u8, key: u8) -> NoteEvent {
        NoteEvent {
            start_tick: start,
            duration_ticks: dur,
            channel: ch,
            program: 0,
            key,
            velocity: 100,
        }
    }

    #[test]
    fn loads_generated_sf2_and_renders_note_in_time() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("teste.sf2");
        std::fs::write(&p, tiny_sf2()).unwrap();
        let sf = SoundFontSynth::load(&p).unwrap();
        assert_eq!(sf.preset_count(), 1);
        assert_eq!(sf.bank_name(), "Teste");
        let tempo = TempoMap::default(); // 120 BPM
        let [l, r] = sf
            .render(&[ev(960, 960, 0, 69)], &tempo, 48_000, 0.5)
            .unwrap();
        assert_eq!(l.len(), 72_000, "fim da nota (1 s) + cauda 0,5 s");
        assert_eq!(r.len(), l.len());
        assert!(l[..24_000].iter().all(|x| x.abs() < 1e-6), "silêncio antes");
        assert!(l[24_500..47_000].iter().any(|x| x.abs() > 1e-3), "som");
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
    }

    #[test]
    fn missing_or_invalid_files_are_typed_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = SoundFontSynth::load(&dir.path().join("nao-existe.sf2")).unwrap_err();
        assert_eq!(missing.code(), "synth.io");
        let junk = dir.path().join("lixo.sf2");
        std::fs::write(&junk, b"isto nao e um soundfont").unwrap();
        assert_eq!(
            SoundFontSynth::load(&junk).unwrap_err().code(),
            "synth.invalid_soundfont"
        );
        // truncado em vários pontos: nunca entra em pânico
        let full = tiny_sf2();
        for cut in [4usize, 12, 40, 200, full.len() / 2, full.len() - 3] {
            let f = dir.path().join(format!("cut{cut}.sf2"));
            std::fs::write(&f, &full[..cut]).unwrap();
            assert!(SoundFontSynth::load(&f).is_err(), "corte em {cut}");
        }
        // bytes corrompidos no meio dos parâmetros
        let mut bad = full.clone();
        let at = bad.len() - 120;
        for b in &mut bad[at..at + 40] {
            *b = 0xFF;
        }
        let f = dir.path().join("corrompido.sf2");
        std::fs::write(&f, &bad).unwrap();
        let _ = SoundFontSynth::load(&f); // erro ou sucesso, mas sem pânico
        assert_eq!(
            SoundFontSynth::load(dir.path()).unwrap_err().code(),
            "synth.io"
        );
    }

    #[test]
    fn rejects_absurd_length_and_bad_rate() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.sf2");
        std::fs::write(&p, tiny_sf2()).unwrap();
        let sf = SoundFontSynth::load(&p).unwrap();
        let tempo = TempoMap::default();
        assert_eq!(
            sf.render(&[ev(0, 960 * 2 * 60 * 60, 0, 60)], &tempo, 48_000, 0.0)
                .unwrap_err(),
            SynthError::TooLong
        );
        assert_eq!(
            sf.render(&[ev(0, 960, 0, 60)], &tempo, 1_000, 0.0)
                .unwrap_err()
                .code(),
            "synth.unsupported_rate"
        );
    }
}
