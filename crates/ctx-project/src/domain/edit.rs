//! Edição não destrutiva de clipes (Fase 3 — RF-3.1 a RF-3.3).
//!
//! Tudo aqui é puro: calcula o novo estado de um clipe a partir do agregado, sem I/O. A fonte de
//! áudio nunca é tocada (RNF-07): cortar/dividir só mudam a janela `[offset, offset + length)`, e
//! fades/polaridade/reverso são aplicados pela engine na reprodução.

use super::{Clip, ClipId, Project, ProjectError};
use serde::{Deserialize, Serialize};
pub use shared_kernel::FadeShape;
use shared_kernel::Tick;

/// Processamento não destrutivo de um clipe (Value Object).
///
/// Invariante: `fade_in_frames + fade_out_frames <= length_frames` do clipe.
/// Os fades são medidos na timeline (após o reverso).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipEdit {
    pub fade_in_frames: u64,
    pub fade_out_frames: u64,
    pub fade_in_shape: FadeShape,
    pub fade_out_shape: FadeShape,
    pub polarity_inverted: bool,
    pub reversed: bool,
}

impl ClipEdit {
    pub fn validate(&self, length_frames: u64) -> Result<(), ProjectError> {
        match self.fade_in_frames.checked_add(self.fade_out_frames) {
            Some(total) if total <= length_frames => Ok(()),
            _ => Err(ProjectError::InvalidFade),
        }
    }

    /// Reduz os fades para caber em `length_frames` (o fade de entrada tem prioridade).
    pub fn clamped_to(self, length_frames: u64) -> Self {
        let fade_in_frames = self.fade_in_frames.min(length_frames);
        let fade_out_frames = self.fade_out_frames.min(length_frames - fade_in_frames);
        Self {
            fade_in_frames,
            fade_out_frames,
            ..self
        }
    }
}

impl Project {
    fn sample_rate(&self) -> u32 {
        self.format.sample_rate.hz()
    }

    /// Início do clipe na timeline, em amostras.
    pub fn clip_start_frame(&self, clip: &Clip) -> u64 {
        self.tempo.tick_to_frame(clip.start, self.sample_rate())
    }

    /// Fim do clipe na timeline, em amostras (sem overflow).
    pub fn clip_end_frame(&self, clip: &Clip) -> u64 {
        self.clip_start_frame(clip)
            .saturating_add(clip.length_frames)
    }

    /// Substitui um clipe (mesmo id, mesma faixa e posição), validando as invariantes.
    /// Retorna o clipe anterior. Falha não altera o estado.
    pub fn replace_clip(&mut self, clip: Clip) -> Result<Clip, ProjectError> {
        let (ti, ci) = self.find_clip(clip.id)?;
        self.check_clip(&clip)?;
        Ok(std::mem::replace(&mut self.tracks[ti].clips[ci], clip))
    }

    /// Insere `clip` logo após o clipe `after`, na mesma faixa.
    pub fn insert_clip_after(&mut self, after: ClipId, clip: Clip) -> Result<(), ProjectError> {
        let (ti, ci) = self.find_clip(after)?;
        let track = self.tracks[ti].id;
        self.insert_clip(track, clip, ci + 1).map(|_| ())
    }

    /// Calcula o clipe recortado: nova posição de início na timeline e nova duração.
    ///
    /// Arrastar a borda esquerda muda `start` (e o offset na fonte); a direita muda `length`.
    /// Com o clipe reverso, a borda esquerda da timeline corresponde ao fim da região na fonte.
    pub fn trimmed_clip(
        &self,
        id: ClipId,
        start: Tick,
        length_frames: u64,
    ) -> Result<Clip, ProjectError> {
        let c = self.clip(id)?;
        if start.0 < 0 {
            return Err(ProjectError::InvalidPosition);
        }
        if length_frames == 0 {
            return Err(ProjectError::ClipOutOfBounds);
        }
        let src_len = self.source(c.source_id)?.length_frames as i128;
        let shift = self.tempo.tick_to_frame(start, self.sample_rate()) as i128
            - self.clip_start_frame(c) as i128;
        let len = length_frames as i128;
        let (off, old_len) = (c.offset_frames as i128, c.length_frames as i128);
        let new_off = if c.edit.reversed {
            off + old_len - (shift + len)
        } else {
            off + shift
        };
        if new_off < 0 || new_off + len > src_len {
            return Err(ProjectError::ClipOutOfBounds);
        }
        Ok(Clip {
            start,
            offset_frames: new_off as u64,
            length_frames,
            edit: c.edit.clamped_to(length_frames),
            ..c.clone()
        })
    }

    /// Divide o clipe no tick `at` (estritamente dentro dele): a primeira parte mantém o id,
    /// a segunda recebe `new_id`. O áudio resultante é idêntico ao do clipe original quando não
    /// há fades: a segunda parte começa exatamente na amostra de `at`.
    /// O fade de entrada fica na primeira parte; o de saída, na segunda.
    pub fn split_parts(
        &self,
        id: ClipId,
        at: Tick,
        new_id: ClipId,
    ) -> Result<(Clip, Clip), ProjectError> {
        let c = self.clip(id)?;
        if at.0 < 0 {
            return Err(ProjectError::InvalidSplit);
        }
        let split = self.tempo.tick_to_frame(at, self.sample_rate()) as i128
            - self.clip_start_frame(c) as i128;
        if split <= 0 || split >= c.length_frames as i128 {
            return Err(ProjectError::InvalidSplit);
        }
        let f = split as u64;
        let rest = c.length_frames - f;
        let (first_off, second_off) = if c.edit.reversed {
            (c.offset_frames + rest, c.offset_frames)
        } else {
            (c.offset_frames, c.offset_frames + f)
        };
        let first = Clip {
            offset_frames: first_off,
            length_frames: f,
            edit: ClipEdit {
                fade_in_frames: c.edit.fade_in_frames.min(f),
                fade_out_frames: 0,
                ..c.edit
            },
            ..c.clone()
        };
        let second = Clip {
            id: new_id,
            start: at,
            offset_frames: second_off,
            length_frames: rest,
            edit: ClipEdit {
                fade_in_frames: 0,
                fade_out_frames: c.edit.fade_out_frames.min(rest),
                ..c.edit
            },
            ..c.clone()
        };
        Ok((first, second))
    }

    /// Crossfade entre dois clipes da mesma faixa que se sobrepõem parcialmente: o que começa
    /// antes recebe fade de saída e o outro, fade de entrada, ambos com a duração da sobreposição.
    /// Retorna `(anterior, posterior)` já ajustados.
    pub fn crossfade_parts(
        &self,
        a: ClipId,
        b: ClipId,
        shape: FadeShape,
    ) -> Result<(Clip, Clip), ProjectError> {
        let (ta, _) = self.find_clip(a)?;
        let (tb, _) = self.find_clip(b)?;
        if ta != tb || a == b {
            return Err(ProjectError::ClipsDoNotOverlap);
        }
        let (mut x, mut y) = (self.clip(a)?.clone(), self.clip(b)?.clone());
        if self.clip_start_frame(&y) < self.clip_start_frame(&x) {
            std::mem::swap(&mut x, &mut y);
        }
        let (xs, xe) = (self.clip_start_frame(&x), self.clip_end_frame(&x));
        let (ys, ye) = (self.clip_start_frame(&y), self.clip_end_frame(&y));
        if !(xs < ys && ys < xe && ye >= xe) {
            return Err(ProjectError::ClipsDoNotOverlap);
        }
        let overlap = xe - ys;
        x.edit.fade_out_frames = overlap;
        x.edit.fade_out_shape = shape;
        x.edit.fade_in_frames = x.edit.fade_in_frames.min(x.length_frames - overlap);
        y.edit.fade_in_frames = overlap;
        y.edit.fade_in_shape = shape;
        y.edit.fade_out_frames = y.edit.fade_out_frames.min(y.length_frames - overlap);
        Ok((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;
    use shared_kernel::Db;

    // 120 bpm @ 48 kHz: 1 tick = 25 amostras.

    #[test]
    fn edit_invariant_fades_fit_length() {
        let e = ClipEdit {
            fade_in_frames: 60,
            fade_out_frames: 40,
            ..ClipEdit::default()
        };
        assert!(e.validate(100).is_ok());
        assert_eq!(e.validate(99), Err(ProjectError::InvalidFade));
        let huge = ClipEdit {
            fade_in_frames: u64::MAX,
            fade_out_frames: 1,
            ..e
        };
        assert_eq!(huge.validate(u64::MAX), Err(ProjectError::InvalidFade));
        let c = e.clamped_to(70);
        assert_eq!((c.fade_in_frames, c.fade_out_frames), (60, 10));
        assert!(c.validate(70).is_ok());
    }

    #[test]
    fn insert_and_replace_reject_invalid_fades() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 0, 100);
        c.edit.fade_in_frames = 101;
        assert_eq!(
            p.insert_clip(t, c.clone(), 0),
            Err(ProjectError::InvalidFade)
        );
        c.edit.fade_in_frames = 50;
        p.insert_clip(t, c.clone(), 0).unwrap();
        let mut bad = c.clone();
        bad.edit.fade_out_frames = 51;
        assert_eq!(p.replace_clip(bad), Err(ProjectError::InvalidFade));
        assert_eq!(p.clip(c.id).unwrap(), &c, "falha não altera estado");
    }

    #[test]
    fn trim_right_edge_changes_only_length() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 960, 1000);
        c.offset_frames = 100;
        c.edit.fade_out_frames = 300;
        let id = c.id;
        p.insert_clip(t, c, 0).unwrap();
        let r = p.trimmed_clip(id, Tick(960), 500).unwrap();
        assert_eq!(
            (r.start, r.offset_frames, r.length_frames),
            (Tick(960), 100, 500)
        );
        assert_eq!(r.edit.fade_out_frames, 300);
        let r = p.trimmed_clip(id, Tick(960), 200).unwrap();
        assert_eq!(r.edit.fade_out_frames, 200, "fade limitado à nova duração");
        assert_eq!(
            p.trimmed_clip(id, Tick(960), 48_000).unwrap_err(),
            ProjectError::ClipOutOfBounds,
            "não passa do fim da fonte"
        );
    }

    #[test]
    fn trim_left_edge_moves_start_and_offset() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 960, 1000);
        c.offset_frames = 100;
        let id = c.id;
        p.insert_clip(t, c, 0).unwrap();
        // borda esquerda 2 ticks para a direita = 50 amostras
        let r = p.trimmed_clip(id, Tick(962), 950).unwrap();
        assert_eq!((r.offset_frames, r.length_frames), (150, 950));
        // para a esquerda: revela 4 ticks = 100 amostras anteriores da fonte
        let r = p.trimmed_clip(id, Tick(956), 1100).unwrap();
        assert_eq!(r.offset_frames, 0);
        assert_eq!(
            p.trimmed_clip(id, Tick(955), 1125).unwrap_err(),
            ProjectError::ClipOutOfBounds,
            "não revela antes do início da fonte"
        );
        assert_eq!(
            p.trimmed_clip(id, Tick(-1), 10).unwrap_err(),
            ProjectError::InvalidPosition
        );
        assert_eq!(
            p.trimmed_clip(id, Tick(960), 0).unwrap_err(),
            ProjectError::ClipOutOfBounds
        );
    }

    #[test]
    fn trim_reversed_clip_maps_left_edge_to_source_end() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 0, 1000);
        c.offset_frames = 100;
        c.edit.reversed = true;
        let id = c.id;
        p.insert_clip(t, c, 0).unwrap();
        // corta 50 amostras da esquerda: remove o FIM da região na fonte
        let r = p.trimmed_clip(id, Tick(2), 950).unwrap();
        assert_eq!((r.offset_frames, r.length_frames), (100, 950));
        // corta 50 da direita: remove o INÍCIO da região na fonte
        let r = p.trimmed_clip(id, Tick(0), 950).unwrap();
        assert_eq!((r.offset_frames, r.length_frames), (150, 950));
    }

    #[test]
    fn split_produces_contiguous_parts() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 960, 1000);
        c.offset_frames = 10;
        c.gain = Db(-3.0);
        c.edit.fade_in_frames = 100;
        c.edit.fade_out_frames = 200;
        c.edit.polarity_inverted = true;
        let id = c.id;
        p.insert_clip(t, c, 0).unwrap();
        let nid = ClipId::new();
        let (a, b) = p.split_parts(id, Tick(960 + 16), nid).unwrap(); // 400 amostras
        assert_eq!(
            (a.id, a.start, a.offset_frames, a.length_frames),
            (id, Tick(960), 10, 400)
        );
        assert_eq!(
            (b.id, b.start, b.offset_frames, b.length_frames),
            (nid, Tick(976), 410, 600)
        );
        assert_eq!((a.edit.fade_in_frames, a.edit.fade_out_frames), (100, 0));
        assert_eq!((b.edit.fade_in_frames, b.edit.fade_out_frames), (0, 200));
        assert!(a.edit.polarity_inverted && b.edit.polarity_inverted);
        assert_eq!((a.gain, b.gain), (Db(-3.0), Db(-3.0)));
        assert_eq!(p.clip_end_frame(&a), p.clip_start_frame(&b), "contíguos");
        for bad in [Tick(960), Tick(1000), Tick(0), Tick(-5), Tick(i64::MAX)] {
            assert_eq!(
                p.split_parts(id, bad, nid).unwrap_err(),
                ProjectError::InvalidSplit,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn split_reversed_clip_keeps_reversed_order() {
        let (mut p, t, s) = project_with_track();
        let mut c = clip(&s, 0, 1000);
        c.edit.reversed = true;
        let id = c.id;
        p.insert_clip(t, c, 0).unwrap();
        let (a, b) = p.split_parts(id, Tick(4), ClipId::new()).unwrap(); // 100 amostras
                                                                         // primeira parte toca o fim da fonte (900..1000) ao contrário
        assert_eq!((a.offset_frames, a.length_frames), (900, 100));
        assert_eq!((b.offset_frames, b.length_frames), (0, 900));
    }

    #[test]
    fn crossfade_sets_fades_over_overlap() {
        let (mut p, t, s) = project_with_track();
        let a = clip(&s, 0, 1000);
        let b = clip(&s, 32, 1000); // começa em 800 amostras: sobreposição 200
        let (aid, bid) = (a.id, b.id);
        p.insert_clip(t, a, 0).unwrap();
        p.insert_clip(t, b, 1).unwrap();
        // a ordem dos argumentos não importa
        let (x, y) = p.crossfade_parts(bid, aid, FadeShape::EqualPower).unwrap();
        assert_eq!((x.id, y.id), (aid, bid));
        assert_eq!(x.edit.fade_out_frames, 200);
        assert_eq!(y.edit.fade_in_frames, 200);
        assert_eq!(x.edit.fade_out_shape, FadeShape::EqualPower);
        assert_eq!(y.edit.fade_in_shape, FadeShape::EqualPower);
    }

    #[test]
    fn crossfade_rejects_non_overlapping_or_other_track() {
        let (mut p, t, s) = project_with_track();
        let a = clip(&s, 0, 100);
        let b = clip(&s, 960, 100);
        let inner = clip(&s, 1, 10); // totalmente dentro de `a`
        let (aid, bid, iid) = (a.id, b.id, inner.id);
        p.insert_clip(t, a, 0).unwrap();
        p.insert_clip(t, b, 1).unwrap();
        p.insert_clip(t, inner, 2).unwrap();
        for (x, y) in [(aid, bid), (aid, aid), (aid, iid)] {
            assert_eq!(
                p.crossfade_parts(x, y, FadeShape::Linear).unwrap_err(),
                ProjectError::ClipsDoNotOverlap
            );
        }
        let t2 = super::super::Track::new_audio("B").unwrap();
        let t2id = t2.id;
        p.insert_track(t2, 1).unwrap();
        let c = clip(&s, 1, 100);
        let cid = c.id;
        p.insert_clip(t2id, c, 0).unwrap();
        assert_eq!(
            p.crossfade_parts(aid, cid, FadeShape::Linear).unwrap_err(),
            ProjectError::ClipsDoNotOverlap
        );
    }

    #[test]
    fn serde_is_flat_and_backward_compatible() {
        let (_, _, s) = project_with_track();
        let mut c = clip(&s, 0, 10);
        c.edit.reversed = true;
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["reversed"], true);
        assert_eq!(v["fadeInShape"], "linear");
        // JSON antigo (sem os campos da Fase 3) continua válido
        let mut old = v.clone();
        for k in [
            "fadeInFrames",
            "fadeOutFrames",
            "fadeInShape",
            "fadeOutShape",
            "polarityInverted",
            "reversed",
        ] {
            old.as_object_mut().unwrap().remove(k);
        }
        let back: Clip = serde_json::from_value(old).unwrap();
        assert_eq!(back.edit, ClipEdit::default());
    }
}
