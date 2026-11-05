//! Ida e volta codificar → decodificar com sinais sintéticos (seno e ruído).

mod common;

use common::*;
#[cfg(feature = "mp3")]
use infra_codecs::Mp3Mode;
use infra_codecs::{
    decode_to_f32, export_stems, probe, DecodedAudio, EncodeOptions, Encoder, ExportFormat,
    SampleDepth, SourceFormat, Stem,
};
use tempfile::tempdir;

const SR: u32 = 48_000;

fn test_signal(frames: usize) -> DecodedAudio {
    let l: Vec<f32> = sine(997.0, SR, frames, 0.6)
        .into_iter()
        .zip(noise(1, frames, 0.2))
        .map(|(a, b)| a + b)
        .collect();
    let r = noise(2, frames, 0.7);
    stereo(l, r, SR)
}

#[cfg(any(feature = "mp3", feature = "vorbis"))]
/// Soma de senos inarmônicos: sem periodicidade curta, então a correlação tem um único pico.
fn aperiodic(sr: u32, frames: usize) -> Vec<f32> {
    let a = sine(440.0, sr, frames, 0.3);
    let b = sine(1234.5, sr, frames, 0.2);
    let c = sine(3001.7, sr, frames, 0.1);
    a.iter()
        .zip(&b)
        .zip(&c)
        .map(|((x, y), z)| x + y + z)
        .collect()
}

#[cfg(any(feature = "mp3", feature = "vorbis"))]
/// Atraso de `decoded` em relação a `orig` (procura em ±`max_lag` em torno de `start`).
fn lag_of(orig: &[f32], decoded: &[f32], start: usize, len: usize, max_lag: usize) -> (isize, f64) {
    let (lag, c) = best_correlation(
        &orig[start..start + len],
        &decoded[start - max_lag..],
        max_lag as isize,
    );
    (lag - max_lag as isize, c)
}

fn no_dither(depth: SampleDepth) -> EncodeOptions {
    EncodeOptions {
        depth,
        dither: false,
        ..EncodeOptions::default()
    }
}

/// Valor esperado após quantização sem dither e decodificação (`q / 2^(bits-1)`).
fn quantized(x: f32, bits: u32) -> f32 {
    let scale = f64::from(1u32 << (bits - 1));
    let q = (f64::from(x) * scale).round().clamp(-scale, scale - 1.0);
    (q / scale) as f32
}

fn assert_lossless(format: ExportFormat, depth: SampleDepth, expected_format: SourceFormat) {
    let dir = tempdir().unwrap();
    let audio = test_signal(10_007); // não múltiplo do bloco
    let path = encode(
        &dir.path().join(format!("x.{}", format.extension())),
        format,
        &audio,
        &no_dither(depth),
    );
    let info = probe(&path).unwrap();
    assert_eq!(info.format, expected_format);
    assert_eq!(info.sample_rate, SR);
    assert_eq!(info.channels, 2);
    assert_eq!(info.frames, 10_007);

    let back = decode_to_f32(&path, SR).unwrap();
    assert_eq!(back.sample_rate, SR);
    assert_eq!(back.channels.len(), 2);
    assert_eq!(back.frames(), 10_007, "{format:?} {depth:?}");
    for (orig, dec) in audio.channels.iter().zip(&back.channels) {
        for (i, (&a, &b)) in orig.iter().zip(dec).enumerate() {
            let expected = match depth {
                SampleDepth::Int16 => quantized(a, 16),
                SampleDepth::Int24 => quantized(a, 24),
                SampleDepth::Float32 => a,
            };
            assert_eq!(b, expected, "{format:?} {depth:?} amostra {i}");
        }
    }
}

#[test]
fn wav_round_trip_is_exact_for_all_depths() {
    for d in [SampleDepth::Int16, SampleDepth::Int24, SampleDepth::Float32] {
        assert_lossless(ExportFormat::Wav, d, SourceFormat::Wav);
    }
}

#[test]
fn aiff_round_trip_is_exact_for_all_depths() {
    for d in [SampleDepth::Int16, SampleDepth::Int24, SampleDepth::Float32] {
        assert_lossless(ExportFormat::Aiff, d, SourceFormat::Aiff);
    }
}

#[test]
fn flac_round_trip_is_exact_for_16_and_24_bits() {
    for d in [SampleDepth::Int16, SampleDepth::Int24] {
        assert_lossless(ExportFormat::Flac, d, SourceFormat::Flac);
    }
}

#[test]
fn flac_mono_and_tiny_files() {
    let dir = tempdir().unwrap();
    for frames in [1usize, 17, 4096, 4097] {
        let audio = mono(noise(frames as u64, frames, 0.5), 44_100);
        let p = encode(
            &dir.path().join(format!("m{frames}.flac")),
            ExportFormat::Flac,
            &audio,
            &no_dither(SampleDepth::Int16),
        );
        let back = decode_to_f32(&p, 44_100).unwrap();
        assert_eq!(back.channels.len(), 1);
        assert_eq!(back.frames(), frames);
        for (a, b) in audio.channels[0].iter().zip(&back.channels[0]) {
            assert_eq!(*b, quantized(*a, 16));
        }
    }
}

#[test]
fn dithered_16_bit_error_is_bounded() {
    let dir = tempdir().unwrap();
    let audio = test_signal(20_000);
    let opts = EncodeOptions {
        depth: SampleDepth::Int16,
        dither: true,
        ..EncodeOptions::default()
    };
    let p = encode(&dir.path().join("d.wav"), ExportFormat::Wav, &audio, &opts);
    let back = decode_to_f32(&p, SR).unwrap();
    let lsb = 1.0 / 32768.0;
    let mut diff_count = 0;
    for (o, d) in audio.channels.iter().zip(&back.channels) {
        for (&a, &b) in o.iter().zip(d) {
            assert!((a - b).abs() <= 1.5 * lsb + 1e-9);
            if b != quantized(a, 16) {
                diff_count += 1;
            }
        }
    }
    assert!(diff_count > 0, "o dither deveria alterar algumas amostras");
}

#[cfg(feature = "mp3")]
#[test]
fn mp3_cbr_and_v0_preserve_duration_and_level() {
    let dir = tempdir().unwrap();
    let frames = 44_100 * 2;
    for (sr, channels) in [(44_100u32, 2usize), (48_000, 1)] {
        let x = sine(1000.0, sr, frames, 0.5);
        let audio = DecodedAudio {
            channels: vec![x.clone(); channels],
            sample_rate: sr,
        };
        for mode in [Mp3Mode::Cbr(128), Mp3Mode::Cbr(320), Mp3Mode::V0] {
            let opts = EncodeOptions {
                mp3: mode,
                ..EncodeOptions::default()
            };
            let p = encode(
                &dir.path().join(format!("s{sr}-{channels}-{mode:?}.mp3")),
                ExportFormat::Mp3,
                &audio,
                &opts,
            );
            let info = probe(&p).unwrap();
            assert_eq!(info.format, SourceFormat::Mp3);
            let back = decode_to_f32(&p, sr).unwrap();
            assert_eq!(back.channels.len(), channels);
            // Com a tag LAME/Xing que gravamos, atraso (576 + 529) e preenchimento do
            // codificador são removidos pelo decodificador (gapless): comprimento exato.
            assert_eq!(back.frames(), frames, "{mode:?}");
            let ratio = rms(&back.channels[0]) / rms(&x);
            assert!((0.9..1.1).contains(&ratio), "{mode:?}: RMS {ratio}");
            let mid = &back.channels[0][4410..frames - 4410];
            let f = estimate_freq(mid, sr);
            assert!((f - 1000.0).abs() < 1.0, "{mode:?}: {f} Hz");
        }
    }
}

#[cfg(feature = "mp3")]
#[test]
fn mp3_is_gapless_sample_exact_with_lame_tag() {
    let dir = tempdir().unwrap();
    let frames = 100_000;
    let audio = stereo(
        aperiodic(44_100, frames),
        sine(660.0, 44_100, frames, 0.5),
        44_100,
    );
    let p = encode(
        &dir.path().join("g.mp3"),
        ExportFormat::Mp3,
        &audio,
        &EncodeOptions::default(),
    );
    let back = decode_to_f32(&p, 44_100).unwrap();
    let (lag, c) = lag_of(&audio.channels[0], &back.channels[0], 5000, 4000, 1200);
    assert!(c > 0.99, "correlação {c}");
    assert_eq!(lag, 0, "atraso do codificador deveria ser compensado");
    assert_eq!(back.frames(), frames);
}

#[cfg(feature = "vorbis")]
#[test]
fn ogg_vorbis_preserves_duration_and_level() {
    let dir = tempdir().unwrap();
    let frames = 48_000 * 2;
    let x = aperiodic(SR, frames);
    let audio = stereo(x.clone(), x.clone(), SR);
    let p = encode(
        &dir.path().join("v.ogg"),
        ExportFormat::OggVorbis,
        &audio,
        &EncodeOptions::default(),
    );
    let info = probe(&p).unwrap();
    assert_eq!(info.format, SourceFormat::Ogg);
    assert_eq!(info.codec, "vorbis");
    let back = decode_to_f32(&p, SR).unwrap();
    // Ogg guarda a posição final em granule: comprimento exato.
    assert_eq!(back.frames(), frames);
    let ratio = rms(&back.channels[1]) / rms(&x);
    assert!((0.9..1.1).contains(&ratio), "RMS {ratio}");
    let (lag, c) = lag_of(&x, &back.channels[0], 4800, 4800, 64);
    assert!(c > 0.99, "correlação {c}");
    assert_eq!(lag, 0, "Vorbis não tem atraso após o corte por granule");
}

#[test]
fn resampling_44k1_to_48k_preserves_frequency_and_length_ratio() {
    let dir = tempdir().unwrap();
    let frames = 44_100 * 2;
    let audio = mono(sine(1000.0, 44_100, frames, 0.5), 44_100);
    let p = encode(
        &dir.path().join("r.wav"),
        ExportFormat::Wav,
        &audio,
        &EncodeOptions {
            depth: SampleDepth::Float32,
            ..EncodeOptions::default()
        },
    );
    let back = decode_to_f32(&p, 48_000).unwrap();
    assert_eq!(back.sample_rate, 48_000);
    assert_eq!(back.channels.len(), 1, "mono continua mono");
    assert_eq!(back.frames(), 96_000, "razão de comprimento 48000/44100");
    let f = estimate_freq(&back.channels[0][2_000..94_000], 48_000);
    assert!((f - 1000.0).abs() < 0.05, "{f} Hz");
    let ratio = rms(&back.channels[0][2_000..94_000]) / rms(&audio.channels[0]);
    assert!((ratio - 1.0).abs() < 0.01, "RMS {ratio}");
}

#[test]
fn surround_wav_is_downmixed_to_stereo() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("six.wav");
    let spec = hound::WavSpec {
        channels: 6,
        sample_rate: SR,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    // FL=0.1 FR=0.2 FC=0.3 LFE=0.9 RL=0.4 RR=0.5
    let vals = [0.1f32, 0.2, 0.3, 0.9, 0.4, 0.5];
    for _ in 0..100 {
        for v in vals {
            w.write_sample(v).unwrap();
        }
    }
    w.finalize().unwrap();
    let info = probe(&path).unwrap();
    assert_eq!(info.channels, 6);
    let back = decode_to_f32(&path, SR).unwrap();
    assert_eq!(back.channels.len(), 2);
    let k = std::f32::consts::FRAC_1_SQRT_2;
    let l = 0.1 + 0.3 * k + 0.4 * k;
    let r = 0.2 + 0.3 * k + 0.5 * k;
    assert!(
        (back.channels[0][50] - l).abs() < 1e-6,
        "{}",
        back.channels[0][50]
    );
    assert!((back.channels[1][50] - r).abs() < 1e-6);
}

#[test]
fn stems_export_one_file_per_stem() {
    let dir = tempdir().unwrap();
    let a = test_signal(5000);
    let b = mono(noise(9, 5000, 0.3), SR);
    let files = export_stems(
        dir.path(),
        &[
            Stem {
                name: "Voz",
                audio: &a,
            },
            Stem {
                name: "Violão/DI",
                audio: &b,
            },
        ],
        ExportFormat::Flac,
        &no_dither(SampleDepth::Int24),
    )
    .unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[0].path.ends_with("Voz.flac"));
    assert!(files[1].path.ends_with("Violão_DI.flac"));
    assert_eq!(decode_to_f32(&files[1].path, SR).unwrap().channels.len(), 1);
    assert_eq!(files[0].frames, 5000);

    // Nomes repetidos são rejeitados antes de gravar qualquer coisa.
    let e = export_stems(
        dir.path(),
        &[
            Stem {
                name: "x",
                audio: &a,
            },
            Stem {
                name: "X",
                audio: &a,
            },
        ],
        ExportFormat::Wav,
        &EncodeOptions::default(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "codec.invalid_parameter");
    assert!(!dir.path().join("x.wav").exists());
}

#[test]
fn stems_failure_removes_partial_outputs() {
    let dir = tempdir().unwrap();
    let ok = test_signal(100);
    let bad = DecodedAudio {
        channels: vec![vec![0.0; 10]],
        sample_rate: 1_000, // taxa inválida → falha no segundo stem
    };
    let e = export_stems(
        dir.path(),
        &[
            Stem {
                name: "a",
                audio: &ok,
            },
            Stem {
                name: "b",
                audio: &bad,
            },
        ],
        ExportFormat::Wav,
        &EncodeOptions::default(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "codec.invalid_parameter");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn encoder_dropped_without_finish_leaves_nothing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("never.wav");
    {
        let mut enc =
            Encoder::new(&path, ExportFormat::Wav, SR, 2, &EncodeOptions::default()).unwrap();
        enc.write(&[&[0.1; 100], &[0.2; 100]]).unwrap();
    }
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn encoder_rejects_bad_blocks_and_overwrites_atomically() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("o.wav");
    std::fs::write(&path, b"old").unwrap();
    let mut enc = Encoder::new(
        &path,
        ExportFormat::Wav,
        SR,
        2,
        &no_dither(SampleDepth::Int24),
    )
    .unwrap();
    assert_eq!(
        enc.write(&[&[0.0; 10]]).unwrap_err().code(),
        "codec.invalid_parameter"
    );
    assert_eq!(
        enc.write(&[&[0.0; 10], &[0.0; 9]]).unwrap_err().code(),
        "codec.invalid_parameter"
    );
    enc.write(&[&[f32::NAN; 10], &[f32::INFINITY; 10]]).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"old",
        "destino só muda no finish"
    );
    enc.finish().unwrap();
    let back = decode_to_f32(&path, SR).unwrap();
    assert!(back.channels.iter().flatten().all(|&x| x == 0.0));
}

#[test]
fn aac_export_is_unsupported_with_clear_message() {
    let dir = tempdir().unwrap();
    let e = match Encoder::new(
        &dir.path().join("a.m4a"),
        ExportFormat::Aac,
        SR,
        2,
        &EncodeOptions::default(),
    ) {
        Err(e) => e,
        Ok(_) => panic!("AAC não deveria estar disponível"),
    };
    assert_eq!(e.code(), "codec.unsupported_format");
    assert!(e.to_string().contains("ADR-0023"));
}
