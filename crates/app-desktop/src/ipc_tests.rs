//! Testes de IPC com o runtime simulado do Tauri: os comandos são chamados com o MESMO JSON que a UI
//! envia (`ui/src/ipc/api.ts`), validando nomes de argumentos (camelCase) e formatos de resposta.

use crate::state::AppState;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

fn app() -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = crate::with_commands(mock_builder())
        .manage(AppState::new(None))
        .build(tauri::generate_context!())
        .expect("app");
    let w = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela");
    (app, w)
}

fn call(w: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Result<Value, Value> {
    get_ipc_response(
        w,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: origin(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().unwrap_or(Value::Null))
}

fn origin() -> tauri::Url {
    let u = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    u.parse().unwrap()
}

fn ok(w: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Value {
    call(w, cmd, args).unwrap_or_else(|e| panic!("{cmd} falhou: {e}"))
}

fn tone(path: &std::path::Path) {
    let s: Vec<f32> = (0..24_000).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
    let opts = infra_codecs::EncodeOptions {
        depth: infra_codecs::SampleDepth::Float32,
        dither: false,
        ..Default::default()
    };
    let mut e = infra_codecs::Encoder::new(path, infra_codecs::ExportFormat::Wav, 48_000, 1, &opts)
        .unwrap();
    e.write(&[&s]).unwrap();
    e.finish().unwrap();
}

#[test]
fn commands_accept_the_ui_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let (_app, w) = app();
    let d = dir.path().to_string_lossy().to_string();

    let v = ok(
        &w,
        "project_create",
        json!({ "req": { "dir": d, "name": "IPC", "sampleRate": 48000, "bitDepth": 24 } }),
    );
    assert_eq!(v["project"]["name"], "IPC");
    let v = ok(&w, "track_create", json!({ "name": "Voz", "index": null }));
    let tid = v["tracks"][0]["id"].as_str().unwrap().to_owned();
    ok(
        &w,
        "track_rename",
        json!({ "trackId": tid, "name": "Voz principal" }),
    );
    let target = json!({ "kind": "track", "id": tid });
    ok(
        &w,
        "mixer_edit",
        json!({ "target": target, "edit": { "op": "volume", "value": -6.0 }, "gesture": 1 }),
    );
    ok(
        &w,
        "mixer_edit",
        json!({ "target": { "kind": "master" }, "edit": { "op": "addEffect", "slot": { "id": "0196f7b4-0000-7000-8000-000000000001", "bypass": false, "effect": { "type": "limiter", "params": { "ceilingDb": -1.0, "inputGainDb": 0.0, "releaseMs": 80.0 } } }, "index": null }, "gesture": null }),
    );
    let v = ok(
        &w,
        "recording_track",
        json!({ "req": { "trackId": tid, "armed": true, "input": { "left": 0, "right": null } } }),
    );
    assert_eq!(v["tracks"][0]["recording"]["armed"], true);
    ok(
        &w,
        "recording_metronome",
        json!({ "config": { "enabled": true, "volume": 0.5, "subdivision": 2, "accentFirstBeat": true, "countInBars": 1 } }),
    );
    ok(&w, "recording_punch", json!({ "range": [960, 3840] }));
    ok(&w, "recording_loop", json!({ "range": null }));
    ok(&w, "tempo_set", json!({ "tick": 3840, "bpm": 90.0 }));
    ok(
        &w,
        "meter_set",
        json!({ "tick": 3840, "numerator": 3, "denominator": 4 }),
    );
    let presets = ok(&w, "mixer_presets", json!({}));
    let first = presets[0]["id"].as_str().unwrap().to_owned();
    ok(
        &w,
        "mixer_apply_preset",
        json!({ "target": target, "preset": first }),
    );
    let bus = ok(&w, "bus_create", json!({ "name": "Reverb" }));
    let bid = bus["buses"][0]["id"].as_str().unwrap().to_owned();
    ok(&w, "bus_rename", json!({ "busId": bid, "name": "Sala" }));

    let wav = dir.path().join("tom.wav");
    tone(&wav);
    let v = ok(
        &w,
        "audio_import",
        json!({ "paths": [wav.to_string_lossy()], "trackId": tid, "atTick": 0 }),
    );
    let clip = v["tracks"][0]["clips"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let src = v["sources"][0]["id"].as_str().unwrap().to_owned();
    ok(
        &w,
        "clip_move",
        json!({ "clipId": clip, "trackId": tid, "start": 960, "gesture": null }),
    );
    ok(
        &w,
        "clip_set_gain",
        json!({ "clipId": clip, "gainDb": -3.0, "gesture": null }),
    );
    let peaks = get_ipc_response(
        &w,
        InvokeRequest {
            cmd: "waveform_peaks".into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: origin(),
            body: InvokeBody::Json(json!({ "sourceId": src, "framesPerPeak": 256 })),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .expect("picos");
    match peaks {
        tauri::ipc::InvokeResponseBody::Raw(bytes) => {
            assert_eq!(bytes.len(), 24_000usize.div_ceil(256) * 2 * 4)
        }
        other => panic!("esperado binário, veio {other:?}"),
    }

    let v = ok(&w, "history_undo", json!({}));
    assert_eq!(v["history"]["canRedo"], true);
    ok(&w, "history_redo", json!({}));
    ok(&w, "history_end_gesture", json!({}));
    let v = ok(&w, "project_save", json!({}));
    assert_eq!(v["history"]["dirty"], false);
    let formats = ok(&w, "export_formats", json!({}));
    assert!(formats.as_array().unwrap().iter().any(|f| f == "wav"));
    let out = dir.path().join("mix.flac");
    ok(
        &w,
        "export_mix",
        json!({ "path": out.to_string_lossy(), "settings": { "format": "flac", "bitDepth": 24, "mp3Kbps": null, "vorbisQuality": 0.6, "normalize": true }, "range": null }),
    );
    assert!(out.exists());
    let stems = ok(
        &w,
        "export_stems",
        json!({ "dir": dir.path().join("stems").to_string_lossy(), "settings": { "format": "wav", "bitDepth": 16, "mp3Kbps": null, "vorbisQuality": 0.6, "normalize": false } }),
    );
    assert_eq!(stems.as_array().unwrap().len(), 1);

    // erros tipados chegam como { code, message }
    let err = call(
        &w,
        "track_rename",
        json!({ "trackId": "não-é-uuid", "name": "x" }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "api.invalid_argument");
    let err = call(
        &w,
        "audio_import",
        json!({ "paths": ["relativo.wav"], "trackId": null, "atTick": 0 }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "api.invalid_argument");
    let err = call(
        &w,
        "track_delete",
        json!({ "trackId": "0196f7b4-0000-7000-8000-00000000ffff" }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "project.track_not_found");

    let root = ok(&w, "project_view", json!({}))["project"]["root"]
        .as_str()
        .unwrap()
        .to_owned();
    ok(&w, "project_close", json!({}));
    assert_eq!(
        ok(&w, "project_recovery_check", json!({ "path": root })),
        json!(false)
    );
    let v = ok(&w, "project_open", json!({ "path": root }));
    assert_eq!(v["tracks"][0]["name"], "Voz principal");
}

fn raw(w: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Vec<u8> {
    let r = get_ipc_response(
        w,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: origin(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .unwrap_or_else(|e| panic!("{cmd} falhou: {e:?}"));
    match r {
        tauri::ipc::InvokeResponseBody::Raw(bytes) => bytes,
        other => panic!("esperado binário, veio {other:?}"),
    }
}

/// Fase 3: os comandos de edição aceitam o MESMO JSON de `ui/src/ipc/api.ts`.
#[test]
fn edit_commands_accept_the_ui_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let (_app, w) = app();
    let d = dir.path().to_string_lossy().to_string();
    ok(
        &w,
        "project_create",
        json!({ "req": { "dir": d, "name": "Edição", "sampleRate": 48000, "bitDepth": 24 } }),
    );
    let wav = dir.path().join("tom.wav");
    tone(&wav);
    let v = ok(
        &w,
        "audio_import",
        json!({ "paths": [wav.to_string_lossy()], "trackId": null, "atTick": 0 }),
    );
    let clip = v["tracks"][0]["clips"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let src = v["sources"][0]["id"].as_str().unwrap().to_owned();
    assert_eq!(v["tracks"][0]["clips"][0]["sourceFrames"], 24_000);
    assert_eq!(v["tracks"][0]["clips"][0]["fadeInShape"], "linear");

    let v = ok(
        &w,
        "clip_trim",
        json!({ "clipId": clip, "start": 0, "lengthFrames": 20000, "gesture": 3 }),
    );
    assert_eq!(v["tracks"][0]["clips"][0]["lengthFrames"], 20_000);
    let v = ok(
        &w,
        "clip_set_fades",
        json!({ "clipId": clip, "fades": { "fadeInFrames": 480, "fadeOutFrames": 960, "fadeInShape": "equalPower", "fadeOutShape": "linear" }, "gesture": null }),
    );
    assert_eq!(v["tracks"][0]["clips"][0]["fadeOutFrames"], 960);
    assert_eq!(v["tracks"][0]["clips"][0]["fadeInShape"], "equalPower");
    let v = ok(
        &w,
        "clip_set_polarity",
        json!({ "clipId": clip, "inverted": true }),
    );
    assert_eq!(v["tracks"][0]["clips"][0]["polarityInverted"], true);
    let v = ok(
        &w,
        "clip_set_reversed",
        json!({ "clipId": clip, "reversed": true }),
    );
    assert_eq!(v["tracks"][0]["clips"][0]["reversed"], true);
    let v = ok(
        &w,
        "clip_normalize",
        json!({ "clipId": clip, "targetDbfs": -1.0 }),
    );
    assert!(v["tracks"][0]["clips"][0]["gain"].as_f64().unwrap() > 5.0);
    // 0,2 s = 192 ticks a 120 BPM
    let v = ok(&w, "clip_split", json!({ "clipId": clip, "atTick": 192 }));
    let clips = v["tracks"][0]["clips"].as_array().unwrap();
    assert_eq!(clips.len(), 2);
    let second = clips[1]["id"].as_str().unwrap().to_owned();
    let err = call(
        &w,
        "clip_crossfade",
        json!({ "aId": clip, "bId": second, "shape": "equalPower" }),
    )
    .unwrap_err();
    assert_eq!(
        err["code"], "project.clips_do_not_overlap",
        "contíguos, sem sobreposição"
    );
    let err = call(
        &w,
        "clip_trim",
        json!({ "clipId": clip, "start": -1, "lengthFrames": 10, "gesture": null }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "api.invalid_argument");

    let bytes = raw(
        &w,
        "source_spectrogram",
        json!({ "sourceId": src, "params": { "fftSize": 1024, "hop": 256, "bins": 64 } }),
    );
    assert_eq!(&bytes[0..4], b"AMSG");
    assert_eq!(bytes.len(), 40 + 24_000usize.div_ceil(256) * 64);
    let bytes = raw(
        &w,
        "source_spectrogram",
        json!({ "sourceId": src, "params": null }),
    );
    assert_eq!(bytes.len(), 40 + 24_000usize.div_ceil(512) * 128);

    let v = ok(&w, "history_undo", json!({}));
    assert_eq!(v["tracks"][0]["clips"].as_array().unwrap().len(), 1);
}
