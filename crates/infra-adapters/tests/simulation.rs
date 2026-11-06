//! Simulação de sistema: uma sessão realista montada pelas mesmas portas que a interface usa,
//! com as implementações reais (codecs, DSP, SQLite, engine) e medida contra o prazo de tempo real.
//!
//! - 32 faixas (mínimo do RNF-05) com as cadeias de efeitos dos presets de instrumento;
//! - 2 buses com envios de todas as faixas e cadeia de master (limiter com look-ahead → PDC);
//! - edições não destrutivas (divisão, fades, reverso, polaridade) e mudança de andamento;
//! - render bloco a bloco (128 amostras a 48 kHz = 2,67 ms) medindo cada bloco e verificando
//!   que o caminho de tempo real não aloca memória.
//!
//! Em release o teste exige folga de CPU; em debug só verifica correção.
//! Medição detalhada: `cargo test --release -p infra-adapters --test simulation -- --nocapture`.
mod common;
use app_core::edit_service::FadeRequest;
use app_core::ports::{ExportFormatKind, ExportSettings};
use app_core::*;
use assert_no_alloc::{assert_no_alloc, reset_violation_count, violation_count, AllocDisabler};
use audio_engine::{create_engine, EngineConfig};
use common::studio;
use ctx_mixing::commands::{CreateBus, EditStrip};
use ctx_mixing::presets::InstrumentPreset;
use ctx_mixing::{AuxSend, Bus, BusId, ChannelStrip, Effect, SendId, StripEdit, StripTarget};
use ctx_project::commands::SetTempo;
use ctx_project::{ClipId, FadeShape};
use infra_codecs::{EncodeOptions, Encoder, ExportFormat, SampleDepth};
use shared_kernel::id::TrackId;
use shared_kernel::{AudioFormat, Db, Tick};
use std::time::Instant;

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

const SR: u32 = 48_000;
const SECONDS: f32 = 8.0;

/// Tamanho da sessão e do buffer; `EBIKI_SIM_TRACKS` / `EBIKI_SIM_BLOCK` permitem medir a
/// capacidade (ex.: 128 faixas, buffer de 64) sem mudar o teste. Padrão: 32 faixas, 128 amostras.
fn env_or(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
fn tracks() -> usize {
    env_or("EBIKI_SIM_TRACKS", 32).max(4)
}
fn block() -> usize {
    env_or("EBIKI_SIM_BLOCK", 128).clamp(16, audio_engine::MAX_BLOCK)
}

/// Material "musical": fundamental + harmônicos com envelope de notas (não é um seno estático).
fn instrument_file(dir: &std::path::Path, k: usize) -> std::path::PathBuf {
    let n = (SR as f32 * SECONDS) as usize;
    let f0 = 110.0 * 2f32.powf(k as f32 / 12.0 * 3.0);
    let note = SR as usize / 2;
    let (mut l, mut r) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let env = (-((i % note) as f32) / (note as f32 * 0.4)).exp();
        let s = (1..=4)
            .map(|h| (std::f32::consts::TAU * f0 * h as f32 * t).sin() / h as f32)
            .sum::<f32>()
            * 0.18
            * env;
        l.push(s);
        r.push(s * (0.8 + 0.2 * (t * 0.5).sin()));
    }
    let path = dir.join(format!("inst{k}.wav"));
    let opts = EncodeOptions {
        depth: SampleDepth::Float32,
        dither: false,
        ..Default::default()
    };
    let mut enc = Encoder::new(&path, ExportFormat::Wav, SR, 2, &opts).unwrap();
    enc.write(&[&l, &r]).unwrap();
    enc.finish().unwrap();
    path
}

fn edit(st: &mut Studio, target: StripTarget, e: StripEdit) {
    st.execute(on_mixer(EditStrip::new(target, e)), None)
        .unwrap();
}

/// Monta a sessão pesada e devolve o estúdio com o projeto aberto.
fn heavy_session(dir: &std::path::Path) -> Studio {
    let mut st = studio();
    st.create_project(dir, "Simulação", AudioFormat::default())
        .unwrap();
    // 8 fontes distintas, cada uma importada 4 vezes → 32 faixas (fontes deduplicadas)
    let files: Vec<_> = (0..8).map(|k| instrument_file(dir, k)).collect();
    let paths: Vec<_> = (0..tracks())
        .map(|i| files[i % files.len()].clone())
        .collect();
    let v = st.import_audio(&paths, None, Tick(0)).unwrap();
    assert_eq!(v.tracks.len(), tracks());
    assert_eq!(
        v.sources.len(),
        8,
        "importações repetidas viram a mesma fonte"
    );

    // buses: sala (reverb) e eco (delay)
    let reverb = InstrumentPreset::ALL
        .iter()
        .flat_map(|p| p.chain())
        .find(|s| matches!(s.effect, Effect::Reverb(_)));
    let delay = InstrumentPreset::ALL
        .iter()
        .flat_map(|p| p.chain())
        .find(|s| matches!(s.effect, Effect::Delay(_)));
    let mut buses = Vec::new();
    for (name, fx) in [("Sala", reverb), ("Eco", delay)] {
        let bus = Bus {
            id: BusId::new(),
            name: name.into(),
            strip: ChannelStrip::default(),
        };
        let id = bus.id;
        st.execute(on_mixer(CreateBus::new(bus, usize::MAX)), None)
            .unwrap();
        if let Some(slot) = fx {
            edit(
                &mut st,
                StripTarget::Bus(id),
                StripEdit::AddEffect {
                    slot: ctx_mixing::EffectSlot::new(slot.effect),
                    index: None,
                },
            );
        }
        buses.push(id);
    }

    let instruments: Vec<_> = InstrumentPreset::ALL
        .into_iter()
        .filter(|p| *p != InstrumentPreset::MasterBus)
        .collect();
    for (i, t) in v.tracks.iter().enumerate() {
        let tid: TrackId = t.id.parse().unwrap();
        let target = StripTarget::Track(tid);
        for slot in instruments[i % instruments.len()].chain() {
            edit(&mut st, target, StripEdit::AddEffect { slot, index: None });
        }
        for (b, level) in buses.iter().zip([-12.0, -18.0]) {
            edit(
                &mut st,
                target,
                StripEdit::AddSend {
                    send: AuxSend {
                        id: SendId::new(),
                        bus: *b,
                        level: Db(level),
                        pre_fader: false,
                    },
                },
            );
        }
        edit(&mut st, target, StripEdit::Volume { value: Db(-9.0) });
        // edições não destrutivas em 1 de cada 4 faixas
        if i % 4 == 0 {
            let clip: ClipId = t.clips[0].id.parse().unwrap();
            let v = st.clip_split(clip, Tick(960 * 6)).unwrap();
            let parts: Vec<ClipId> = v.tracks[i]
                .clips
                .iter()
                .map(|c| c.id.parse().unwrap())
                .collect();
            assert_eq!(parts.len(), 2, "divisão gera dois clipes");
            for p in &parts {
                st.clip_set_fades(
                    *p,
                    FadeRequest {
                        fade_in_frames: 2_400,
                        fade_out_frames: 4_800,
                        fade_in_shape: FadeShape::EqualPower,
                        fade_out_shape: FadeShape::Linear,
                    },
                    None,
                )
                .unwrap();
            }
            st.clip_set_reversed(parts[1], i % 8 == 0).unwrap();
            st.clip_set_polarity(parts[0], i % 8 == 4).unwrap();
        }
    }
    for slot in InstrumentPreset::MasterBus.chain() {
        edit(
            &mut st,
            StripTarget::Master,
            StripEdit::AddEffect { slot, index: None },
        );
    }
    // andamento variável: 120 → 96 BPM no compasso 3
    st.execute(on_project(SetTempo::new(Tick(960 * 8), 96.0)), None)
        .unwrap();
    st
}

struct Stats {
    mean_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    over_deadline: usize,
    blocks: usize,
}

#[test]
fn heavy_32_track_session_renders_within_real_time_without_allocating() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = heavy_session(dir.path());
    let (h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 0).unwrap();
    st.play().unwrap();

    let (n_tracks, block) = (tracks(), block());
    let frames = (SR as f32 * SECONDS) as usize;
    let blocks = frames / block;
    let mut out = vec![0.0f32; block * 2];
    let mut times = Vec::with_capacity(blocks);
    let (mut peak, mut energy, mut finite) = (0f32, 0f64, true);
    // aquecimento: aplica o grafo e prepara os processadores (fora da medição)
    for _ in 0..8 {
        core.process(&mut out, 2, &[], 0);
    }
    reset_violation_count();
    assert_no_alloc(|| {
        for _ in 0..blocks {
            let t0 = Instant::now();
            core.process(&mut out, 2, &[], 0);
            times.push(t0.elapsed());
            for x in &out {
                finite &= x.is_finite();
                peak = peak.max(x.abs());
                energy += (*x as f64) * (*x as f64);
            }
        }
    });
    let allocations = violation_count();
    let deadline = block as f64 / SR as f64 * 1_000.0;
    let mut ms: Vec<f64> = times.iter().map(|d| d.as_secs_f64() * 1_000.0).collect();
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let s = Stats {
        mean_ms: ms.iter().sum::<f64>() / ms.len() as f64,
        p99_ms: ms[(ms.len() * 99) / 100],
        max_ms: *ms.last().unwrap(),
        over_deadline: ms.iter().filter(|m| **m > deadline).count(),
        blocks: ms.len(),
    };
    let rms = (energy / (blocks * block * 2) as f64).sqrt();
    println!(
        "simulação {n_tracks} faixas · {SECONDS} s · bloco {block} ({deadline:.3} ms): \
         média {:.3} ms ({:.0}% do prazo) · p99 {:.3} ms · máx {:.3} ms · \
         blocos acima do prazo {}/{} · pico {:.3} · RMS {:.4} · alocações {}",
        s.mean_ms,
        s.mean_ms / deadline * 100.0,
        s.p99_ms,
        s.max_ms,
        s.over_deadline,
        s.blocks,
        peak,
        rms,
        allocations
    );

    assert!(finite, "saída com NaN/inf");
    assert!(rms > 1e-3, "a sessão produz som (RMS {rms})");
    assert!(peak <= 1.05, "limiter do master segura o pico: {peak}");
    assert_eq!(allocations, 0, "o caminho de tempo real alocou memória");
    if !cfg!(debug_assertions) {
        assert!(
            s.mean_ms <= deadline * 0.5,
            "CPU média acima de 50% do prazo: {:.3} ms de {deadline:.3} ms",
            s.mean_ms
        );
        assert!(
            s.p99_ms <= deadline,
            "1% dos blocos estoura o prazo: p99 {:.3} ms de {deadline:.3} ms",
            s.p99_ms
        );
    }
}

/// Mesma sessão, duas exportações → amostras idênticas; salvar e reabrir → mesma exportação.
#[test]
fn heavy_session_export_is_deterministic_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut st = heavy_session(dir.path());
    let settings = ExportSettings {
        format: ExportFormatKind::Wav,
        ..Default::default()
    };
    let range = Some((Tick(0), Tick(960 * 4)));
    let export = |st: &mut Studio, name: &str| {
        let f = dir.path().join(name);
        st.export_mix(&f, &settings, range).unwrap();
        infra_codecs::decode_to_f32(&f, SR).unwrap().channels
    };
    let a = export(&mut st, "a.wav");
    let b = export(&mut st, "b.wav");
    assert_eq!(a, b, "render determinístico");
    assert!(a[0].iter().all(|x| x.is_finite()));

    let root = st.project_root().unwrap().to_path_buf();
    st.save().unwrap();
    st.close().unwrap();
    st.open_project(&root).unwrap();
    let v = st.view().unwrap();
    let n = tracks();
    assert_eq!(v.tracks.len(), n);
    assert_eq!(
        v.tracks.iter().map(|t| t.clips.len()).sum::<usize>(),
        n + n.div_ceil(4),
        "divisões preservadas"
    );
    let c = export(&mut st, "c.wav");
    assert_eq!(
        a, c,
        "reabrir o projeto reproduz exatamente a mesma mixagem"
    );
}
