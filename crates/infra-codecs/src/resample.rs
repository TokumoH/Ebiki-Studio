//! Conversão de taxa de amostragem em fluxo (sinc de alta qualidade, `rubato`).
//!
//! O resampler recebe blocos de tamanho arbitrário (como saem do decodificador), acumula numa
//! fila pequena e processa em pedaços fixos de [`CHUNK_FRAMES`] quadros, de modo que a memória
//! de entrada fica limitada independentemente do tamanho do arquivo. O atraso inicial do filtro
//! é descartado e o comprimento final é exatamente `ceil(entrada × razão)`.

use crate::error::CodecError;
use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

/// Tamanho do pedaço de entrada processado por chamada.
pub const CHUNK_FRAMES: usize = 1024;

/// Parâmetros do filtro sinc: 256 pontos, janela Blackman-Harris², sobreamostragem 128,
/// interpolação cúbica, corte automático (rejeição de aliasing ≈ nível dos lóbulos laterais).
fn sinc_params() -> SincInterpolationParameters {
    SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: None,
        oversampling_factor: 128,
        interpolation: SincInterpolationType::Cubic,
        window: WindowFunction::BlackmanHarris2,
    }
}

fn map_err(e: impl std::fmt::Display) -> CodecError {
    CodecError::Resample(e.to_string())
}

/// Resampler em fluxo para `channels` canais de `from_hz` para `to_hz`.
pub struct StreamResampler {
    inner: Async<f32>,
    channels: usize,
    ratio: f64,
    from_hz: u32,
    to_hz: u32,
    fifo: Vec<Vec<f32>>,
    scratch: Vec<Vec<f32>>,
    to_trim: usize,
    input_total: u64,
    output_total: u64,
}

impl StreamResampler {
    pub fn new(from_hz: u32, to_hz: u32, channels: usize) -> Result<Self, CodecError> {
        if from_hz == 0 || to_hz == 0 || channels == 0 {
            return Err(CodecError::InvalidParameter(
                "taxa de amostragem e número de canais devem ser positivos".into(),
            ));
        }
        let ratio = f64::from(to_hz) / f64::from(from_hz);
        let inner = Async::<f32>::new_sinc(
            ratio,
            1.0,
            &sinc_params(),
            CHUNK_FRAMES,
            channels,
            FixedAsync::Input,
        )
        .map_err(map_err)?;
        let out_max = inner.output_frames_max();
        let to_trim = inner.output_delay();
        Ok(Self {
            inner,
            channels,
            ratio,
            from_hz,
            to_hz,
            fifo: vec![Vec::with_capacity(CHUNK_FRAMES * 2); channels],
            scratch: vec![vec![0.0; out_max]; channels],
            to_trim,
            input_total: 0,
            output_total: 0,
        })
    }

    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Acrescenta `frames` quadros de `input` (um `Vec` por canal) e escreve em `out` toda a
    /// saída já disponível.
    pub fn push(
        &mut self,
        input: &[Vec<f32>],
        frames: usize,
        out: &mut [Vec<f32>],
    ) -> Result<(), CodecError> {
        debug_assert_eq!(input.len(), self.channels);
        let mut offset = 0;
        while offset < frames {
            let room = CHUNK_FRAMES - self.fifo[0].len();
            let take = room.min(frames - offset);
            for (q, src) in self.fifo.iter_mut().zip(input) {
                q.extend_from_slice(&src[offset..offset + take]);
            }
            offset += take;
            self.input_total += take as u64;
            if self.fifo[0].len() == CHUNK_FRAMES {
                self.process_fifo(None, out)?;
            }
        }
        Ok(())
    }

    /// Processa o resto da fila, esvazia o filtro e corta a saída no comprimento exato.
    pub fn finish(mut self, out: &mut [Vec<f32>]) -> Result<(), CodecError> {
        // Aritmética inteira: evita que 44100 × (48000/44100) vire 48000,0000001 → 48001.
        let num = u128::from(self.input_total) * u128::from(self.to_hz);
        let expected = num.div_ceil(u128::from(self.from_hz)) as u64;
        let pending = self.fifo[0].len();
        if pending > 0 {
            self.process_fifo(Some(pending), out)?;
        }
        let mut guard = 0;
        while self.output_total < expected {
            self.process_fifo(Some(0), out)?;
            guard += 1;
            if guard > 1_000 {
                return Err(CodecError::Resample(
                    "o resampler não produziu a saída esperada".into(),
                ));
            }
        }
        let excess = (self.output_total - expected) as usize;
        if excess > 0 {
            for c in out.iter_mut() {
                let len = c.len() - excess;
                c.truncate(len);
            }
        }
        Ok(())
    }

    fn process_fifo(
        &mut self,
        partial: Option<usize>,
        out: &mut [Vec<f32>],
    ) -> Result<(), CodecError> {
        for q in &mut self.fifo {
            q.resize(CHUNK_FRAMES, 0.0);
        }
        let input =
            SequentialSliceOfVecs::new(&self.fifo, self.channels, CHUNK_FRAMES).map_err(map_err)?;
        let out_len = self.scratch[0].len();
        let mut output = SequentialSliceOfVecs::new_mut(&mut self.scratch, self.channels, out_len)
            .map_err(map_err)?;
        let indexing = partial.map(|n| Indexing::new().partial_len(n));
        let (_, produced) = self
            .inner
            .process_into_buffer(&input, &mut output, indexing.as_ref())
            .map_err(map_err)?;
        for q in &mut self.fifo {
            q.clear();
        }
        let skip = self.to_trim.min(produced);
        self.to_trim -= skip;
        for (dst, src) in out.iter_mut().zip(&self.scratch) {
            dst.extend_from_slice(&src[skip..produced]);
        }
        self.output_total += (produced - skip) as u64;
        Ok(())
    }
}

/// Reamostra planos inteiros já em memória (conveniência sobre [`StreamResampler`]).
pub fn resample_planes(
    planes: &[Vec<f32>],
    from_hz: u32,
    to_hz: u32,
) -> Result<Vec<Vec<f32>>, CodecError> {
    if from_hz == to_hz || planes.is_empty() {
        return Ok(planes.to_vec());
    }
    let frames = planes.iter().map(Vec::len).min().unwrap_or(0);
    let mut rs = StreamResampler::new(from_hz, to_hz, planes.len())?;
    let mut out = vec![Vec::new(); planes.len()];
    let truncated: Vec<Vec<f32>> = planes.iter().map(|p| p[..frames].to_vec()).collect();
    rs.push(&truncated, frames, &mut out)?;
    rs.finish(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn length_is_exact_ceil_of_ratio() {
        for &(from, to, n) in &[
            (44_100u32, 48_000u32, 44_100usize),
            (48_000, 44_100, 10_000),
            (96_000, 48_000, 777),
            (44_100, 96_000, 1),
        ] {
            let out = resample_planes(&[vec![0.1; n]], from, to).unwrap();
            let expected = (n as f64 * to as f64 / from as f64).ceil() as usize;
            assert_eq!(out[0].len(), expected, "{from}->{to} n={n}");
        }
    }

    #[test]
    fn chunked_push_equals_single_push() {
        let x = sine(1000.0, 44_100, 9_000);
        let whole = resample_planes(std::slice::from_ref(&x), 44_100, 48_000).unwrap();
        let mut rs = StreamResampler::new(44_100, 48_000, 1).unwrap();
        let mut out = vec![Vec::new()];
        for chunk in x.chunks(333) {
            rs.push(&[chunk.to_vec()], chunk.len(), &mut out).unwrap();
        }
        rs.finish(&mut out).unwrap();
        assert_eq!(out[0].len(), whole[0].len());
        for (a, b) in out[0].iter().zip(&whole[0]) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn delay_is_compensated_within_one_sample() {
        // Seno de 1 kHz: a saída reamostrada coincide com o seno ideal na taxa de destino,
        // deslocado de menos de 1 amostra (o rubato descarta um atraso inteiro; o resto
        // fracionário fica < 1 amostra ≈ 21 µs a 48 kHz).
        let x = sine(1000.0, 44_100, 44_100);
        let y = resample_planes(&[x], 44_100, 48_000).unwrap();
        let err_at = |d: f32| {
            (2_000..y[0].len() - 2_000)
                .map(|i| {
                    let t = (i as f32 - d) / 48_000.0;
                    (y[0][i] - (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5).abs()
                })
                .fold(0.0f32, f32::max)
        };
        let (best_d, best_e) = (-20..=20)
            .map(|k| k as f32 * 0.05)
            .map(|d| (d, err_at(d)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        assert!(best_d.abs() < 1.0, "deslocamento {best_d}");
        assert!(best_e < 3e-3, "erro {best_e}");
    }

    #[test]
    fn invalid_parameters_are_errors() {
        assert!(StreamResampler::new(0, 48_000, 1).is_err());
        assert!(StreamResampler::new(48_000, 48_000, 0).is_err());
    }
}
