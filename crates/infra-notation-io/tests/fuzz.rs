//! Testes estilo fuzz: bytes aleatórios e arquivos válidos corrompidos nunca causam pânico.
mod common;

use infra_notation_io::midi::{export_midi, import_midi, MidiExportOptions, MidiImportOptions};
use infra_notation_io::musicxml::{export_musicxml, export_mxl, import_musicxml, import_mxl};
use infra_notation_io::{gp, import_any};
use proptest::prelude::*;

fn corpus() -> Vec<Vec<u8>> {
    let s = common::rich_score();
    vec![
        export_musicxml(&s).into_bytes(),
        export_mxl(&s).unwrap(),
        export_midi(&s, &MidiExportOptions::default()).unwrap(),
        include_bytes!("data/sample.gpif").to_vec(),
        include_bytes!("data/sample.musicxml").to_vec(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = import_any(&bytes);
        let _ = import_midi(&bytes, &MidiImportOptions::default());
        let _ = import_musicxml(&bytes);
        let _ = import_mxl(&bytes);
        let _ = gp::import_gpif(&bytes);
        let _ = gp::import_gp(&bytes);
    }

    #[test]
    fn prefixed_random_bytes_never_panic(
        kind in 0usize..4,
        tail in proptest::collection::vec(any::<u8>(), 0..1024),
    ) {
        let prefix: &[u8] = match kind {
            0 => b"MThd\x00\x00\x00\x06\x00\x01\x00\x02\x01\xe0MTrk",
            1 => b"PK\x03\x04",
            2 => b"<score-partwise version=\"4.0\"><part id=\"P1\"><measure>",
            _ => b"<GPIF><Tracks><Track id=\"0\">",
        };
        let mut bytes = prefix.to_vec();
        bytes.extend(tail);
        let _ = import_any(&bytes);
    }

    #[test]
    fn corrupted_valid_files_never_panic(
        which in 0usize..5,
        edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..16),
        cut in any::<prop::sample::Index>(),
    ) {
        let mut bytes = corpus()[which].clone();
        for (idx, b) in edits {
            let i = idx.index(bytes.len());
            bytes[i] = b;
        }
        let _ = import_any(&bytes);
        let n = cut.index(bytes.len() + 1);
        let _ = import_any(&bytes[..n]);
    }
}
