//! Importação para a pasta do projeto: hash, deduplicação, confinamento e limites.

mod common;

use common::*;
use infra_codecs::{
    import_into_project, import_into_project_with, sha256_file, EncodeOptions, ExportFormat,
    ImportOptions, SampleDepth, SourceFormat,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn make_wav(dir: &Path, name: &str, seed: u64) -> std::path::PathBuf {
    let audio = stereo(noise(seed, 4800, 0.5), noise(seed + 1, 4800, 0.5), 48_000);
    encode(
        &dir.join(name),
        ExportFormat::Wav,
        &audio,
        &EncodeOptions {
            depth: SampleDepth::Int16,
            ..EncodeOptions::default()
        },
    )
}

fn imported_files(project: &Path) -> Vec<String> {
    let dir = project.join("audio/imported");
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|rd| {
            rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn import_copies_with_content_hash_and_preserves_original() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let src = make_wav(src_dir.path(), "Minha Voz.WAV", 1);
    let original = fs::read(&src).unwrap();

    let imported = import_into_project(&src, project.path()).unwrap();
    let hash = sha256_file(&src).unwrap();
    assert_eq!(imported.content_hash, hash);
    assert_eq!(hash.len(), 64);
    assert!(hash
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert_eq!(imported.rel_path, format!("audio/imported/{hash}.wav"));
    assert_eq!(imported.original_name, "Minha Voz.WAV");
    assert!(!imported.deduplicated);
    assert_eq!(imported.info.format, SourceFormat::Wav);
    assert_eq!(imported.info.frames, 4800);
    assert_eq!(imported.info.channels, 2);

    let copied = project.path().join(&imported.rel_path);
    assert_eq!(fs::read(&copied).unwrap(), original, "cópia byte a byte");
    assert_eq!(fs::read(&src).unwrap(), original, "original intocado");
    assert_eq!(imported_files(project.path()).len(), 1, "sem temporários");
}

#[test]
fn same_content_is_deduplicated() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let a = make_wav(src_dir.path(), "a.wav", 7);
    let b = src_dir.path().join("copia.wave");
    fs::copy(&a, &b).unwrap();
    let c = make_wav(src_dir.path(), "c.wav", 99);

    let first = import_into_project(&a, project.path()).unwrap();
    let second = import_into_project(&b, project.path()).unwrap();
    let third = import_into_project(&c, project.path()).unwrap();
    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(first.rel_path, second.rel_path, "extensão canônica .wav");
    assert!(second.deduplicated);
    assert_eq!(second.original_name, "copia.wave");
    assert_ne!(first.content_hash, third.content_hash);
    assert_eq!(imported_files(project.path()).len(), 2);
}

#[test]
fn non_whitelisted_or_fake_audio_is_rejected_without_writing() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let exe = src_dir.path().join("virus.exe");
    fs::write(&exe, b"MZ\x90\x00").unwrap();
    let e = import_into_project(&exe, project.path()).unwrap_err();
    assert_eq!(e.code(), "codec.unsupported_format");

    let fake = src_dir.path().join("texto.wav");
    fs::write(
        &fake,
        "isto não é áudio, só texto com extensão de wav".repeat(100),
    )
    .unwrap();
    let e = import_into_project(&fake, project.path()).unwrap_err();
    assert!(
        ["codec.unsupported_format", "codec.corrupt"].contains(&e.code()),
        "{e:?}"
    );

    let empty = src_dir.path().join("vazio.mp3");
    fs::write(&empty, b"").unwrap();
    assert_eq!(
        import_into_project(&empty, project.path())
            .unwrap_err()
            .code(),
        "codec.corrupt"
    );
    assert!(imported_files(project.path()).is_empty());
}

#[test]
fn size_limit_is_enforced() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let src = make_wav(src_dir.path(), "a.wav", 3);
    let e = import_into_project_with(&src, project.path(), &ImportOptions { max_bytes: 1000 })
        .unwrap_err();
    assert_eq!(e.code(), "codec.too_large");
    assert!(imported_files(project.path()).is_empty());
}

#[test]
fn missing_source_and_directories_are_rejected() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let e =
        import_into_project(&src_dir.path().join("nao-existe.wav"), project.path()).unwrap_err();
    assert_eq!(e.code(), "codec.not_found");
    let d = src_dir.path().join("pasta.wav");
    fs::create_dir(&d).unwrap();
    assert_eq!(
        import_into_project(&d, project.path()).unwrap_err().code(),
        "codec.invalid_path"
    );
    let src = make_wav(src_dir.path(), "a.wav", 3);
    assert_eq!(
        import_into_project(&src, &src_dir.path().join("sem-projeto"))
            .unwrap_err()
            .code(),
        "codec.not_found"
    );
}

#[cfg(unix)]
#[test]
fn symlink_escaping_the_project_is_rejected() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::create_dir_all(project.path().join("audio")).unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join("audio/imported")).unwrap();
    let src = make_wav(src_dir.path(), "a.wav", 5);
    let e = import_into_project(&src, project.path()).unwrap_err();
    assert_eq!(e.code(), "codec.invalid_path");
    assert_eq!(
        fs::read_dir(outside.path()).unwrap().count(),
        0,
        "nada fora do projeto"
    );
}

#[cfg(unix)]
#[test]
fn existing_symlink_at_destination_name_is_not_trusted() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let src = make_wav(src_dir.path(), "a.wav", 11);
    let hash = sha256_file(&src).unwrap();
    let dir = project.path().join("audio/imported");
    fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink("/etc/hostname", dir.join(format!("{hash}.wav"))).unwrap();
    let e = import_into_project(&src, project.path()).unwrap_err();
    assert_eq!(e.code(), "codec.invalid_path");
}

#[test]
fn flac_and_mp3_imports_keep_their_format() {
    let src_dir = tempdir().unwrap();
    let project = tempdir().unwrap();
    let audio = mono(sine(440.0, 44_100, 44_100, 0.5), 44_100);
    let flac = encode(
        &src_dir.path().join("x.flac"),
        ExportFormat::Flac,
        &audio,
        &EncodeOptions {
            depth: SampleDepth::Int16,
            ..EncodeOptions::default()
        },
    );
    let i = import_into_project(&flac, project.path()).unwrap();
    assert!(i.rel_path.ends_with(".flac"));
    assert_eq!(i.info.format, SourceFormat::Flac);
    assert_eq!(i.info.sample_rate, 44_100);
    assert_eq!(i.info.channels, 1);
    #[cfg(feature = "mp3")]
    {
        let mp3 = encode(
            &src_dir.path().join("x.mp3"),
            ExportFormat::Mp3,
            &audio,
            &EncodeOptions::default(),
        );
        let i = import_into_project(&mp3, project.path()).unwrap();
        assert!(i.rel_path.ends_with(".mp3"));
        assert_eq!(i.info.format, SourceFormat::Mp3);
        assert_eq!(i.info.frames, 44_100);
    }
}
