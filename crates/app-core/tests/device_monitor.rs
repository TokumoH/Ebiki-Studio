//! "Ouvir entrada" e medidores de entrada pela camada de aplicação.
mod support;
use app_core::device_monitor::DeviceMonitorView;
use audio_engine::{create_engine, offline, EngineConfig};
use support::*;

fn mon(left: usize, right: Option<usize>, gain_db: f64) -> Option<DeviceMonitorView> {
    Some(DeviceMonitorView {
        left,
        right,
        gain_db,
    })
}

#[test]
fn starts_off_and_validates_the_route() {
    let mut st = studio();
    assert_eq!(
        st.set_device_monitor(mon(0, None, 0.0)).unwrap_err().code,
        "app.no_engine"
    );
    let (h, _core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 2).unwrap();
    assert_eq!(st.device_monitor(), None);
    assert_eq!(
        st.set_device_monitor(mon(2, None, 0.0)).unwrap_err().code,
        "app.input_unavailable"
    );
    assert_eq!(
        st.set_device_monitor(mon(0, Some(5), 0.0))
            .unwrap_err()
            .code,
        "app.input_unavailable"
    );
    assert_eq!(
        st.set_device_monitor(mon(0, None, f64::NAN))
            .unwrap_err()
            .code,
        "app.invalid"
    );
    // ganho fora da faixa é limitado
    let v = st
        .set_device_monitor(mon(0, Some(1), 30.0))
        .unwrap()
        .unwrap();
    assert_eq!(v.gain_db, 6.0);
    assert_eq!(st.set_device_monitor(None).unwrap(), None);
    assert_eq!(st.device_monitor(), None);
}

#[test]
fn input_is_heard_and_metered_without_a_project() {
    let mut st = studio();
    let (h, mut core) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 1).unwrap();
    st.set_device_monitor(mon(0, None, 0.0)).unwrap();
    let input = vec![0.5f32; 4_800];
    let (l, r) = offline::render_with_input(&mut core, &input, 1, 256);
    assert!((l[l.len() - 1] - 0.5).abs() < 1e-4 && (r[r.len() - 1] - 0.5).abs() < 1e-4);
    let m = st.meters();
    assert_eq!(m.inputs.len(), 1);
    assert!((m.inputs[0] - 0.5).abs() < 1e-6);
    assert_eq!(m.master, [0.0, 0.0], "o monitor não passa pelo master");
}

#[test]
fn survives_a_device_change_only_if_the_route_still_exists() {
    let mut st = studio();
    let (h, _c) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h, 2).unwrap();
    st.set_device_monitor(mon(0, Some(1), -6.0)).unwrap();
    let (h2, _c2) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h2, 2).unwrap();
    assert_eq!(st.device_monitor(), mon(0, Some(1), -6.0));
    // novo dispositivo mono: a rota estéreo deixa de existir → desliga
    let (h3, _c3) = create_engine(EngineConfig {
        sample_rate: SR,
        ..Default::default()
    });
    st.attach_engine(h3, 1).unwrap();
    assert_eq!(st.device_monitor(), None);
}
