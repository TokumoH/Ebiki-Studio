//! Entrada malformada nunca causa pânico: bytes aleatórios e arquivos válidos truncados ou
//! com bytes trocados resultam em `Ok` ou num `CodecError` com código conhecido.

mod common;

use common::*;
use infra_codecs::{decode_to_f32, probe, CodecError, EncodeOptions, ExportFormat, SampleDepth};
use proptest::prelude::*;
use std::path::Path;
use tempfile::tempdir;

const KNOWN: [&str; 6] = [
    "codec.unsupported_format",
    "codec.corrupt",
    "codec.io",
    "codec.too_large",
    "codec.invalid_parameter",
    "codec.resample",
];

fn check(r: Result<impl Sized, CodecError>) {
    if let Err(e) = r {
        assert!(KNOWN.contains(&e.code()), "código inesperado: {e:?}");
        assert!(!e.to_string().is_empty());
    }
}

fn exercise(path: &Path) {
    check(probe(path));
    check(decode_to_f32(path, 48_000));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn random_bytes_never_panic(
        bytes in proptest::collection::vec(any::<u8>(), 0..4096),
        ext in prop::sample::select(vec!["wav", "aiff", "flac", "mp3", "ogg", "m4a", "aac", "bin"]),
    ) {
        let dir = tempdir().unwrap();
        let p = dir.path().join(format!("r.{ext}"));
        std::fs::write(&p, &bytes).unwrap();
        exercise(&p);
    }

    #[test]
    fn random_bytes_after_valid_magic_never_panic(
        bytes in proptest::collection::vec(any::<u8>(), 0..2048),
        magic in prop::sample::select(vec![
            b"RIFF\x24\x00\x00\x00WAVEfmt ".to_vec(),
            b"FORM\x00\x00\x00\x40AIFFCOMM".to_vec(),
            b"fLaC\x00\x00\x00\x22".to_vec(),
            b"OggS\x00\x02".to_vec(),
            b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec(),
            vec![0xFF, 0xFB, 0x90, 0x64],
            b"\x00\x00\x00\x20ftypM4A ".to_vec(),
        ]),
    ) {
        let dir = tempdir().unwrap();
        let p = dir.path().join("m.bin");
        let mut data = magic;
        data.extend_from_slice(&bytes);
        std::fs::write(&p, &data).unwrap();
        exercise(&p);
    }
}

fn valid_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let audio = stereo(
        noise(1, 20_000, 0.5),
        sine(440.0, 44_100, 20_000, 0.5),
        44_100,
    );
    let mut formats = vec![ExportFormat::Wav, ExportFormat::Aiff, ExportFormat::Flac];
    if cfg!(feature = "mp3") {
        formats.push(ExportFormat::Mp3);
    }
    if cfg!(feature = "vorbis") {
        formats.push(ExportFormat::OggVorbis);
    }
    formats
        .into_iter()
        .map(|f| {
            encode(
                &dir.join(format!("v.{}", f.extension())),
                f,
                &audio,
                &EncodeOptions {
                    depth: SampleDepth::Int16,
                    ..EncodeOptions::default()
                },
            )
        })
        .collect()
}

#[test]
fn truncated_and_bit_flipped_files_never_panic() {
    let dir = tempdir().unwrap();
    for src in valid_files(dir.path()) {
        let bytes = std::fs::read(&src).unwrap();
        let ext = src.extension().unwrap().to_str().unwrap().to_string();
        // Truncamentos em vários pontos.
        for cut in [
            1usize,
            4,
            12,
            44,
            100,
            1000,
            bytes.len() / 3,
            bytes.len() - 1,
        ] {
            let p = dir.path().join(format!("t{cut}.{ext}"));
            std::fs::write(&p, &bytes[..cut.min(bytes.len())]).unwrap();
            exercise(&p);
        }
        // Bytes trocados de forma determinística.
        let mut s = 0x1234_5678u64;
        for round in 0..16 {
            let mut b = bytes.clone();
            for _ in 0..8 {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                let i = (s as usize) % b.len();
                b[i] ^= (s >> 32) as u8 | 1;
            }
            let p = dir.path().join(format!("f{round}.{ext}"));
            std::fs::write(&p, &b).unwrap();
            exercise(&p);
        }
    }
}
