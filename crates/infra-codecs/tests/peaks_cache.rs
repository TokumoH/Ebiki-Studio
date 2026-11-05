//! Cache de picos: gravação, reutilização e regeneração quando corrompido.

mod common;

use common::*;
use infra_codecs::{
    compute_peaks, load_or_compute, load_or_compute_multi, peaks_cache_path, CodecError,
    DecodedAudio, DEFAULT_RESOLUTIONS,
};
use std::cell::Cell;
use std::fs;
use tempfile::tempdir;

const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn audio() -> DecodedAudio {
    stereo(
        sine(100.0, 48_000, 48_000, 0.8),
        noise(4, 48_000, 0.3),
        48_000,
    )
}

#[test]
fn computes_then_reuses_cache_without_decoding() {
    let project = tempdir().unwrap();
    let calls = Cell::new(0);
    let src = || {
        calls.set(calls.get() + 1);
        Ok::<_, CodecError>(audio())
    };
    let p1 = load_or_compute(project.path(), HASH, 1024, src).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(p1, compute_peaks(&audio(), 1024).unwrap());
    let path = peaks_cache_path(project.path(), HASH, 1024).unwrap();
    assert!(path.exists());
    assert!(path.ends_with(format!("cache/peaks/{HASH}-1024.peaks")));

    let p2 = load_or_compute(project.path(), HASH, 1024, || {
        calls.set(calls.get() + 1);
        Ok(audio())
    })
    .unwrap();
    assert_eq!(calls.get(), 1, "segunda leitura vem do cache");
    assert_eq!(p1, p2);
}

#[test]
fn corrupted_cache_is_recomputed_and_rewritten() {
    let project = tempdir().unwrap();
    let good = load_or_compute(project.path(), HASH, 256, || Ok(audio())).unwrap();
    let path = peaks_cache_path(project.path(), HASH, 256).unwrap();
    for corruption in [b"lixo".to_vec(), Vec::new(), {
        let mut b = fs::read(&path).unwrap();
        let n = b.len();
        b[n / 2] ^= 0xFF;
        b
    }] {
        fs::write(&path, &corruption).unwrap();
        let calls = Cell::new(0);
        let again = load_or_compute(project.path(), HASH, 256, || {
            calls.set(calls.get() + 1);
            Ok(audio())
        })
        .unwrap();
        assert_eq!(calls.get(), 1, "cache corrompido deve ser recalculado");
        assert_eq!(again, good);
        assert_ne!(fs::read(&path).unwrap(), corruption, "cache regravado");
    }
}

#[test]
fn multi_resolution_decodes_once_and_values_are_correct() {
    let project = tempdir().unwrap();
    let calls = Cell::new(0);
    let all = load_or_compute_multi(project.path(), HASH, &DEFAULT_RESOLUTIONS, || {
        calls.set(calls.get() + 1);
        Ok(audio())
    })
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(all.len(), 3);
    for (p, &r) in all.iter().zip(&DEFAULT_RESOLUTIONS) {
        assert_eq!(p.frames_per_peak, r);
        assert_eq!(p.buckets(), 48_000usize.div_ceil(r as usize));
        // Seno de 100 Hz com amplitude 0,8: em blocos ≥ 1 período (480 amostras) o pico é ±0,8.
        if r >= 1024 {
            for (lo, hi) in p.channels[0].min.iter().zip(&p.channels[0].max) {
                assert!((hi - 0.8).abs() < 1e-3 && (lo + 0.8).abs() < 1e-3);
            }
        }
    }
    // Remover só uma resolução: decodifica de novo uma vez e recompõe.
    fs::remove_file(peaks_cache_path(project.path(), HASH, 4096).unwrap()).unwrap();
    let again = load_or_compute_multi(project.path(), HASH, &DEFAULT_RESOLUTIONS, || {
        calls.set(calls.get() + 1);
        Ok(audio())
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(again, all);
}

#[test]
fn invalid_hash_never_touches_the_filesystem() {
    let project = tempdir().unwrap();
    let e = load_or_compute(project.path(), "../../x", 256, || Ok(audio())).unwrap_err();
    assert_eq!(e.code(), "codec.invalid_path");
    assert!(!project.path().join("cache").exists());
}

#[test]
fn source_errors_are_propagated() {
    let project = tempdir().unwrap();
    let e = load_or_compute(project.path(), HASH, 256, || {
        Err(CodecError::Corrupt("x".into()))
    })
    .unwrap_err();
    assert_eq!(e.code(), "codec.corrupt");
}
