//! Prévia (audição de Harmonia, Bases e Partitura) — Pure Fabrication de tempo real.
//!
//! Toca um trecho estéreo pronto na saída do **dispositivo**, depois do master (como o "Testar som"):
//! sai no fone escolhido no app, não passa pela mixagem, não é gravada nem exportada. O áudio chega
//! já alocado (fora do tempo real) e volta para ser liberado fora dele quando acaba ou é trocado.

/// Trecho estéreo intercalado (L, R, L, R…) na taxa da engine.
#[derive(Debug, Default)]
pub struct PreviewClip {
    pub samples: Vec<f32>,
}

impl PreviewClip {
    /// Monta a partir de canais separados (mono vira os dois lados; o menor canal manda).
    pub fn from_planar(channels: &[Vec<f32>]) -> Self {
        let l = channels.first().map(Vec::as_slice).unwrap_or(&[]);
        let r = channels.get(1).map(Vec::as_slice).unwrap_or(l);
        let n = l.len().min(r.len());
        let mut samples = Vec::with_capacity(n * 2);
        for i in 0..n {
            samples.push(if l[i].is_finite() { l[i] } else { 0.0 });
            samples.push(if r[i].is_finite() { r[i] } else { 0.0 });
        }
        Self { samples }
    }

    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }
}

/// Reprodutor no núcleo: posição no trecho atual. Sem alocação.
#[derive(Default)]
pub(crate) struct Preview {
    clip: Option<Box<PreviewClip>>,
    pos: usize,
}

impl Preview {
    /// Troca o trecho (devolve o anterior para liberar fora do tempo real).
    pub(crate) fn play(&mut self, clip: Box<PreviewClip>) -> Option<Box<PreviewClip>> {
        self.pos = 0;
        self.clip.replace(clip)
    }

    /// Para (devolve o trecho para liberar fora do tempo real).
    pub(crate) fn stop(&mut self) -> Option<Box<PreviewClip>> {
        self.pos = 0;
        self.clip.take()
    }

    #[cfg(test)]
    pub(crate) fn is_playing(&self) -> bool {
        self.clip.is_some()
    }

    /// Soma o próximo pedaço em `out` (intercalado, `out_channels` canais). Devolve o trecho quando
    /// termina, para o chamador descartar fora do tempo real.
    pub(crate) fn add_to(
        &mut self,
        out: &mut [f32],
        out_channels: usize,
    ) -> Option<Box<PreviewClip>> {
        let clip = self.clip.as_ref()?;
        if out_channels == 0 {
            return None;
        }
        let frames = clip.frames();
        for o in out.chunks_exact_mut(out_channels) {
            if self.pos >= frames {
                break;
            }
            let (l, r) = (clip.samples[self.pos * 2], clip.samples[self.pos * 2 + 1]);
            if out_channels == 1 {
                o[0] += 0.5 * (l + r);
            } else {
                o[0] += l;
                o[1] += r;
            }
            self.pos += 1;
        }
        if self.pos >= frames {
            self.pos = 0;
            return self.clip.take();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_the_clip_across_blocks_and_hands_it_back_at_the_end() {
        let mut p = Preview::default();
        p.play(Box::new(PreviewClip::from_planar(&[
            vec![0.5; 5],
            vec![-0.5; 5],
        ])));
        let mut out = vec![0.0f32; 3 * 2];
        assert!(p.add_to(&mut out, 2).is_none());
        assert_eq!(out, vec![0.5, -0.5, 0.5, -0.5, 0.5, -0.5]);
        let mut out = vec![0.1f32; 3 * 2];
        let done = p.add_to(&mut out, 2);
        assert!(
            done.is_some(),
            "terminou: devolve para liberar fora do tempo real"
        );
        assert_eq!(
            out,
            vec![0.6, -0.4, 0.6, -0.4, 0.1, 0.1],
            "soma por cima e para no fim"
        );
        assert!(!p.is_playing());
    }

    #[test]
    fn mono_clip_goes_to_both_sides_and_mono_output_averages() {
        let mut p = Preview::default();
        p.play(Box::new(PreviewClip::from_planar(&[vec![0.4, f32::NAN]])));
        let mut out = vec![0.0f32; 2];
        p.add_to(&mut out, 1);
        assert!(
            (out[0] - 0.4).abs() < 1e-6 && out[1] == 0.0,
            "NaN vira silêncio"
        );
    }

    #[test]
    fn stop_and_replace_return_the_old_clip() {
        let mut p = Preview::default();
        assert!(p
            .play(Box::new(PreviewClip::from_planar(&[vec![0.1; 10]])))
            .is_none());
        assert!(p
            .play(Box::new(PreviewClip::from_planar(&[vec![0.2; 10]])))
            .is_some());
        assert!(p.stop().is_some());
        assert!(p.stop().is_none());
    }
}
