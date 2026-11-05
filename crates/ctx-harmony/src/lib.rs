//! Bounded context **Harmony**: teoria musical aplicada à composição.
//!
//! - `domain`: puro (sem I/O). Escalas e modos como conjuntos de intervalos, campos harmônicos
//!   **derivados** (nunca armazenados por tom), acordes com cifra popular e graus romanos,
//!   progressões em graus relativos (ADR-0007), catálogo embutido, motor de sugestões atrás da
//!   porta [`SuggestionProvider`] e realização de voicings em notas MIDI.
//! - `commands`: operações desfazíveis sobre o agregado [`HarmonyWorkspace`] (progressões do usuário).
//!
//! O DDL do banco da biblioteca (`library.db`) está em [`LIBRARY_SCHEMA_V1`]; os dados de semente
//! vêm do próprio catálogo Rust (ver `docs/specs/2026-11-05-fase-5-harmonia-e-4-bases.md`).

pub mod commands;
pub mod domain;

pub use domain::*;

/// DDL versão 1 do `library.db` (harmonia + bases). Embutido para a camada de persistência.
pub const LIBRARY_SCHEMA_V1: &str = include_str!("../sql/library_v1.sql");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_declares_every_normalized_table_and_view() {
        for name in [
            "seed_meta",
            "scale_family",
            "scale_family_interval",
            "mode",
            "chord_quality",
            "chord_quality_interval",
            "tension",
            "genre",
            "progression",
            "builtin_progression",
            "user_progression",
            "progression_genre",
            "progression_step",
            "progression_step_tension",
            "instrument",
            "tag",
            "style_pattern",
            "drum_pattern",
            "bass_pattern",
            "base",
            "base_audio",
            "base_recipe",
            "base_instrument",
            "base_tag",
        ] {
            assert!(
                LIBRARY_SCHEMA_V1.contains(&format!("CREATE TABLE {name} (")),
                "tabela {name} ausente"
            );
        }
        for view in [
            "v_mode_interval",
            "v_progression_summary",
            "v_progression_step",
            "v_base_summary",
        ] {
            assert!(LIBRARY_SCHEMA_V1.contains(&format!("CREATE VIEW {view} AS")));
        }
        // A transação é responsabilidade da camada de persistência.
        assert!(!LIBRARY_SCHEMA_V1.contains("BEGIN TRANSACTION"));
        assert!(!LIBRARY_SCHEMA_V1
            .lines()
            .any(|l| l.trim_start().starts_with("COMMIT")));
    }

    #[test]
    fn seed_vocabulary_fits_schema_constraints() {
        // genre.code: GLOB '[a-z]*' e apenas [a-z0-9_]
        for g in Genre::ALL {
            let c = g.code();
            assert!(c.starts_with(|ch: char| ch.is_ascii_lowercase()), "{c}");
            assert!(c
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'));
        }
        // mode: (family, rotation) único e rotação 0–6
        let mut seen = std::collections::HashSet::new();
        for m in Mode::ALL {
            assert!(m.rotation() <= 6);
            assert!(seen.insert((m.family(), m.rotation())));
        }
        // chord_quality_interval: semitons 0–11 sem repetição
        for q in ChordQuality::ALL {
            let set: std::collections::HashSet<_> = q.intervals().iter().collect();
            assert_eq!(set.len(), q.intervals().len());
            assert!(q.intervals().iter().all(|s| *s < 12));
        }
        // tension.semitones é UNIQUE
        let t: std::collections::HashSet<_> = Tension::ALL.iter().map(|t| t.semitones()).collect();
        assert_eq!(t.len(), Tension::ALL.len());
        // progression_step: posição 0–63, beats 1–64, acidente −2..2
        for p in builtin_catalog() {
            assert!(p.steps.len() <= 64);
            assert!(p.steps.iter().all(|s| (1..=64).contains(&s.beats)));
        }
    }
}
