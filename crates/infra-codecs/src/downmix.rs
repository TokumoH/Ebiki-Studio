//! Redução de canais (downmix) para estéreo.
//!
//! Regra: mono continua mono, estéreo continua estéreo e qualquer arquivo com mais de dois canais
//! é reduzido a estéreo com os coeficientes abaixo (inspirados na ITU-R BS.775, sem normalização —
//! o pipeline interno é `f32` e tem folga acima de 0 dBFS; o limiter/normalização ficam com o
//! usuário):
//!
//! | Canal de origem                                  | Esquerdo | Direito |
//! |--------------------------------------------------|----------|---------|
//! | Frontal esquerdo / direito                       | 1,0      | 1,0     |
//! | Central (frontal, traseiro, superior, inferior)  | 0,7071   | 0,7071  |
//! | Surround/lateral/traseiro/superior de um lado    | 0,7071   | 0,7071  |
//! | LFE (1 e 2)                                      | 0        | 0       |
//! | Desconhecido                                     | 0,5      | 0,5     |
//!
//! Para layouts sem posições (canais "discretos"), o canal 0 vai para a esquerda, o 1 para a
//! direita e os demais alternam esquerda/direita com 0,7071.

pub(crate) const MINUS_3DB: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Ganhos (esquerdo, direito) para cada bit de posição do symphonia (`Position`, bit 0 = FL).
fn gains_for_position_bit(bit: u32) -> (f32, f32) {
    match bit {
        0 => (1.0, 0.0),                                         // FRONT_LEFT
        1 => (0.0, 1.0),                                         // FRONT_RIGHT
        2 | 8 | 11 | 13 | 16 | 21 => (MINUS_3DB, MINUS_3DB),     // centrais
        3 | 18 => (0.0, 0.0),                                    // LFE1, LFE2
        4 | 6 | 9 | 12 | 15 | 19 | 22 | 24 => (MINUS_3DB, 0.0),  // lado esquerdo
        5 | 7 | 10 | 14 | 17 | 20 | 23 | 25 => (0.0, MINUS_3DB), // lado direito
        _ => (0.5, 0.5),
    }
}

/// Ganhos por canal de origem, na ordem canônica (bits de posição em ordem crescente).
pub(crate) fn stereo_gains_for_mask(mask: u64) -> Vec<(f32, f32)> {
    (0..64u32)
        .filter(|b| mask & (1u64 << b) != 0)
        .map(gains_for_position_bit)
        .collect()
}

/// Ganhos para `n` canais sem posição conhecida.
pub(crate) fn stereo_gains_discrete(n: usize) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| match i {
            0 => (1.0, 0.0),
            1 => (0.0, 1.0),
            i if i % 2 == 0 => (MINUS_3DB, 0.0),
            _ => (0.0, MINUS_3DB),
        })
        .collect()
}

/// Mistura `src` (planos de origem) em `dst` (1 ou 2 planos), acrescentando `frames` quadros.
///
/// - `dst.len() == src.len()` (1 ou 2): cópia direta;
/// - `dst.len() == 1` e `src.len() > 1`: média dos canais;
/// - `dst.len() == 2` e `src.len() == 1`: duplica o mono;
/// - `dst.len() == 2` e `src.len() > 2`: aplica `gains`.
///
/// Amostras não finitas (`NaN`/infinito) viram 0.
pub(crate) fn mix_into(
    src: &[Vec<f32>],
    frames: usize,
    gains: &[(f32, f32)],
    dst: &mut [Vec<f32>],
) {
    let clean = |x: f32| if x.is_finite() { x } else { 0.0 };
    match (src.len(), dst.len()) {
        (0, _) | (_, 0) => {}
        (s, d) if s == d => {
            for (out, inp) in dst.iter_mut().zip(src) {
                out.extend(inp[..frames].iter().map(|&x| clean(x)));
            }
        }
        (s, 1) => {
            let inv = 1.0 / s as f32;
            let out = &mut dst[0];
            out.reserve(frames);
            for f in 0..frames {
                let sum: f32 = src.iter().map(|c| clean(c[f])).sum();
                out.push(sum * inv);
            }
        }
        (1, 2) => {
            for out in dst.iter_mut() {
                out.extend(src[0][..frames].iter().map(|&x| clean(x)));
            }
        }
        (_, _) => {
            let (l, r) = dst.split_at_mut(1);
            let (l, r) = (&mut l[0], &mut r[0]);
            l.reserve(frames);
            r.reserve(frames);
            for f in 0..frames {
                let (mut sl, mut sr) = (0.0f32, 0.0f32);
                for (c, &(gl, gr)) in src.iter().zip(gains) {
                    let x = clean(c[f]);
                    sl += x * gl;
                    sr += x * gr;
                }
                l.push(sl);
                r.push(sr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_one_downmix_uses_documented_coefficients() {
        // 5.1: FL FR FC LFE RL RR (bits 0..=5)
        let g = stereo_gains_for_mask(0b11_1111);
        assert_eq!(g.len(), 6);
        let src: Vec<Vec<f32>> = (0..6).map(|i| vec![0.1 * (i + 1) as f32]).collect();
        let mut dst = vec![Vec::new(), Vec::new()];
        mix_into(&src, 1, &g, &mut dst);
        let expected_l = 0.1 + 0.3 * MINUS_3DB + 0.5 * MINUS_3DB;
        let expected_r = 0.2 + 0.3 * MINUS_3DB + 0.6 * MINUS_3DB;
        assert!((dst[0][0] - expected_l).abs() < 1e-6);
        assert!((dst[1][0] - expected_r).abs() < 1e-6);
    }

    #[test]
    fn lfe_is_dropped() {
        let g = stereo_gains_for_mask(1 << 3);
        assert_eq!(g, vec![(0.0, 0.0)]);
    }

    #[test]
    fn mono_and_stereo_pass_through_and_nan_is_zeroed() {
        let src = vec![vec![0.5, f32::NAN], vec![-0.5, f32::INFINITY]];
        let mut dst = vec![Vec::new(), Vec::new()];
        mix_into(&src, 2, &[], &mut dst);
        assert_eq!(dst, vec![vec![0.5, 0.0], vec![-0.5, 0.0]]);

        let mut mono = vec![Vec::new()];
        mix_into(&src, 1, &[], &mut mono);
        assert_eq!(mono, vec![vec![0.0]]);

        let mut dup = vec![Vec::new(), Vec::new()];
        mix_into(&[vec![0.25]], 1, &[], &mut dup);
        assert_eq!(dup, vec![vec![0.25], vec![0.25]]);
    }

    #[test]
    fn discrete_layout_alternates_sides() {
        let g = stereo_gains_discrete(4);
        assert_eq!(g[0], (1.0, 0.0));
        assert_eq!(g[1], (0.0, 1.0));
        assert_eq!(g[2], (MINUS_3DB, 0.0));
        assert_eq!(g[3], (0.0, MINUS_3DB));
    }
}
