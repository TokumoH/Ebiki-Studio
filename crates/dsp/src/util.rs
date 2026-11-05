//! Funções auxiliares numéricas usadas por todos os processadores.
//!
//! Todas são `#[inline]`, sem alocação e sem `panic`, podendo ser chamadas dentro de `process`.

use shared_kernel::Db;

/// Piso em dB usado pelos medidores e conversões (igual a [`Db::FLOOR`]).
pub const DB_FLOOR: f32 = Db::FLOOR;

/// Converte dB em ganho linear de amplitude (abaixo de [`DB_FLOOR`] ou não finito → 0).
#[inline]
pub fn db_to_lin(db: f32) -> f32 {
    Db(db).to_linear()
}

/// Converte ganho linear em dB, com piso [`DB_FLOOR`] para silêncio, negativos e não finitos.
#[inline]
pub fn lin_to_db(lin: f32) -> f32 {
    Db::from_linear(lin).0
}

/// Limita `v` a `[lo, hi]`; valores não finitos viram `default`.
#[inline]
pub fn clamp_param(v: f32, lo: f32, hi: f32, default: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        default
    }
}

/// Amostra não finita (NaN/±∞) vira silêncio, evitando envenenar estados internos.
#[inline]
pub(crate) fn sanitize(x: f32) -> f32 {
    if x.is_finite() {
        x
    } else {
        0.0
    }
}

/// Zera valores muito pequenos para evitar números desnormalizados (lentos em x86).
#[inline]
pub(crate) fn flush(x: f32) -> f32 {
    if x.abs() < 1.0e-20 {
        0.0
    } else {
        x
    }
}

/// Versão `f64` de [`flush`].
#[inline]
pub(crate) fn flush64(x: f64) -> f64 {
    if x.abs() < 1.0e-30 {
        0.0
    } else {
        x
    }
}

/// Coeficiente de um filtro de um polo com constante de tempo `ms` (63 % do caminho).
/// `ms <= 0` resulta em 0 (resposta instantânea).
#[inline]
pub(crate) fn time_coef(ms: f32, sample_rate: f32) -> f32 {
    let samples = ms * 0.001 * sample_rate;
    if samples.is_finite() && samples > 0.0 {
        (-1.0 / samples).exp()
    } else {
        0.0
    }
}

/// Taxa de amostragem segura (finita, entre 8 kHz e 768 kHz; fora disso usa 48 kHz).
#[inline]
pub(crate) fn safe_sr(sr: f32) -> f32 {
    if sr.is_finite() && (8_000.0..=768_000.0).contains(&sr) {
        sr
    } else {
        48_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_conversions_match_shared_kernel() {
        assert!((db_to_lin(-6.0) - 0.501_187).abs() < 1e-5);
        assert_eq!(db_to_lin(f32::NAN), 0.0);
        assert_eq!(lin_to_db(0.0), DB_FLOOR);
        assert!(lin_to_db(1.0).abs() < 1e-6);
    }

    #[test]
    fn clamp_param_rejects_non_finite() {
        assert_eq!(clamp_param(f32::NAN, 0.0, 1.0, 0.5), 0.5);
        assert_eq!(clamp_param(f32::INFINITY, 0.0, 1.0, 0.5), 0.5);
        assert_eq!(clamp_param(3.0, 0.0, 1.0, 0.5), 1.0);
    }

    #[test]
    fn time_coef_reaches_63_percent_after_time_constant() {
        let sr = 48_000.0;
        let a = time_coef(10.0, sr);
        let mut y = 0.0f32;
        for _ in 0..480 {
            y = a * y + (1.0 - a) * 1.0;
        }
        assert!((y - 0.632).abs() < 0.01, "y = {y}");
        assert_eq!(time_coef(0.0, sr), 0.0);
    }

    #[test]
    fn helpers_sanitize_and_flush() {
        assert_eq!(sanitize(f32::NAN), 0.0);
        assert_eq!(sanitize(0.5), 0.5);
        assert_eq!(flush(1e-30), 0.0);
        assert_eq!(flush64(1e-40), 0.0);
        assert_eq!(safe_sr(f32::NAN), 48_000.0);
        assert_eq!(safe_sr(44_100.0), 44_100.0);
    }
}
